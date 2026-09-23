//! Reading and changing git state. Everything here is local except [`fetch`]
//! and [`populate`], the network calls constitution §3 allows: on either side
//! of cutting a branch, and never fatal.

use crate::config::Config;
use crate::group;
use crate::host::{Host, Output};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// The branch checked out in `path`, empty when it is not a repository or its
/// HEAD is detached — git answers `HEAD` for that, which names nothing.
pub fn branch(host: &dyn Host, path: &Path) -> Result<String> {
    let out = host.run(&["git", "rev-parse", "--abbrev-ref", "HEAD"], Some(path))?;
    Ok(match out.succeeded() && out.trimmed() != "HEAD" {
        true => out.trimmed().to_string(),
        false => String::new(),
    })
}

/// The branch new work is cut from: the configured override, else what
/// `origin/HEAD` points at, else whichever of `main` and `master` exists.
pub fn default_branch(host: &dyn Host, config: &Config, path: &Path) -> Result<String> {
    if let Some(configured) = &config.default_branch {
        return Ok(configured.clone());
    }

    let head = host.run(
        &["git", "symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
        Some(path),
    )?;
    if head.succeeded() {
        // `origin/main` — the remote name is not part of the branch name.
        if let Some((_, branch)) = head.trimmed().rsplit_once('/') {
            return Ok(branch.to_string());
        }
    }

    let main = host.run(
        &["git", "rev-parse", "--verify", "--quiet", "refs/heads/main"],
        Some(path),
    )?;
    Ok(match main.succeeded() {
        true => "main",
        false => "master",
    }
    .to_string())
}

/// Whether the checkout has uncommitted changes, and so must not be branched
/// in place. A path git cannot report on counts as clean: there is nothing to
/// lose there.
pub fn is_dirty(host: &dyn Host, path: &Path) -> Result<bool> {
    let out = host.run(&["git", "status", "--porcelain"], Some(path))?;
    Ok(out.succeeded() && !out.trimmed().is_empty())
}

pub fn fetch(host: &dyn Host, path: &Path) -> Result<Output> {
    host.run(&["git", "fetch", "origin"], Some(path))
}

/// Cut `name` from `base` in the existing checkout and switch to it.
///
/// `--no-track`: cutting from `origin/<default>` would otherwise set that as
/// the new branch's upstream, pointing its push and pull at the default
/// branch. A branch cut here tracks nothing until its first push names the
/// remote branch after itself.
pub fn create_branch(host: &dyn Host, path: &Path, name: &str, base: &str) -> Result<()> {
    let out = host.run(
        &["git", "checkout", "--no-track", "-b", name, base],
        Some(path),
    )?;
    match out.succeeded() {
        true => Ok(()),
        false => Err(anyhow!(
            "git would not create branch {name}: {}",
            out.stderr.trim()
        )),
    }
}

/// Cut `name` from `base` into a new worktree at `path`, in one operation —
/// `git worktree add` creates the branch and the directory together. Tracks
/// nothing, for the reason [`create_branch`] gives.
pub fn add_worktree(
    host: &dyn Host,
    main: &Path,
    path: &Path,
    name: &str,
    base: &str,
) -> Result<()> {
    let out = host.run(
        &[
            "git",
            "worktree",
            "add",
            "--no-track",
            "-b",
            name,
            &path.to_string_lossy(),
            base,
        ],
        Some(main),
    )?;
    match out.succeeded() {
        true => Ok(()),
        false => Err(anyhow!(
            "git would not add worktree {}: {}",
            path.display(),
            out.stderr.trim()
        )),
    }
}

/// Remove the worktree at `path`, never forced: git itself refuses one with
/// changes or untracked files. Its branch stays, and with it every commit.
pub fn remove_worktree(host: &dyn Host, main: &Path, path: &Path) -> Result<()> {
    let out = host.run(
        &["git", "worktree", "remove", &path.to_string_lossy()],
        Some(main),
    )?;
    match out.succeeded() {
        true => Ok(()),
        false => Err(anyhow!(
            "git would not remove worktree {}: {}",
            path.display(),
            out.stderr.trim()
        )),
    }
}

/// Forget the worktrees whose directories are gone. There is nothing on disk
/// to lose, and the picker never listed them.
pub fn prune_worktrees(host: &dyn Host, main: &Path) -> Result<()> {
    host.run(&["git", "worktree", "prune"], Some(main))?;
    Ok(())
}

/// The worktrees of `main` whose work is done: merging their HEAD into the
/// default branch would change nothing — merged, squashed or rebased in as it
/// stands — or their branch's remote was deleted. Nothing is fetched, so this
/// is as current as the last fetch.
///
/// A branch cut and not yet committed to is merged by this measure, which is
/// why each one is still asked about before it goes.
pub fn outdated(host: &dyn Host, config: &Config, main: &Path) -> Result<Vec<PathBuf>> {
    let default = default_branch(host, config, main)?;
    // The remote ref where there is one: branches are cut from it, and
    // nothing here pulls the local default branch up to it.
    let remote = format!("origin/{default}");
    let (base, tree) = match tree(host, main, &remote)? {
        Some(tree) => (remote, Some(tree)),
        None => (default.clone(), tree(host, main, &default)?),
    };

    let refs = host.run(
        &[
            "git",
            "for-each-ref",
            "--format=%(refname:short)\t%(upstream:track)",
            "refs/heads",
        ],
        Some(main),
    )?;
    let gone: Vec<&str> = refs
        .stdout
        .lines()
        .filter_map(|line| line.strip_suffix("\t[gone]"))
        .collect();

    let mut done = Vec::new();
    // Asked of git rather than the cache, which is as old as the last refresh.
    for worktree in group::worktrees(host, main)? {
        let merged = match &tree {
            // git before 2.38 has no --write-tree, fails, and so merges nothing.
            Some(tree) => {
                let out = host.run(
                    &["git", "merge-tree", "--write-tree", &base, "HEAD"],
                    Some(&worktree.path),
                )?;
                out.succeeded() && out.trimmed() == tree
            }
            None => false,
        };
        if merged || gone.contains(&worktree.branch.as_str()) {
            done.push(worktree.path);
        }
    }
    Ok(done)
}

/// The tree `rev` points at, `None` when there is no such ref.
fn tree(host: &dyn Host, main: &Path, rev: &str) -> Result<Option<String>> {
    let out = host.run(
        &[
            "git",
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{rev}^{{tree}}"),
        ],
        Some(main),
    )?;
    Ok(out.succeeded().then(|| out.trimmed().to_string()))
}

/// Bring a checkout a branch was just cut into up to what the branch records.
/// Neither `git worktree add` nor `git checkout` touches submodules, so a new
/// worktree starts with them empty; `--init` because a submodule's own
/// submodules start uninitialised in every fresh clone. In a repository that
/// uses LFS the files are pulled too: a checkout that skipped the smudge
/// filter left pointers where they belong, and one that ran it has nothing
/// left to fetch.
///
/// Both can reach the network, so neither is fatal: what failed is returned
/// for a warning, and the work goes on without it.
pub fn populate(host: &dyn Host, path: &Path) -> Result<Vec<String>> {
    // ponytail: the root .gitattributes only; ask `git lfs ls-files` if a
    // repository turns up that tracks LFS from a nested one.
    let lfs = host
        .read_file(&path.join(".gitattributes"))?
        .is_some_and(|attributes| attributes.contains("filter=lfs"));
    let mut steps: Vec<&[&str]> = vec![&["git", "submodule", "update", "--init", "--recursive"]];
    if lfs {
        steps.push(&["git", "lfs", "pull"]);
    }

    let mut failed = Vec::new();
    for argv in steps {
        let out = host.run(argv, Some(path))?;
        if !out.succeeded() {
            failed.push(format!("{} failed ({})", argv.join(" "), out.stderr.trim()));
        }
    }
    Ok(failed)
}
