//! Reading live tmux sessions: one `list-sessions` call for everything tmux
//! knows, and one git call per session for the branch it doesn't.

use crate::host::Host;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// A live tmux session, as the picker will show it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub name: String,
    /// The active pane's working directory.
    pub path: PathBuf,
    /// The command running in the active pane.
    pub command: String,
    /// A client is attached, possibly in another terminal.
    pub attached: bool,
    /// Empty when the path is not a git repository.
    pub branch: String,
}

/// Only the active pane is inspected: in a session's format context, the pane
/// variables already resolve to the active pane of the active window, so no
/// window or pane enumeration is needed.
///
/// Tab-separated, with the session name last so that a name containing a tab
/// still parses — the fields before it are split off and the rest is the name.
const FORMAT: &str =
    "#{session_attached}\t#{pane_current_path}\t#{pane_current_command}\t#{session_name}";

/// The live sessions, newest tmux ordering preserved. No tmux server is a
/// normal cold start and yields an empty list.
pub fn list(host: &dyn Host) -> Result<Vec<Session>> {
    let out = host.run(&["tmux", "list-sessions", "-F", FORMAT], None)?;
    if !out.succeeded() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    for line in out.stdout.lines() {
        if let Some(mut session) = parse(line) {
            session.branch = branch(host, &session.path)?;
            sessions.push(session);
        }
    }
    Ok(sessions)
}

fn parse(line: &str) -> Option<Session> {
    let mut fields = line.splitn(4, '\t');
    let attached = fields.next()?;
    let path = fields.next()?;
    let command = fields.next()?;
    let name = fields.next().filter(|name| !name.is_empty())?;

    Some(Session {
        name: name.to_string(),
        path: PathBuf::from(path),
        command: command.to_string(),
        attached: attached != "0",
        branch: String::new(),
    })
}

/// The branch of a directory, empty when it is not a repository.
fn branch(host: &dyn Host, path: &Path) -> Result<String> {
    let out = host.run(&["git", "rev-parse", "--abbrev-ref", "HEAD"], Some(path))?;
    Ok(match out.succeeded() {
        true => out.trimmed().to_string(),
        false => String::new(),
    })
}

/// Join `name`: switching the current client when run inside tmux, since
/// attaching there would nest a client inside its own session, and attaching
/// otherwise. This replaces the process, so it is the last thing the tool does.
pub fn attach(host: &dyn Host, name: &str) -> Result<()> {
    let verb = match host.in_tmux() {
        true => "switch-client",
        false => "attach-session",
    };
    host.exec(&["tmux", verb, "-t", name])
}
