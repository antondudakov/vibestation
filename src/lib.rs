pub mod config;
pub mod fake;
pub mod git;
pub mod group;
pub mod host;
pub mod naming;
pub mod picker;
pub mod scan;
pub mod state;
pub mod tmux;

use anyhow::Result;
use config::Config;
use host::Host;
use picker::Row;
use scan::Project;
use std::path::Path;

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
        Row::Project(index) => open(host, &config, &projects[index], None),
        Row::Worktree(index, child) => open(host, &config, &projects[index], Some(child)),
    }
}

/// Start work: settle on a session name, offer to cut the branch it names,
/// create the session in the directory the work lives in, count the open
/// against the project, and join it.
fn open(
    host: &dyn Host,
    config: &Config,
    project: &Project,
    worktree: Option<usize>,
) -> Result<()> {
    let (dir, branch) = match worktree {
        Some(child) => {
            let worktree = &project.worktrees[child];
            (worktree.path.as_path(), worktree.branch.clone())
        }
        None => (
            project.path.as_path(),
            git::branch(host, &project.path)?.clone(),
        ),
    };

    // A worktree row, or a checkout already on a feature branch, is work in
    // progress: its branch names it, nothing is asked and no branch is cut.
    let default = match worktree.is_some() || branch.is_empty() {
        true => String::new(),
        false => git::default_branch(host, config, dir)?,
    };
    let started = !branch.is_empty() && branch != default;
    let name = match started {
        true => naming::from_branch(&config.username, &label(dir, &branch)),
        false => {
            let said = host.input("What are you working on?", "")?;
            let suggested = naming::suggest(&config.username, &said);
            naming::sanitize(&host.input("Session name", &suggested)?)
        }
    };

    if !started && !branch.is_empty() && name != branch {
        cut_branch(host, config, dir, &name, &default)?;
    }

    tmux::create(host, &name, dir)?;
    // Frecency is tracked against the project, so a worktree credits its parent.
    state::record(host, &project.path)?;
    tmux::attach(host, &name)
}

/// Offer the branch the session was just named for, cut in place in this
/// checkout — or not at all, which still gets the session on the current
/// branch. Ticket 10 adds the third option, a worktree.
///
/// The fetch is opt-in, pre-answered from config, and never fatal: a remote
/// that cannot be reached costs a warning and the local ref.
fn cut_branch(
    host: &dyn Host,
    config: &Config,
    dir: &Path,
    name: &str,
    default: &str,
) -> Result<()> {
    if git::is_dirty(host, dir)? {
        println!(
            "uncommitted changes in {}, staying on {default}",
            dir.display()
        );
        return Ok(());
    }
    if !host.confirm(&format!("Create branch {name}?"), true)? {
        return Ok(());
    }

    // Cutting from `origin/<default>` rather than fast-forwarding the local
    // ref: nothing existing is touched, so divergence cannot fail the cut.
    let mut base = default.to_string();
    if host.confirm("Fetch origin first?", config.fetch_before_branch)? {
        let out = git::fetch(host, dir)?;
        match out.succeeded() {
            true => base = format!("origin/{default}"),
            false => println!(
                "fetch failed ({}), branching from local {default}",
                out.stderr.trim()
            ),
        }
    }
    git::create_branch(host, dir, name, &base)
}

/// What to name work by when it has a branch — or, for a detached worktree and
/// for a directory git has nothing to say about, its own directory name.
fn label(dir: &Path, branch: &str) -> String {
    match branch.is_empty() {
        true => dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        false => branch.to_string(),
    }
}
