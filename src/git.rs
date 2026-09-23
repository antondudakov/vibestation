//! Reading and changing git state. Everything here is local except [`fetch`],
//! the one network call constitution §3 allows: opt-in, immediately before a
//! branch is cut, and never fatal.

use crate::config::Config;
use crate::host::{Host, Output};
use anyhow::{anyhow, Result};
use std::path::Path;

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
