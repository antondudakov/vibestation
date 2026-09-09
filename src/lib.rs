pub mod config;
pub mod fake;
pub mod host;
pub mod picker;
pub mod scan;
pub mod tmux;

use anyhow::Result;
use host::Host;

/// The single entry point tests drive: settle the config, then open the picker
/// over what tmux has and attach to the choice. Projects join the list in the
/// tickets that follow.
pub fn run(host: &dyn Host) -> Result<()> {
    let _config = config::load_or_init(host)?;

    let sessions = tmux::list(host)?;
    if sessions.is_empty() {
        // A picker over nothing is a prompt with no answer. Projects and the
        // action rows arrive in tickets 05 and 11 and this case goes away.
        println!("no tmux sessions");
        return Ok(());
    }

    let rows = picker::rows(&sessions, &host.home()?);
    let chosen = &sessions[host.select("Session", &rows)?];
    tmux::attach(host, &chosen.name)
}
