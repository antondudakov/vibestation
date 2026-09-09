pub mod config;
pub mod fake;
pub mod host;
pub mod tmux;

use anyhow::Result;
use host::Host;

/// The single entry point tests drive. The picker it will open arrives with the
/// tickets that follow; for now it settles the config and lists what tmux has.
pub fn run(host: &dyn Host) -> Result<()> {
    let _config = config::load_or_init(host)?;

    let sessions = tmux::list(host)?;
    if sessions.is_empty() {
        println!("no tmux sessions");
    }
    for session in &sessions {
        println!(
            "{}{}\t{}\t{}\t{}",
            session.name,
            if session.attached { " *" } else { "" },
            session.path.display(),
            session.branch,
            session.command,
        );
    }
    Ok(())
}
