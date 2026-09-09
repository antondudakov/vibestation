//! Reading live tmux sessions: one `list-sessions` call for everything tmux
//! knows, and one git call per session for the branch it doesn't.

use crate::git;
use crate::host::Host;
use anyhow::{anyhow, Result};
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
            session.branch = git::branch(host, &session.path)?;
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

/// A detached session rooted in `dir`, running the login shell: no startup
/// command, no window layout.
///
/// A session of that name already existing is the resume path rather than a
/// failure, so it is checked for first. `new-session -A` cannot serve here:
/// when the session exists it attaches, and `-d` does not stop it — which
/// outside a terminal is an error and inside tmux would nest a client.
pub fn create(host: &dyn Host, name: &str, dir: &Path) -> Result<()> {
    if host
        .run(&["tmux", "has-session", "-t", name], None)?
        .succeeded()
    {
        return Ok(());
    }

    let out = host.run(
        &[
            "tmux",
            "new-session",
            "-d",
            "-s",
            name,
            "-c",
            &dir.to_string_lossy(),
        ],
        None,
    )?;
    match out.succeeded() {
        true => Ok(()),
        false => Err(anyhow!(
            "tmux would not create session {name}: {}",
            out.stderr.trim()
        )),
    }
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
