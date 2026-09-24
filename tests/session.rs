//! Selecting a project or a worktree: what is asked, what tmux is told, and
//! what the frecency file remembers afterwards.

use std::path::PathBuf;
use vibestation::fake::{Answer, FakeHost};
use vibestation::state;

const LIST: &str = "tmux list-sessions -F #{session_attached}\t#{session_last_attached}\t#{pane_current_path}\t#{pane_current_command}\t#{@note}\t#{session_name}";
const BRANCH: &str = "git rev-parse --abbrev-ref HEAD";
const ORIGIN_HEAD: &str = "git symbolic-ref --short refs/remotes/origin/HEAD";
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
        // The worktree has been checked out onto something else since the
        // cache above was written, which is what the cache cannot know.
        .succeeds(
            &format!("/home/dev/code/api-VBSN-1 $ {BRANCH}"),
            "ada/VBSN-2-retries\n",
        )
        .succeeds(
            &format!("/home/dev/code/api $ {ORIGIN_HEAD}"),
            "origin/main\n",
        )
        .succeeds(
            &format!("/home/dev/code/api-VBSN-1 $ {ORIGIN_HEAD}"),
            "origin/main\n",
        )
        .succeeds(
            &format!("/home/dev/code/notes $ {ORIGIN_HEAD}"),
            "origin/main\n",
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
        "ada/VBSN-2-retries",
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
        .succeeds(
            "/home/dev/code/api $ git status --porcelain --ignore-submodules=none",
            "",
        )
        .answers([
            Answer::Select(0),
            Answer::text("VBSN-4 picker rows"),
            Answer::text("ada/VBSN-4-picker-rows"),
            Answer::Select(2),
        ]);

    vibestation::run(&host).unwrap();

    assert_eq!(
        host.prompts(),
        [
            "Open  2 projects",
            "What are you working on? []",
            "Session name [ada/VBSN-4-picker-rows]",
            "Create branch ada/VBSN-4-picker-rows?",
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
        .succeeds(
            "/home/dev/code/api $ git status --porcelain --ignore-submodules=none",
            "",
        )
        .answers([
            Answer::Select(0),
            Answer::text("VBSN-4 picker rows"),
            Answer::text("ada/something.else"),
            Answer::Select(2),
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

    assert_eq!(
        host.prompts(),
        ["Open  2 projects"],
        "resuming asks nothing"
    );
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
fn a_worktree_is_named_for_the_branch_it_is_on_now_not_the_cached_one() {
    let host = absent(host())
        .succeeds(
            "tmux new-session -d -s ada/VBSN-2-retries -c /home/dev/code/api-VBSN-1",
            "",
        )
        .answers([Answer::Select(1), Answer::Confirm(true)]);

    vibestation::run(&host).unwrap();

    assert_eq!(
        host.prompts(),
        ["Open  2 projects", "Open ada/VBSN-2-retries? [Y/n]"],
        "the name offered is the branch git reports, not ada/VBSN-1-init from \
         the cache; taking it is one keypress"
    );
    assert_eq!(
        created(&host),
        ["tmux new-session -d -s ada/VBSN-2-retries -c /home/dev/code/api-VBSN-1"],
        "the session is rooted in the worktree, named after the branch in it"
    );
}

#[test]
fn declining_the_offered_name_names_the_work_from_scratch() {
    let host = absent(host())
        .succeeds(
            "tmux new-session -d -s ada/something-else -c /home/dev/code/api-VBSN-1",
            "",
        )
        .succeeds(
            "/home/dev/code/api-VBSN-1 $ git status --porcelain --ignore-submodules=none",
            "",
        )
        .answers([
            Answer::Select(1),
            Answer::Confirm(false),
            Answer::text("VBSN-4 picker rows"),
            Answer::text("ada/something-else"),
            Answer::Select(2),
        ]);

    vibestation::run(&host).unwrap();

    assert_eq!(
        host.prompts(),
        [
            "Open  2 projects",
            "Open ada/VBSN-2-retries? [Y/n]",
            "What are you working on? []",
            "Session name [ada/VBSN-4-picker-rows]",
            "Create branch ada/something-else?",
        ],
        "declining falls through to the same questions a project asks, the \
         branch among them"
    );
    assert_eq!(
        host.options().last().unwrap(),
        "Neither, stay on ada/VBSN-2-retries",
        "declining the branch leaves the worktree on the branch it is on, not \
         on the default one"
    );
    assert_eq!(
        created(&host),
        ["tmux new-session -d -s ada/something-else -c /home/dev/code/api-VBSN-1"],
        "named freshly, still rooted in the worktree and cutting no branch"
    );
}

/// The headline of listing a busy project anyway: its session row resumes the
/// work that is running, and the project row below it starts something else in
/// a worktree of its own, without disturbing either.
#[test]
fn a_project_whose_session_is_running_can_still_start_new_work() {
    let host = absent(host())
        .succeeds(LIST, "1\t0\t/home/dev/code/notes\tnvim\t\tada/notes-tidy\n")
        .succeeds(
            "/home/dev/code/notes $ git status --porcelain --ignore-submodules=none",
            "",
        )
        .succeeds(
            "/home/dev/code/notes $ git worktree add --no-track -b \
             ada/VBSN-4-picker-rows /home/dev/code/notes-VBSN-4-picker-rows main",
            "",
        )
        .succeeds(
            "/home/dev/code/notes-VBSN-4-picker-rows $ git submodule update --init --recursive",
            "",
        )
        .succeeds(
            "tmux new-session -d -s ada/VBSN-4-picker-rows \
             -c /home/dev/code/notes-VBSN-4-picker-rows",
            "",
        )
        .answers([
            // Row 0 is the running session, 1 the separator, 2 `api` with its
            // worktree at 3, and 4 is `notes` — the project the session is in.
            Answer::Select(4),
            Answer::text("VBSN-4 picker rows"),
            Answer::text("ada/VBSN-4-picker-rows"),
            Answer::Select(0),
            Answer::Confirm(false),
        ]);

    vibestation::run(&host).unwrap();

    assert_eq!(
        host.prompts(),
        [
            "Open  1 running · 2 projects",
            "What are you working on? []",
            "Session name [ada/VBSN-4-picker-rows]",
            "Create branch ada/VBSN-4-picker-rows?",
            "Fetch origin first? [Y/n]",
        ],
        "picking the project over its own session row means new work, so the \
         branch it is sitting on is not offered as the name"
    );
    assert_eq!(
        created(&host),
        ["tmux new-session -d -s ada/VBSN-4-picker-rows \
          -c /home/dev/code/notes-VBSN-4-picker-rows"],
        "the new session is rooted in the new worktree"
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
            "tmux new-session -d -s ada/VBSN-2-retries -c /home/dev/code/api-VBSN-1",
            "",
        )
        .answers([Answer::Select(1), Answer::Confirm(true)]);

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
