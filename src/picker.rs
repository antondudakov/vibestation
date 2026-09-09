//! The rows of the one picker. For now it holds live sessions; projects, their
//! worktrees and the action entries join them in later tickets.

use crate::tmux::Session;
use std::path::Path;

/// One row per session, in the order given: name, directory, branch, command,
/// and the attached marker. The name is padded so the columns line up, and the
/// marker is a word rather than a symbol so that typing it filters to those
/// sessions.
pub fn rows(sessions: &[Session], home: &Path) -> Vec<String> {
    let width = sessions
        .iter()
        .map(|s| s.name.chars().count())
        .max()
        .unwrap_or(0);
    sessions
        .iter()
        .map(|session| {
            let mut row = format!(
                "{:width$}  {}",
                session.name,
                abbreviate(&session.path.to_string_lossy(), home)
            );
            for field in [&session.branch, &session.command] {
                if !field.is_empty() {
                    row.push_str("  ");
                    row.push_str(field);
                }
            }
            if session.attached {
                row.push_str("  (attached)");
            }
            row
        })
        .collect()
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
