//! How often and how recently each project was opened, and the ranking that
//! falls out of it. `~/.vibestation/state.json`, one record per project.

use crate::host::Host;
use crate::scan::Project;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// One project's history. Worktrees credit their project, so there is no
/// record per worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Open {
    pub path: PathBuf,
    pub count: u32,
    /// Seconds since the Unix epoch.
    pub last: u64,
}

const HOUR: u64 = 60 * 60;
const DAY: u64 = 24 * HOUR;
const WEEK: u64 = 7 * DAY;

pub fn path(host: &dyn Host) -> Result<PathBuf> {
    Ok(host.home()?.join(".vibestation/state.json"))
}

/// The recorded opens. A missing or unreadable file is simply no history: the
/// ranking degrades to the order projects were found in, never to an error.
pub fn load(host: &dyn Host) -> Result<Vec<Open>> {
    let Some(text) = host.read_file(&path(host)?)? else {
        return Ok(Vec::new());
    };
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

/// Count one open of `project`, now.
pub fn record(host: &dyn Host, project: &Path) -> Result<()> {
    let mut opens = load(host)?;
    let now = seconds(host.now());
    match opens.iter_mut().find(|open| open.path == project) {
        Some(open) => {
            open.count += 1;
            open.last = now;
        }
        None => opens.push(Open {
            path: project.to_path_buf(),
            count: 1,
            last: now,
        }),
    }

    let json = serde_json::to_string_pretty(&opens).context("encoding the frecency state")?;
    host.write_file(&path(host)?, &format!("{json}\n"))
}

/// Most-used-lately first. Ties — including everything never opened — keep the
/// order they were discovered in, since the sort is stable.
pub fn rank(projects: Vec<Project>, opens: &[Open], now: SystemTime) -> Vec<Project> {
    let now = seconds(now);
    let mut ranked = projects;
    ranked.sort_by(|a, b| {
        let score = |project: &Project| {
            opens
                .iter()
                .find(|open| open.path == project.path)
                .map_or(0.0, |open| score(open, now))
        };
        score(b).total_cmp(&score(a))
    });
    ranked
}

/// zoxide's frecency: the open count, multiplied up while the last open is
/// fresh and divided down as it ages.
fn score(open: &Open, now: u64) -> f64 {
    let count = f64::from(open.count);
    match now.saturating_sub(open.last) {
        age if age < HOUR => count * 4.0,
        age if age < DAY => count * 2.0,
        age if age < WEEK => count * 0.5,
        _ => count * 0.25,
    }
}

/// Shared with [`crate::tmux`], which ages sessions off the same clock.
pub fn seconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}
