//! Reading live tmux sessions: one `list-sessions` call for everything tmux
//! knows, one git call per session for the branch it doesn't, and — only
//! inside tmux — one more for which session the client is in.

use crate::git;
use crate::host::Host;
use crate::state::seconds;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;

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
    /// This is the session the client running vibestation is sitting in —
    /// which `attached` alone cannot say, since that is equally true of a
    /// session left open on another machine.
    pub current: bool,
    /// Empty when the path is not a git repository.
    pub branch: String,
    /// How long since a client last had it — `now`, `5m`, `3h`, `12d` — and
    /// empty for a session no client has ever attached. Rendered here because
    /// this is the module holding the clock tmux's answer is measured against.
    pub age: String,
}

/// Only the active pane is inspected: in a session's format context, the pane
/// variables already resolve to the active pane of the active window, so no
/// window or pane enumeration is needed.
///
/// Tab-separated, with the session name last so that a name containing a tab
/// still parses — the fields before it are split off and the rest is the name.
const FORMAT: &str = "#{session_attached}\t#{session_last_attached}\t#{pane_current_path}\t#{pane_current_command}\t#{session_name}";

/// The live sessions, most recently left first — the one you want is usually
/// the one you were just in. A session no client has ever attached sorts last,
/// and ties keep tmux's own order, since the sort is stable. No tmux server is
/// a normal cold start and yields an empty list.
pub fn list(host: &dyn Host) -> Result<Vec<Session>> {
    let out = host.run(&["tmux", "list-sessions", "-F", FORMAT], None)?;
    if !out.succeeded() {
        return Ok(Vec::new());
    }
    let here = current(host)?;
    let now = seconds(host.now());

    let mut sessions = Vec::new();
    for line in out.stdout.lines() {
        if let Some((last, mut session)) = parse(line) {
            session.branch = git::branch(host, &session.path)?;
            session.current = session.name == here;
            session.age = age(last, now);
            sessions.push((last, session));
        }
    }
    sessions.sort_by(|(a, _), (b, _)| b.cmp(a));
    Ok(sessions.into_iter().map(|(_, session)| session).collect())
}

/// The session this client is sitting in. Outside tmux there is no such
/// session, and tmux is not asked.
fn current(host: &dyn Host) -> Result<String> {
    if !host.in_tmux() {
        return Ok(String::new());
    }
    let out = host.run(&["tmux", "display-message", "-p", "#{session_name}"], None)?;
    Ok(out.trimmed().to_string())
}

/// How long since a client last had the session. Coarse on purpose: the
/// question it answers is which of these you were in, not how many minutes ago.
fn age(last: u64, now: u64) -> String {
    if last == 0 {
        return String::new();
    }
    match now.saturating_sub(last) {
        since if since < MINUTE => "now".to_string(),
        since if since < HOUR => format!("{}m", since / MINUTE),
        since if since < DAY => format!("{}h", since / HOUR),
        since => format!("{}d", since / DAY),
    }
}

fn parse(line: &str) -> Option<(u64, Session)> {
    let mut fields = line.splitn(5, '\t');
    let attached = fields.next()?;
    let last = fields.next()?.trim().parse().unwrap_or(0);
    let path = fields.next()?;
    let command = fields.next()?;
    let name = fields.next().filter(|name| !name.is_empty())?;

    Some((
        last,
        Session {
            name: name.to_string(),
            path: PathBuf::from(path),
            command: command.to_string(),
            attached: attached != "0",
            current: false,
            branch: String::new(),
            age: String::new(),
        },
    ))
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
