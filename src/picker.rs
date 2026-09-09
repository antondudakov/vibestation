//! The one picker: live sessions, a separator, then projects ranked by
//! frecency with their worktrees as indented children. Every row is one line
//! of text, so tmux sessions, projects and worktrees are all reachable by the
//! same typing.

use crate::scan::Project;
use crate::tmux::Session;
use std::path::Path;

/// What a row stands for, by index into the lists it was built from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Session(usize),
    Project(usize),
    /// A project and one of its worktrees.
    Worktree(usize, usize),
    /// The line between the sessions and everything else. Selectable, because
    /// no prompt library can make a row inert, and harmless when selected.
    Separator,
}

/// The rows, in picker order. A project or worktree that already has a live
/// session is left out: its session row above is the same work.
pub fn rows(sessions: &[Session], projects: &[Project], home: &Path) -> Vec<(Row, String)> {
    let live: Vec<&Path> = sessions.iter().map(|s| s.path.as_path()).collect();

    let session_width = width(sessions.iter().map(|s| s.name.as_str()));
    let mut rows: Vec<(Row, String)> = sessions
        .iter()
        .enumerate()
        .map(|(index, session)| {
            let row = columns(&[
                &pad(&session.name, session_width),
                &abbreviate(&session.path.to_string_lossy(), home),
                &session.branch,
                &session.command,
                if session.attached { "(attached)" } else { "" },
            ]);
            (Row::Session(index), row)
        })
        .collect();

    let project_width = width(
        projects
            .iter()
            .filter(|project| !live.contains(&project.path.as_path()))
            .map(|project| project.name.as_str()),
    );
    let mut below = Vec::new();
    for (index, project) in projects.iter().enumerate() {
        if !live.contains(&project.path.as_path()) {
            let row = columns(&[
                &pad(&project.name, project_width),
                &abbreviate(&project.path.to_string_lossy(), home),
            ]);
            below.push((Row::Project(index), row));
        }
        for (child, worktree) in project.worktrees.iter().enumerate() {
            if live.contains(&worktree.path.as_path()) {
                continue;
            }
            // Indented and marked, so the main checkout above reads as the
            // primary directory and this as one of its states.
            let row = columns(&[&base(&worktree.path), &worktree.branch]);
            below.push((Row::Worktree(index, child), format!("  └ {row}")));
        }
    }

    if !rows.is_empty() && !below.is_empty() {
        rows.push((Row::Separator, "─".repeat(24)));
    }
    rows.extend(below);
    rows
}

/// Fields two spaces apart, with the empty ones left out rather than showing
/// as a gap — a session outside a repository has no branch to print.
fn columns(fields: &[&str]) -> String {
    fields
        .iter()
        .filter(|field| !field.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("  ")
}

fn width<'a>(names: impl Iterator<Item = &'a str>) -> usize {
    names.map(|name| name.chars().count()).max().unwrap_or(0)
}

fn pad(name: &str, width: usize) -> String {
    format!("{name:<width$}")
}

fn base(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// `/home/dev/code/api` as `~/code/api`, so the useful end of a long path is
/// what the eye lands on.
fn abbreviate(path: &str, home: &Path) -> String {
    match path.strip_prefix(&*home.to_string_lossy()) {
        Some("") => "~".to_string(),
        Some(rest) if rest.starts_with('/') => format!("~{rest}"),
        _ => path.to_string(),
    }
}
