//! Selecting a project or a worktree: what is asked, what tmux is told, and
//! what the frecency file remembers afterwards.

use std::path::PathBuf;
use vibestation::fake::{Answer, FakeHost};
use vibestation::state;

const LIST: &str = "tmux list-sessions -F #{session_attached}\t#{pane_current_path}\t#{pane_current_command}\t#{session_name}";
const BRANCH: &str = "git rev-parse --abbrev-ref HEAD";
const CONFIG: &str = "/home/dev/.vibestation/config.toml";
const CACHE: &str = "/home/dev/.vibestation/projects-cache.json";

/// No live sessions, so the picker is exactly the two cached projects: `api`
/// with a worktree, and `notes` without one. Row 0 is `api`, row 1 its
/// worktree, row 2 `notes`.
fn host() -> FakeHost {
    FakeHost::new()
        .file(
            CONFIG,
            "projects_dirs = [\"/home/dev/code\"]\nusername = \"ada\"\n",
        )
        .file(
            CACHE,
            r#"[
              {"name": "api", "path": "/home/dev/code/api",
               "worktrees": [{"path": "/home/dev/code/api-VBSN-1",
                              "branch": "ada/VBSN-1-init"}]},
              {"name": "notes", "path": "/home/dev/code/notes", "worktrees": []}
            ]"#,
        )
        .fails(LIST, 1, "no server running on /tmp/tmux-1000/default")
        .succeeds(&format!("/home/dev/code/api $ {BRANCH}"), "main\n")
        .succeeds(
            &format!("/home/dev/code/notes $ {BRANCH}"),
            "ada/notes-tidy\n",
        )
}

fn created(host: &FakeHost) -> Vec<String> {
    host.log()
        .into_iter()
        .filter(|command| command.starts_with("tmux new-session"))
        .collect()
}

/// None of the names these tests reach for has a session yet.
fn absent(host: FakeHost) -> FakeHost {
    [
        "ada/VBSN-4-picker-rows",
        "ada/something-else",
        "ada/notes-tidy",
        "ada/VBSN-1-init",
    ]
    .iter()
    .fold(host, |host, name| {
        host.fails(
            &format!("tmux has-session -t {name}"),
            1,
            "can't find session",
        )
    })
}

#[test]
fn a_checkout_on_the_default_branch_asks_once_and_offers_the_name() {
    let host = absent(host())
        .succeeds(
            "tmux new-session -d -s ada/VBSN-4-picker-rows -c /home/dev/code/api",
            "",
        )
        .answers([
            Answer::Select(0),
            Answer::text("VBSN-4 picker rows"),
            Answer::text("ada/VBSN-4-picker-rows"),
        ]);

    vibestation::run(&host).unwrap();

    assert_eq!(
        host.prompts(),
        [
            "Open",
            "What are you working on? []",
            "Session name [ada/VBSN-4-picker-rows]",
        ],
        "one line collects ticket and description, and the generated name is \
         offered rather than applied"
    );
    assert_eq!(
        host.log().last().unwrap(),
        "tmux attach-session -t ada/VBSN-4-picker-rows"
    );
}

#[test]
fn an_edited_name_is_the_one_used() {
    let host = absent(host())
        .succeeds(
            "tmux new-session -d -s ada/something-else -c /home/dev/code/api",
            "",
        )
        .answers([
            Answer::Select(0),
            Answer::text("VBSN-4 picker rows"),
            Answer::text("ada/something.else"),
        ]);

    vibestation::run(&host).unwrap();

    assert_eq!(
        created(&host),
        ["tmux new-session -d -s ada/something-else -c /home/dev/code/api"],
        "the edited name is still sanitised for tmux"
    );
}

#[test]
fn a_checkout_already_on_a_feature_branch_asks_nothing() {
    let host = absent(host())
        .succeeds(
            "tmux new-session -d -s ada/notes-tidy -c /home/dev/code/notes",
            "",
        )
        .answer(Answer::Select(2));

    vibestation::run(&host).unwrap();

    assert_eq!(host.prompts(), ["Open"], "resuming asks nothing");
    assert_eq!(
        created(&host),
        ["tmux new-session -d -s ada/notes-tidy -c /home/dev/code/notes"],
        "detached, rooted in the project, with no command or layout"
    );
    assert_eq!(
        host.log().last().unwrap(),
        "tmux attach-session -t ada/notes-tidy"
    );
}

#[test]
fn selecting_a_worktree_creates_a_session_in_it_with_no_prompts() {
    let host = absent(host())
        .succeeds(
            "tmux new-session -d -s ada/VBSN-1-init -c /home/dev/code/api-VBSN-1",
            "",
        )
        .answer(Answer::Select(1));

    vibestation::run(&host).unwrap();

    assert_eq!(host.prompts(), ["Open"]);
    assert_eq!(
        created(&host),
        ["tmux new-session -d -s ada/VBSN-1-init -c /home/dev/code/api-VBSN-1"],
        "the session is rooted in the worktree, named after its branch"
    );
}

#[test]
fn inside_tmux_the_new_session_is_switched_to_rather_than_attached() {
    let host = absent(host())
        .in_tmux(true)
        .succeeds(
            "tmux new-session -d -s ada/notes-tidy -c /home/dev/code/notes",
            "",
        )
        .answer(Answer::Select(2));

    vibestation::run(&host).unwrap();

    assert_eq!(
        host.log().last().unwrap(),
        "tmux switch-client -t ada/notes-tidy"
    );
}

#[test]
fn the_open_counts_towards_the_project_not_the_worktree() {
    let host = absent(host())
        .succeeds(
            "tmux new-session -d -s ada/VBSN-1-init -c /home/dev/code/api-VBSN-1",
            "",
        )
        .answer(Answer::Select(1));

    vibestation::run(&host).unwrap();

    let opens = state::load(&host).unwrap();
    assert_eq!(opens.len(), 1);
    assert_eq!(opens[0].path, PathBuf::from("/home/dev/code/api"));
    assert_eq!(opens[0].count, 1);
    assert_eq!(opens[0].last, 1_700_000_000, "the timestamp is bumped too");
}

#[test]
fn a_refused_session_stops_before_attaching() {
    let host = absent(host())
        .fails(
            "tmux new-session -d -s ada/notes-tidy -c /home/dev/code/notes",
            1,
            "duplicate session: ada/notes-tidy",
        )
        .answer(Answer::Select(2));

    let error = vibestation::run(&host).unwrap_err();

    assert!(
        format!("{error:#}").contains("duplicate session"),
        "the failure names what tmux said: {error:#}"
    );
    assert!(
        !host.log().iter().any(|c| c.contains("attach-session")),
        "and nothing was joined"
    );
    assert!(host.writes().is_empty(), "nor was the open recorded");
}

#[test]
fn a_session_that_already_has_the_name_is_joined_rather_than_recreated() {
    let host = host()
        .succeeds("tmux has-session -t ada/notes-tidy", "")
        .answer(Answer::Select(2));

    vibestation::run(&host).unwrap();

    assert!(created(&host).is_empty(), "nothing is created over it");
    assert_eq!(
        host.log().last().unwrap(),
        "tmux attach-session -t ada/notes-tidy",
        "the existing session is the resume path, not an error"
    );
}
