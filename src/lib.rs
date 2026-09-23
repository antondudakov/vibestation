pub mod config;
pub mod fake;
pub mod git;
pub mod group;
pub mod host;
pub mod line;
pub mod naming;
pub mod picker;
pub mod scan;
pub mod select;
pub mod state;
pub mod tmux;

use anyhow::{anyhow, Result};
use config::Config;
use host::{Host, Pick};
use picker::Row;
use scan::Project;
use std::path::{Path, PathBuf};
use tmux::Session;

/// What ← and → do in the picker, under it.
const KEYS: &str = "↑↓ move · type to filter · enter select · ← refresh · → more · esc cancel";

/// And in a row's menu.
const MENU_KEYS: &str = "↑↓ move · enter select · ← back · esc cancel";

/// What can be done to a row. Indexes are into the lists the picker was
/// built from, as a [`Row`]'s are.
#[derive(Debug, Clone, Copy)]
enum Action {
    Join(usize),
    Kill(usize),
    Rename(usize),
    /// A project, or one of its worktrees.
    Start(usize, Option<usize>),
    Edit(usize, Option<usize>),
    /// Take a manually added project off the list.
    Forget(usize),
    /// A project and one of its worktrees.
    Remove(usize, usize),
    Rescan,
    Add,
}

/// The single entry point tests drive: settle the config, open one picker over
/// the live sessions and the ranked projects, and act on the choice.
///
/// Joining a session and starting or editing work leave the picker; every
/// other action changes the list and reopens the picker over what it changed.
pub fn run(host: &dyn Host) -> Result<()> {
    let mut config = config::load_or_init(host)?;
    let mut found = scan::load_or_scan(host, &config)?;
    // Read again only after killing or renaming one.
    let mut sessions = tmux::list(host)?;
    // Read once: nothing the picker does touches the frecency records.
    let opens = state::load(host)?;
    let home = host.home()?;

    loop {
        // Cloned because most actions reopen the picker over the same list.
        let projects = state::rank(found.clone(), &opens, host.now());
        let rows = picker::rows(
            &sessions,
            &projects,
            &home,
            &config.username,
            host.terminal().0,
        );
        let labels: Vec<String> = rows.iter().map(|(_, label)| label.clone()).collect();

        let chosen = match host.pick(&picker::title(&rows), &labels, KEYS)? {
            Pick::Left => Some(Action::Rescan),
            Pick::Enter(index) => actions(host, rows[index].0, &sessions, &projects, &config)
                .1
                .first()
                .map(|(action, _)| *action),
            Pick::Right(index) => more(
                host,
                actions(host, rows[index].0, &sessions, &projects, &config),
            )?,
        };
        // The separator, or ← back out of a menu: the list comes back as it was.
        let Some(action) = chosen else { continue };

        match action {
            Action::Join(index) => return tmux::attach(host, &sessions[index].name),
            Action::Kill(index) => {
                let name = &sessions[index].name;
                if host.confirm(&format!("Kill {name}?"), false)? {
                    tmux::kill(host, name)?;
                    sessions = tmux::list(host)?;
                }
            }
            Action::Rename(index) => {
                let name = &sessions[index].name;
                let to = naming::sanitize(host.input("Session name", name)?.trim());
                if !to.is_empty() && to != *name {
                    // A taken name is a slip, not a fault: the picker is still
                    // there to try again from.
                    if let Err(refused) = tmux::rename(host, name, &to) {
                        println!("{refused}");
                    }
                    sessions = tmux::list(host)?;
                }
            }
            Action::Start(index, child) => {
                return open(host, &config, &sessions, &projects[index], child)
            }
            Action::Edit(index, child) => return edit(host, &projects[index], child),
            Action::Forget(index) => {
                config::remove_extra(host, &mut config, &projects[index].path)?;
                found = scan::rescan(host, &config)?;
            }
            Action::Remove(index, child) => {
                if remove(host, &sessions, &projects[index], child)? {
                    found = scan::rescan(host, &config)?;
                }
            }
            Action::Rescan => found = scan::rescan(host, &config)?,
            Action::Add => {
                add(host, &mut config)?;
                found = scan::rescan(host, &config)?;
            }
        }
    }
}

/// What can be done to a row, each labelled for its menu, under the name the
/// menu is titled with. The first is what Enter does; → offers them all.
fn actions(
    host: &dyn Host,
    row: Row,
    sessions: &[Session],
    projects: &[Project],
    config: &Config,
) -> (String, Vec<(Action, String)>) {
    let editor = host.editor();
    let edit = format!(
        "Open in {}",
        editor
            .split_whitespace()
            .next()
            .and_then(|bin| Path::new(bin).file_name())
            .map_or("the editor".into(), |bin| bin.to_string_lossy())
    );
    let offer = |pairs: &[(Action, &str)]| {
        pairs
            .iter()
            .map(|(action, label)| (*action, label.to_string()))
            .collect()
    };

    match row {
        Row::Session(index) => (
            sessions[index].name.clone(),
            offer(&[
                (Action::Join(index), "Join"),
                (Action::Kill(index), "Kill"),
                (Action::Rename(index), "Rename"),
            ]),
        ),
        Row::Project(index) => {
            let project = &projects[index];
            let mut offered = offer(&[
                (Action::Start(index, None), "New session"),
                (Action::Edit(index, None), &edit),
            ]);
            // Only what was added by hand can be taken off by hand: a scanned
            // project would be found again by the next refresh.
            if config.extra_projects.contains(&project.path) {
                offered.push((Action::Forget(index), "Remove from the list".into()));
            }
            (project.name.clone(), offered)
        }
        // Titled with the directory's name, as its row names it.
        Row::Worktree(index, child) => (
            projects[index].worktrees[child]
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            offer(&[
                (Action::Start(index, Some(child)), "New session"),
                (Action::Edit(index, Some(child)), &edit),
                (Action::Remove(index, child), "Remove the worktree"),
            ]),
        ),
        Row::Separator => (String::new(), Vec::new()),
        Row::AddManually => (String::new(), offer(&[(Action::Add, "")])),
    }
}

/// → on a row: its menu, when it has more to offer than the one thing Enter
/// does. ← goes back to the list, which is `None`.
fn more(
    host: &dyn Host,
    (title, offered): (String, Vec<(Action, String)>),
) -> Result<Option<Action>> {
    if offered.len() < 2 {
        return Ok(None);
    }
    let labels: Vec<String> = offered.iter().map(|(_, label)| label.clone()).collect();
    Ok(match host.pick(&title, &labels, MENU_KEYS)? {
        Pick::Left => None,
        Pick::Enter(index) | Pick::Right(index) => Some(offered[index].0),
    })
}

/// Open the project or worktree in the developer's editor, in this terminal,
/// from inside the directory. It counts as an open, as a session would.
fn edit(host: &dyn Host, project: &Project, worktree: Option<usize>) -> Result<()> {
    state::record(host, &project.path)?;
    let editor = host.editor();
    let mut argv: Vec<&str> = editor.split_whitespace().collect();
    argv.push(".");
    host.exec(&argv, Some(project.dir(worktree)))
}

/// Remove a worktree — never forced, and after refusing what git would not:
/// a session still sitting in it, whose shell would be left in a directory
/// that is gone. Says whether it went.
fn remove(host: &dyn Host, sessions: &[Session], project: &Project, child: usize) -> Result<bool> {
    let path = &project.worktrees[child].path;
    if let Some(session) = sessions
        .iter()
        .find(|session| session.path.starts_with(path))
    {
        println!(
            "{} is running in {}, so it is not removed",
            session.name,
            path.display()
        );
        return Ok(false);
    }
    // Constitution §4, said up front rather than left to git's refusal.
    if git::is_dirty(host, path)? {
        println!(
            "uncommitted changes in {}, so it is not removed",
            path.display()
        );
        return Ok(false);
    }
    // Clean is not empty: what git ignores — builds, `.env` — goes with it.
    if !host.confirm(&format!("Remove {}?", path.display()), false)? {
        return Ok(false);
    }
    git::remove_worktree(host, &project.path, path)?;
    Ok(true)
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
    let dir = project.dir(worktree);
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
/// lives in — the worktree, or the checkout for the other two.
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
