pub mod config;
pub mod fake;
pub mod git;
pub mod group;
pub mod host;
pub mod line;
pub mod naming;
pub mod picker;
pub mod scan;
pub mod state;
pub mod tmux;

use anyhow::{anyhow, Result};
use config::Config;
use host::Host;
use picker::Row;
use scan::Project;
use std::path::{Path, PathBuf};

/// The single entry point tests drive: settle the config, open one picker over
/// the live sessions and the ranked projects, and act on the choice.
///
/// The two escape hatches change the list rather than leaving it, so the
/// picker reopens over what they changed; every other choice is terminal.
pub fn run(host: &dyn Host) -> Result<()> {
    let mut config = config::load_or_init(host)?;
    let mut found = scan::load_or_scan(host, &config)?;

    // Read once: neither escape hatch touches tmux or the frecency records.
    let sessions = tmux::list(host)?;
    let opens = state::load(host)?;
    let home = host.home()?;

    loop {
        // Cloned because the separator reopens the picker over the same list.
        let projects = state::rank(found.clone(), &opens, host.now());
        let rows = picker::rows(
            &sessions,
            &projects,
            &home,
            &config.username,
            host.terminal().0,
        );
        let labels: Vec<String> = rows.iter().map(|(_, label)| label.clone()).collect();

        match rows[host.select(&picker::title(&rows), &labels)?].0 {
            Row::Session(index) => return tmux::attach(host, &sessions[index].name),
            // Decoration: choosing it costs nothing and reopens the list.
            Row::Separator => continue,
            Row::Project(index) => return open(host, &config, &sessions, &projects[index], None),
            Row::Worktree(index, child) => {
                return open(host, &config, &sessions, &projects[index], Some(child))
            }
            Row::Refresh => found = scan::rescan(host, &config)?,
            Row::AddManually => {
                add(host, &mut config)?;
                found = scan::rescan(host, &config)?;
            }
        }
    }
}

/// Take a repository by path and write it into the config, where it will be
/// picked up by this refresh and every one after it. A path that is not a
/// repository is refused with a message rather than an error: the picker is
/// still open behind it and the developer can simply try again.
fn add(host: &dyn Host, config: &mut Config) -> Result<()> {
    let typed = host.input("Path to the repository", "")?;
    let project = config::expand(&host.home()?, &typed);
    if !host.exists(&project.join(".git")) {
        println!("{} is not a git repository", project.display());
        return Ok(());
    }
    config::add_extra(host, config, project)
}

/// Start work: settle on a session name, offer to cut the branch it names —
/// in a new worktree or in place — create the session in the directory the
/// work now lives in, count the open against the project, and join it.
fn open(
    host: &dyn Host,
    config: &Config,
    sessions: &[tmux::Session],
    project: &Project,
    worktree: Option<usize>,
) -> Result<()> {
    let dir = match worktree {
        Some(child) => project.worktrees[child].path.as_path(),
        None => project.path.as_path(),
    };
    // Asked of git, never of the cache: the cached branch is whatever was
    // checked out at the last refresh, and a worktree moves between branches
    // without the picker being told.
    let branch = git::branch(host, dir)?;

    let default = match branch.is_empty() {
        true => String::new(),
        false => git::default_branch(host, config, dir)?,
    };
    // A checkout already on a feature branch is work in progress: its branch
    // names it and nothing is asked. Two things override that. A directory
    // that already has a session is being chosen over that session's own row,
    // which is asking for something new rather than to resume. And in a
    // worktree the branch is the likely name rather than the only one — the
    // same tree carries successive pieces of work — so it is offered: Enter
    // takes it, declining names the work from scratch.
    let started = !branch.is_empty() && branch != default;
    let running = sessions.iter().any(|session| session.path == dir);
    let named = naming::from_branch(&config.username, &branch);
    let keep = started
        && !running
        && (worktree.is_none() || host.confirm(&format!("Open {named}?"), true)?);
    let name = match keep {
        true => named,
        false => ask(host, config)?,
    };

    // Whenever the settled name is not the branch already checked out here,
    // the branch it names is offered — which is how work started against a
    // directory that is busy gets a worktree of its own.
    let dir = match !keep && !branch.is_empty() && name != branch {
        true => branch_for(host, config, dir, &name, &branch, &default)?,
        false => dir.to_path_buf(),
    };

    tmux::create(host, &name, &dir)?;
    // Frecency is tracked against the project, so a worktree credits its parent.
    state::record(host, &project.path)?;
    tmux::attach(host, &name)
}

/// Name the work from one line of prompt input: a ticket and a description in,
/// the name they imply offered as an editable default, never applied silently.
fn ask(host: &dyn Host, config: &Config) -> Result<String> {
    let said = host.input("What are you working on?", "")?;
    let suggested = naming::suggest(&config.username, &said);
    Ok(naming::sanitize(&host.input("Session name", &suggested)?))
}

/// Offer the branch the session was just named for: a new worktree beside the
/// checkout, in place in it, or neither. Returns the directory the work now
/// lives in — the worktree, or the checkout for the other two — with a cut
/// branch's submodules and LFS files brought in.
///
/// The worktree leads because it alters no existing checkout and so is always
/// safe; a dirty tree withholds the in-place option rather than offering one
/// that would be refused.
///
/// `here` is the branch this checkout is on, which is what declining leaves it
/// on; `default` is what a cut branch comes off, and the two differ whenever
/// work is started from a checkout that is already on a branch of its own.
fn branch_for(
    host: &dyn Host,
    config: &Config,
    main: &Path,
    name: &str,
    here: &str,
    default: &str,
) -> Result<PathBuf> {
    let worktree = naming::worktree_dir(main, &config.username, name);
    let dirty = git::is_dirty(host, main)?;
    if dirty {
        println!(
            "uncommitted changes in {}, so branching in place is not offered",
            main.display()
        );
    }

    let mut options = vec![format!("New worktree at {}", worktree.display())];
    if !dirty {
        options.push(format!("Branch in place in {}", main.display()));
    }
    options.push(format!("Neither, stay on {here}"));

    let choice = host.select(&format!("Create branch {name}?"), &options)?;
    if choice + 1 == options.len() {
        return Ok(main.to_path_buf());
    }

    // An existing worktree on this branch is the same work, so it is joined
    // rather than recreated — and no branch is cut, so nothing is fetched.
    if choice == 0 && host.exists(&worktree) {
        return reuse(host, main, &worktree, name);
    }

    let base = base_ref(host, config, main, default)?;
    let dir = match choice {
        0 => git::add_worktree(host, main, &worktree, name, &base).map(|()| worktree),
        _ => git::create_branch(host, main, name, &base).map(|()| main.to_path_buf()),
    }?;
    for failed in git::populate(host, &dir)? {
        println!("{failed}; run it in {} to finish", dir.display());
    }
    Ok(dir)
}

/// The ref a new branch is cut from: `origin/<default>` when the offered fetch
/// is accepted and succeeds, the local default branch otherwise.
///
/// Cutting from the remote ref rather than fast-forwarding the local one means
/// nothing existing is touched, so divergence cannot fail the cut. The fetch is
/// opt-in, pre-answered from config, and never fatal: a remote that cannot be
/// reached costs a warning and the local ref.
fn base_ref(host: &dyn Host, config: &Config, dir: &Path, default: &str) -> Result<String> {
    if !host.confirm("Fetch origin first?", config.fetch_before_branch)? {
        return Ok(default.to_string());
    }
    let out = git::fetch(host, dir)?;
    Ok(match out.succeeded() {
        true => format!("origin/{default}"),
        false => {
            println!(
                "fetch failed ({}), branching from local {default}",
                out.stderr.trim()
            );
            default.to_string()
        }
    })
}

/// A directory already sitting where the worktree would go: this repository's
/// worktree on this branch is reused, anything else stops the work.
/// Vibestation never writes into a directory it did not create.
fn reuse(host: &dyn Host, main: &Path, path: &Path, name: &str) -> Result<PathBuf> {
    let listed = group::worktrees(host, main)?;
    match listed
        .iter()
        .any(|worktree| worktree.path == path && worktree.branch == name)
    {
        true => Ok(path.to_path_buf()),
        false => Err(anyhow!(
            "{} already exists and is not a worktree on {name}",
            path.display()
        )),
    }
}
