pub mod config;
pub mod fake;
pub mod group;
pub mod host;
pub mod picker;
pub mod scan;
pub mod state;
pub mod tmux;

use anyhow::Result;
use host::Host;
use picker::Row;

/// The single entry point tests drive: settle the config, open one picker over
/// the live sessions and the ranked projects, and act on the choice.
pub fn run(host: &dyn Host) -> Result<()> {
    let config = config::load_or_init(host)?;

    let sessions = tmux::list(host)?;
    let projects = state::rank(
        scan::load_or_scan(host, &config)?,
        &state::load(host)?,
        host.now(),
    );

    let rows = picker::rows(&sessions, &projects, &host.home()?);
    if rows.is_empty() {
        // A picker over nothing is a prompt with no answer. Ticket 11's
        // add-manually row means the list is never empty again.
        println!("no tmux sessions and no projects; check projects_dirs in your config");
        return Ok(());
    }

    let labels: Vec<String> = rows.iter().map(|(_, label)| label.clone()).collect();
    match rows[host.select("Open", &labels)?].0 {
        Row::Session(index) => tmux::attach(host, &sessions[index].name),
        Row::Separator => Ok(()),
        // Ticket 08 turns a project or worktree into a named session. Until
        // then the directory is the useful answer: `cd "$(vibestation)"`.
        Row::Project(index) => {
            println!("{}", projects[index].path.display());
            Ok(())
        }
        Row::Worktree(index, child) => {
            println!("{}", projects[index].worktrees[child].path.display());
            Ok(())
        }
    }
}
