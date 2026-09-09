//! One project in several states. Worktrees are created as sibling
//! directories, so the scanner meets them as ordinary repositories and the
//! relationship has to come from git rather than from the directory names.

use crate::host::Host;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A linked worktree of a project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Worktree {
    pub path: PathBuf,
    /// Empty when the worktree's HEAD is detached.
    pub branch: String,
}

/// A main checkout and every worktree git knows of it, whether or not the scan
/// reached them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub main: PathBuf,
    pub worktrees: Vec<Worktree>,
}

/// Collapse discovered repositories into projects. Repositories sharing a git
/// common directory are one project; the main checkout is the one whose git
/// directory *is* that common directory. A worktree whose main checkout was
/// never scanned stands on its own rather than disappearing.
pub fn group(host: &dyn Host, repos: &[PathBuf]) -> Result<Vec<Group>> {
    let mut resolved = Vec::new();
    for repo in repos {
        let (git_dir, common) = dirs(host, repo)?;
        let is_main = git_dir == common;
        resolved.push((repo, common, is_main));
    }

    let mut groups = Vec::new();
    for (repo, common, is_main) in &resolved {
        if *is_main {
            groups.push(Group {
                main: (*repo).clone(),
                worktrees: worktrees(host, repo)?,
            });
        } else if !resolved.iter().any(|(_, c, main)| *main && c == common) {
            groups.push(Group {
                main: (*repo).clone(),
                worktrees: Vec::new(),
            });
        }
    }
    Ok(groups)
}

/// The repository's git directory and its common directory, equal exactly when
/// this is the main checkout. Git answers with an absolute path from a linked
/// worktree and a relative one from a main checkout, so relative answers are
/// anchored before they are compared. A directory git refuses to answer for
/// reads as its own main checkout with no worktrees, which is how a manually
/// added path that is no longer a repository still lists.
fn dirs(host: &dyn Host, repo: &Path) -> Result<(PathBuf, PathBuf)> {
    let out = host.run(
        &["git", "rev-parse", "--git-dir", "--git-common-dir"],
        Some(repo),
    )?;
    let mut lines = out.trimmed().lines();
    let git_dir = anchor(repo, lines.next().unwrap_or_default());
    let common = anchor(repo, lines.next().unwrap_or_default());
    Ok((git_dir, common))
}

fn anchor(repo: &Path, dir: &str) -> PathBuf {
    match Path::new(dir).is_absolute() {
        true => PathBuf::from(dir),
        false => repo.join(dir),
    }
}

/// `git worktree list` is authoritative: it knows worktrees the scan never
/// reached, deliberately a superset of what was found on disk. The main
/// checkout leads that list and is not a worktree of itself, and an entry
/// whose directory is gone is dropped rather than shown as a broken row.
fn worktrees(host: &dyn Host, main: &Path) -> Result<Vec<Worktree>> {
    let out = host.run(&["git", "worktree", "list", "--porcelain"], Some(main))?;

    let mut listed: Vec<Worktree> = Vec::new();
    for line in out.stdout.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            listed.push(Worktree {
                path: PathBuf::from(path),
                branch: String::new(),
            });
        } else if let Some(branch) = line.strip_prefix("branch refs/heads/") {
            if let Some(entry) = listed.last_mut() {
                entry.branch = branch.to_string();
            }
        }
    }
    Ok(listed
        .into_iter()
        .filter(|worktree| worktree.path != main && host.exists(&worktree.path))
        .collect())
}
