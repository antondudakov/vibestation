//! Finding the developer's repositories: walk the configured roots, merge in
//! the manually added ones, and remember the result in a JSON cache so that
//! opening the picker never waits on the disk. Rescanning is explicit: the
//! picker's ←, never a timer.

use crate::config::Config;
use crate::group::{self, Group, Worktree};
use crate::host::Host;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A project: one main checkout and the worktrees that are the same repository
/// in another state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    /// The directory name, prefixed with its parent when that alone collides.
    pub name: String,
    pub path: PathBuf,
    /// Absent from a cache written before worktrees were grouped.
    #[serde(default)]
    pub worktrees: Vec<Worktree>,
}

impl Project {
    /// The main checkout, or one of its worktrees.
    pub fn dir(&self, worktree: Option<usize>) -> &Path {
        match worktree {
            Some(child) => &self.worktrees[child].path,
            None => &self.path,
        }
    }
}

pub fn cache_path(host: &dyn Host) -> Result<PathBuf> {
    Ok(host.home()?.join(".vibestation/projects-cache.json"))
}

/// The cached projects, scanning and writing the cache when there is none. An
/// unreadable cache is rescanned rather than reported: it is derived data, and
/// the disk is the truth it was derived from.
pub fn load_or_scan(host: &dyn Host, config: &Config) -> Result<Vec<Project>> {
    if let Some(cached) = host.read_file(&cache_path(host)?)? {
        if let Ok(projects) = serde_json::from_str(&cached) {
            return Ok(projects);
        }
    }
    rescan(host, config)
}

/// Scan the roots again and rewrite the cache: the picker's ←, and the only
/// thing that ever invalidates the cache.
pub fn rescan(host: &dyn Host, config: &Config) -> Result<Vec<Project>> {
    let projects = scan(host, config)?;
    let json = serde_json::to_string_pretty(&projects).context("encoding the projects cache")?;
    host.write_file(&cache_path(host)?, &format!("{json}\n"))?;
    Ok(projects)
}

/// Every repository under the configured roots, plus the manually added ones,
/// collapsed into projects with their worktrees. Reads the filesystem and asks
/// git about each repository; never the network. Says how far it has got on
/// the status line, and clears it when done.
pub fn scan(host: &dyn Host, config: &Config) -> Result<Vec<Project>> {
    let mut found: Vec<PathBuf> = Vec::new();
    for root in &config.projects_dirs {
        // One call with no count until it returns, so a name rather than a bar.
        host.status(&format!("scanning {}…", root.display()));
        // ponytail: the walk returns every directory and the pruning happens
        // here, so a `node_modules` under a repository is still traversed
        // before being discarded. Push a prune predicate into `Host::walk` if
        // a refresh ever feels slow.
        //
        // Sorted so a parent is always seen before its children, which is what
        // makes "the topmost repository wins" the same rule as "stop
        // descending at the first `.git`" — submodules and vendored copies
        // below a repository never make the list.
        let mut dirs = host.walk(root, config.scan_depth)?;
        dirs.sort();
        for dir in dirs {
            let nested = found.iter().any(|repo| dir.starts_with(repo));
            if !nested && host.exists(&dir.join(".git")) {
                found.push(dir);
            }
        }
    }
    for extra in &config.extra_projects {
        if !found.contains(extra) {
            found.push(extra.clone());
        }
    }
    let groups = group::group(host, &found)?;
    host.status("");
    Ok(name(groups))
}

/// Directory names, disambiguated by their parent where two roots hold the
/// same name — `work/api` and `personal/api`, but a lone `vibestation` stays
/// bare.
///
/// ponytail: one level of parent is enough for every layout seen so far; if
/// the parents collide too, the loser is still identifiable by its path in the
/// picker row.
fn name(groups: Vec<Group>) -> Vec<Project> {
    let base = |path: &Path| {
        path.file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .into_owned()
    };
    groups
        .iter()
        .map(|group| {
            let bare = base(&group.main);
            let collides = groups
                .iter()
                .any(|other| other.main != group.main && base(&other.main) == bare);
            let name = match (collides, group.main.parent()) {
                (true, Some(parent)) => format!("{}/{bare}", base(parent)),
                _ => bare,
            };
            Project {
                name,
                path: group.main.clone(),
                worktrees: group.worktrees.clone(),
            }
        })
        .collect()
}
