//! Cutting the branch the session was just named for: which default branch is
//! detected, when the offer appears at all, and what the fetch does or does
//! not emit.

use vibestation::fake::{Answer, FakeHost};

const LIST: &str = "tmux list-sessions -F #{session_attached}\t#{pane_current_path}\t#{pane_current_command}\t#{session_name}";
const BRANCH: &str = "/home/dev/code/api $ git rev-parse --abbrev-ref HEAD";
const ORIGIN_HEAD: &str = "/home/dev/code/api $ git symbolic-ref --short refs/remotes/origin/HEAD";
const MAIN: &str = "/home/dev/code/api $ git rev-parse --verify --quiet refs/heads/main";
const STATUS: &str = "/home/dev/code/api $ git status --porcelain";
const FETCH: &str = "/home/dev/code/api $ git fetch origin";
const SESSION: &str = "tmux new-session -d -s ada/VBSN-4-tidy -c /home/dev/code/api";
const CONFIG: &str = "/home/dev/.vibestation/config.toml";
const CACHE: &str = "/home/dev/.vibestation/projects-cache.json";

/// One project, `api`, sitting on its default branch with a clean tree — the
/// only path on which a branch is offered. Row 0 is the project.
fn host(config: &str) -> FakeHost {
    FakeHost::new()
        .file(CONFIG, config)
        .file(
            CACHE,
            r#"[{"name": "api", "path": "/home/dev/code/api", "worktrees": []}]"#,
        )
        .fails(LIST, 1, "no server running")
        .fails(
            "tmux has-session -t ada/VBSN-4-tidy",
            1,
            "can't find session",
        )
        .succeeds(SESSION, "")
        .succeeds(STATUS, "")
}

/// The default config, nothing configured beyond the first-run question.
fn plain() -> FakeHost {
    host("projects_dirs = [\"/home/dev/code\"]\nusername = \"ada\"\n")
}

/// A repository whose `origin/HEAD` answers `main`.
fn detected() -> FakeHost {
    plain()
        .succeeds(BRANCH, "main\n")
        .succeeds(ORIGIN_HEAD, "origin/main\n")
}

/// Pick the project, accept the suggested name, then answer the branch
/// prompts.
fn choosing(host: FakeHost, answers: impl IntoIterator<Item = Answer>) -> FakeHost {
    host.answers([
        Answer::Select(0),
        Answer::text("VBSN-4 tidy"),
        Answer::text("ada/VBSN-4-tidy"),
    ])
    .answers(answers)
}

fn git(host: &FakeHost) -> Vec<String> {
    host.log()
        .into_iter()
        .filter(|command| command.contains("git "))
        .collect()
}

#[test]
fn origin_head_names_the_default_branch() {
    let host = choosing(
        plain()
            .succeeds(BRANCH, "develop\n")
            .succeeds(ORIGIN_HEAD, "origin/develop\n")
            .succeeds(FETCH, "")
            .succeeds(
                "/home/dev/code/api $ git checkout -b ada/VBSN-4-tidy origin/develop",
                "",
            ),
        [Answer::Select(1), Answer::Confirm(true)],
    );

    vibestation::run(&host).unwrap();

    assert!(
        git(&host).iter().any(|c| c.ends_with("origin/develop")),
        "the remote HEAD decides, and nothing else is asked: {:?}",
        git(&host)
    );
}

#[test]
fn without_origin_head_a_local_main_is_the_default_branch() {
    let host = choosing(
        plain()
            .succeeds(BRANCH, "main\n")
            .fails(ORIGIN_HEAD, 128, "is not a symbolic ref")
            .succeeds(MAIN, "9f1c\n")
            .succeeds(
                "/home/dev/code/api $ git checkout -b ada/VBSN-4-tidy main",
                "",
            ),
        [Answer::Select(1), Answer::Confirm(false)],
    );

    vibestation::run(&host).unwrap();

    assert!(git(&host)
        .iter()
        .any(|c| c.ends_with("-b ada/VBSN-4-tidy main")));
}

#[test]
fn without_either_the_default_branch_is_master() {
    let host = choosing(
        plain()
            .succeeds(BRANCH, "master\n")
            .fails(ORIGIN_HEAD, 128, "is not a symbolic ref")
            .fails(MAIN, 1, "")
            .succeeds(
                "/home/dev/code/api $ git checkout -b ada/VBSN-4-tidy master",
                "",
            ),
        [Answer::Select(1), Answer::Confirm(false)],
    );

    vibestation::run(&host).unwrap();

    assert!(git(&host)
        .iter()
        .any(|c| c.ends_with("-b ada/VBSN-4-tidy master")));
}

#[test]
fn a_configured_default_branch_wins_and_asks_git_nothing() {
    let host = choosing(
        host("projects_dirs = [\"/home/dev/code\"]\nusername = \"ada\"\ndefault_branch = \"develop\"\n")
            .succeeds(BRANCH, "develop\n")
            .succeeds("/home/dev/code/api $ git checkout -b ada/VBSN-4-tidy develop", ""),
        [Answer::Select(1), Answer::Confirm(false)],
    );

    vibestation::run(&host).unwrap();

    assert!(
        !git(&host).iter().any(|c| c.contains("symbolic-ref")),
        "the override short-circuits detection: {:?}",
        git(&host)
    );
    assert!(git(&host)
        .iter()
        .any(|c| c.ends_with("-b ada/VBSN-4-tidy develop")));
}

#[test]
fn accepting_the_fetch_cuts_from_the_remote_ref_and_touches_no_local_one() {
    let host = choosing(
        detected().succeeds(FETCH, "").succeeds(
            "/home/dev/code/api $ git checkout -b ada/VBSN-4-tidy origin/main",
            "",
        ),
        [Answer::Select(1), Answer::Confirm(true)],
    );

    vibestation::run(&host).unwrap();

    assert_eq!(
        host.prompts().last().unwrap(),
        "Fetch origin first? [Y/n]",
        "the fetch confirmation follows the strategy choice, pre-answered \
         from fetch_before_branch"
    );
    assert_eq!(
        git(&host).last().unwrap(),
        "/home/dev/code/api $ git checkout -b ada/VBSN-4-tidy origin/main"
    );
    assert!(
        !git(&host)
            .iter()
            .any(|c| c.contains("merge") || c.contains("pull")),
        "the local main is never moved: {:?}",
        git(&host)
    );
    assert_eq!(
        host.log().last().unwrap(),
        "tmux attach-session -t ada/VBSN-4-tidy",
        "and the session follows the branch"
    );
}

#[test]
fn declining_the_fetch_emits_none_and_cuts_from_the_local_ref() {
    let host = choosing(
        detected().succeeds(
            "/home/dev/code/api $ git checkout -b ada/VBSN-4-tidy main",
            "",
        ),
        [Answer::Select(1), Answer::Confirm(false)],
    );

    vibestation::run(&host).unwrap();

    assert!(!git(&host).iter().any(|c| c.contains("fetch")));
    assert!(git(&host)
        .iter()
        .any(|c| c.ends_with("-b ada/VBSN-4-tidy main")));
}

#[test]
fn the_fetch_default_comes_from_config() {
    let host = choosing(
        host(
            "projects_dirs = [\"/home/dev/code\"]\nusername = \"ada\"\nfetch_before_branch = false\n",
        )
            .succeeds(BRANCH, "main\n")
            .succeeds(ORIGIN_HEAD, "origin/main\n")
            .succeeds("/home/dev/code/api $ git checkout -b ada/VBSN-4-tidy main", ""),
        [Answer::Select(1), Answer::Confirm(false)],
    );

    vibestation::run(&host).unwrap();

    assert_eq!(host.prompts().last().unwrap(), "Fetch origin first? [y/N]");
}

#[test]
fn a_failing_fetch_warns_and_branches_from_the_local_ref() {
    let host = choosing(
        detected()
            .fails(FETCH, 128, "could not resolve host: github.com")
            .succeeds(
                "/home/dev/code/api $ git checkout -b ada/VBSN-4-tidy main",
                "",
            ),
        [Answer::Select(1), Answer::Confirm(true)],
    );

    vibestation::run(&host).unwrap();

    assert!(
        git(&host)
            .iter()
            .any(|c| c.ends_with("-b ada/VBSN-4-tidy main")),
        "offline is not a reason to stop working: {:?}",
        git(&host)
    );
    assert_eq!(
        host.log().last().unwrap(),
        "tmux attach-session -t ada/VBSN-4-tidy"
    );
}

#[test]
fn declining_the_branch_still_opens_the_session_and_mutates_nothing() {
    let host = choosing(detected(), [Answer::Select(2)]);

    vibestation::run(&host).unwrap();

    assert!(
        !git(&host)
            .iter()
            .any(|c| c.contains("checkout") || c.contains("fetch")),
        "no git state changed: {:?}",
        git(&host)
    );
    assert_eq!(
        host.log().last().unwrap(),
        "tmux attach-session -t ada/VBSN-4-tidy"
    );
}

#[test]
fn a_dirty_checkout_keeps_the_worktree_option_and_loses_the_in_place_one() {
    let host = choosing(
        detected().succeeds(STATUS, " M src/lib.rs\n"),
        [Answer::Select(1)],
    );

    vibestation::run(&host).unwrap();

    assert_eq!(
        host.options(),
        [
            "New worktree at /home/dev/code/api-VBSN-4-tidy",
            "Neither, stay on main",
        ],
        "work in progress does not block starting something new"
    );
    assert!(!git(&host).iter().any(|c| c.contains("checkout -b")));
    assert_eq!(
        host.log().last().unwrap(),
        "tmux attach-session -t ada/VBSN-4-tidy",
        "and the session is still created, on the current branch"
    );
}

#[test]
fn nothing_is_fetched_on_the_path_to_an_existing_session() {
    let host = FakeHost::new()
        .file(
            CONFIG,
            "projects_dirs = [\"/home/dev/code\"]\nusername = \"ada\"\n",
        )
        .file(CACHE, r#"[]"#)
        .succeeds(LIST, "0\t/home/dev/code/api\tnvim\tada/VBSN-4-tidy\n")
        .succeeds(BRANCH, "ada/VBSN-4-tidy\n")
        .answer(Answer::Select(0));

    vibestation::run(&host).unwrap();

    assert_eq!(
        host.log(),
        [
            LIST.to_string(),
            BRANCH.to_string(),
            "tmux attach-session -t ada/VBSN-4-tidy".to_string(),
        ],
        "attaching reads the branch and nothing else"
    );
}

#[test]
fn a_name_that_is_already_the_current_branch_offers_no_branch() {
    let host = detected()
        .fails("tmux has-session -t main", 1, "can't find session")
        .succeeds("tmux new-session -d -s main -c /home/dev/code/api", "")
        .answers([
            Answer::Select(0),
            Answer::text("VBSN-4 tidy"),
            Answer::text("main"),
        ]);

    vibestation::run(&host).unwrap();

    assert_eq!(
        host.prompts().len(),
        3,
        "there is no branch to cut: {:?}",
        host.prompts()
    );
    assert!(
        !git(&host).iter().any(|c| c.contains("status")),
        "nor is the tree inspected: {:?}",
        git(&host)
    );
    assert_eq!(host.log().last().unwrap(), "tmux attach-session -t main");
}

#[test]
fn the_worktree_is_the_pre_selected_option_and_lands_beside_the_checkout() {
    let host = choosing(
        detected().succeeds(FETCH, "").succeeds(
            "/home/dev/code/api $ git worktree add -b ada/VBSN-4-tidy \
             /home/dev/code/api-VBSN-4-tidy origin/main",
            "",
        ),
        [Answer::Select(0), Answer::Confirm(true)],
    )
    .fails(
        "tmux has-session -t ada/VBSN-4-tidy",
        1,
        "can't find session",
    )
    .succeeds(
        "tmux new-session -d -s ada/VBSN-4-tidy -c /home/dev/code/api-VBSN-4-tidy",
        "",
    );

    vibestation::run(&host).unwrap();

    assert_eq!(
        host.options(),
        [
            "New worktree at /home/dev/code/api-VBSN-4-tidy",
            "Branch in place in /home/dev/code/api",
            "Neither, stay on main",
        ],
        "the always-safe option is the one keypress choice, and the directory \
         is the project name joined to the branch without its username prefix"
    );
    assert_eq!(
        git(&host).last().unwrap(),
        "/home/dev/code/api $ git worktree add -b ada/VBSN-4-tidy \
         /home/dev/code/api-VBSN-4-tidy origin/main",
        "branch and worktree are one operation, cut from the fetched ref"
    );
    assert_eq!(
        host.log().last().unwrap(),
        "tmux attach-session -t ada/VBSN-4-tidy",
        "and the session is rooted in the worktree"
    );
}

#[test]
fn an_existing_worktree_on_that_branch_is_reused_rather_than_recreated() {
    let host = choosing(
        detected()
            .file(
                "/home/dev/code/api-VBSN-4-tidy/.git",
                "gitdir: /home/dev/code/api/.git/worktrees/tidy",
            )
            .succeeds(
                "/home/dev/code/api $ git worktree list --porcelain",
                "worktree /home/dev/code/api\nbranch refs/heads/main\n\n\
                 worktree /home/dev/code/api-VBSN-4-tidy\nbranch refs/heads/ada/VBSN-4-tidy\n",
            ),
        [Answer::Select(0)],
    )
    .fails(
        "tmux has-session -t ada/VBSN-4-tidy",
        1,
        "can't find session",
    )
    .succeeds(
        "tmux new-session -d -s ada/VBSN-4-tidy -c /home/dev/code/api-VBSN-4-tidy",
        "",
    );

    vibestation::run(&host).unwrap();

    assert!(
        !git(&host).iter().any(|c| c.contains("worktree add")),
        "repeating the action is harmless: {:?}",
        git(&host)
    );
    assert!(
        !git(&host).iter().any(|c| c.contains("fetch")),
        "and no branch is cut, so nothing is fetched"
    );
    assert_eq!(
        host.log().last().unwrap(),
        "tmux attach-session -t ada/VBSN-4-tidy"
    );
}

#[test]
fn a_path_occupied_by_anything_else_stops_the_operation() {
    let host = choosing(
        detected()
            .file("/home/dev/code/api-VBSN-4-tidy/notes.md", "not a worktree")
            .succeeds(
                "/home/dev/code/api $ git worktree list --porcelain",
                "worktree /home/dev/code/api\nbranch refs/heads/main\n",
            ),
        [Answer::Select(0)],
    );

    let error = vibestation::run(&host).unwrap_err();

    assert!(
        format!("{error:#}").contains("/home/dev/code/api-VBSN-4-tidy"),
        "the message names the path: {error:#}"
    );
    assert!(
        !git(&host).iter().any(|c| c.contains("worktree add")),
        "vibestation never writes into a directory it did not create"
    );
    assert!(
        !host.log().iter().any(|c| c.starts_with("tmux new-session")),
        "and no session is created over it"
    );
}
