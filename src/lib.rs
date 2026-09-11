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

        match rows[host.select("Open", &labels)?].0 {
            Row::Session(index) => return tmux::attach(host, &sessions[index].name),
            // Decoration: choosing it costs nothing and reopens the list.
            Row::Separator => continue,
            Row::Project(index) => return open(host, &config, &projects[index], None),
            Row::Worktree(index, child) => {
                return open(host, &config, &projects[index], Some(child))
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

    let dir = match !started && !branch.is_empty() && name != branch {
        true => branch_for(host, config, dir, &name, &default)?,
        false => dir.to_path_buf(),
    };

    tmux::create(host, &name, &dir)?;
    // Frecency is tracked against the project, so a worktree credits its parent.
    state::record(host, &project.path)?;
    tmux::attach(host, &name)
}

/// Offer the branch the session was just named for: a new worktree beside the
/// checkout, in place in it, or neither. Returns the directory the work now
/// lives in — the worktree, or the checkout for the other two.
///
/// The worktree leads because it alters no existing checkout and so is always
/// safe; a dirty tree withholds the in-place option rather than offering one
/// that would be refused.
fn branch_for(
    host: &dyn Host,
    config: &Config,
    main: &Path,
    name: &str,
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
    options.push(format!("Neither, stay on {default}"));

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
    match choice {
        0 => git::add_worktree(host, main, &worktree, name, &base).map(|()| worktree),
        _ => git::create_branch(host, main, name, &base).map(|()| main.to_path_buf()),
    }
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
