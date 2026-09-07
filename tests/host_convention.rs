//! The testing convention, demonstrated end to end.
//!
//! Every later ticket tests the same way: script a `FakeHost`, run application
//! logic through it, assert on the command log, the file writes and the
//! prompts. `session_setup` below stands in for that application logic until
//! there is some — it is written the way the real modules will be, as a pure
//! function over `Host`.

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use vibestation::fake::{Answer, FakeHost};
use vibestation::host::Host;

/// Reads config (absent means first run), asks where projects live, finds the
/// repositories under it, reads the branch of the one picked, asks whether to
/// create a session, and records the choice.
fn session_setup(host: &dyn Host) -> Result<Option<PathBuf>> {
    let config = Path::new("/home/dev/.vibestation/config.toml");
    let root = match host.read_file(config)? {
        Some(existing) => existing.trim().to_string(),
        None => {
            let answered = host.input("Where do your projects live?", "/home/dev/code")?;
            host.write_file(config, &format!("{answered}\n"))?;
            answered
        }
    };

    let repos: Vec<PathBuf> = host
        .walk(Path::new(&root), 10)?
        .into_iter()
        .filter(|dir| host.exists(&dir.join(".git")))
        .collect();

    let labels: Vec<String> = repos.iter().map(|r| r.display().to_string()).collect();
    let picked = &repos[host.select("Project", &labels)?];

    let branch = host.run(&["git", "rev-parse", "--abbrev-ref", "HEAD"], Some(picked))?;
    if !host.confirm(&format!("Start a session on {}?", branch.trimmed()), true)? {
        return Ok(None);
    }

    let name = format!("dev/{}", branch.trimmed());
    host.run(&["tmux", "new-session", "-d", "-s", &name], Some(picked))?;
    let opened = host.now().duration_since(UNIX_EPOCH)?.as_secs();
    host.write_file(
        Path::new("/home/dev/.vibestation/state.json"),
        &format!("{{\"{}\":{}}}\n", picked.display(), opened),
    )?;
    Ok(Some(picked.clone()))
}

fn host() -> FakeHost {
    FakeHost::new()
        .file(
            "/home/dev/code/vibestation/.git/HEAD",
            "ref: refs/heads/main",
        )
        .file("/home/dev/code/vibestation/Cargo.toml", "")
        .file("/home/dev/code/notes/README.md", "not a repo")
        .succeeds(
            "/home/dev/code/vibestation $ git rev-parse --abbrev-ref HEAD",
            "main\n",
        )
        .succeeds("tmux new-session -d -s dev/main", "")
        .now(UNIX_EPOCH + Duration::from_secs(1_700_000_000))
}

#[test]
fn first_run_writes_config_then_creates_a_session() {
    let host = host().answers([
        Answer::text("/home/dev/code"),
        Answer::Select(0),
        Answer::Confirm(true),
    ]);

    let picked = session_setup(&host).unwrap();

    assert_eq!(picked, Some(PathBuf::from("/home/dev/code/vibestation")));
    assert_eq!(
        host.log(),
        [
            "/home/dev/code/vibestation $ git rev-parse --abbrev-ref HEAD",
            "/home/dev/code/vibestation $ tmux new-session -d -s dev/main",
        ]
    );
    assert_eq!(
        host.writes(),
        [
            (
                PathBuf::from("/home/dev/.vibestation/config.toml"),
                "/home/dev/code\n".to_string()
            ),
            (
                PathBuf::from("/home/dev/.vibestation/state.json"),
                "{\"/home/dev/code/vibestation\":1700000000}\n".to_string()
            ),
        ]
    );
    assert_eq!(
        host.prompts(),
        [
            "Where do your projects live? [/home/dev/code]",
            "Project",
            "Start a session on main? [Y/n]",
        ]
    );
}

#[test]
fn an_existing_config_skips_the_first_run_prompt() {
    let host = host()
        .file("/home/dev/.vibestation/config.toml", "/home/dev/code\n")
        .answers([Answer::Select(0), Answer::Confirm(true)]);

    session_setup(&host).unwrap();

    assert_eq!(host.prompts()[0], "Project");
    assert_eq!(host.writes().len(), 1, "config is not rewritten");
}

#[test]
fn declining_the_confirmation_emits_no_tmux_command() {
    let host = host()
        .file("/home/dev/.vibestation/config.toml", "/home/dev/code\n")
        .answers([Answer::Select(0), Answer::Confirm(false)]);

    assert_eq!(session_setup(&host).unwrap(), None);
    assert_eq!(
        host.log(),
        ["/home/dev/code/vibestation $ git rev-parse --abbrev-ref HEAD"]
    );
    assert!(host.writes().is_empty());
}
