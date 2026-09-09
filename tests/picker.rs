//! The one picker end to end: what the rows say, what a choice emits, and the
//! two ways of joining a session.

use vibestation::fake::{Answer, FakeHost};
use vibestation::host::aborted;

const LIST: &str = "tmux list-sessions -F #{session_attached}\t#{pane_current_path}\t#{pane_current_command}\t#{session_name}";
const BRANCH: &str = "git rev-parse --abbrev-ref HEAD";
const CONFIG: &str = "/home/dev/.vibestation/config.toml";
const CACHE: &str = "/home/dev/.vibestation/projects-cache.json";
const STATE: &str = "/home/dev/.vibestation/state.json";

/// Two live sessions, one of them attached elsewhere, and three cached
/// projects — one of which is the session's own directory, one carrying two
/// worktrees. A settled config and a warm cache keep the disk out of it.
fn host() -> FakeHost {
    FakeHost::new()
        .file(
            CONFIG,
            "projects_dirs = [\"/home/dev/code\"]\nusername = \"ada\"\n",
        )
        .file(
            CACHE,
            r#"[
              {"name": "vibestation", "path": "/home/dev/code/vibestation",
               "worktrees": [
                 {"path": "/home/dev/code/vibestation-VBSN-1", "branch": "ada/VBSN-1-init"},
                 {"path": "/home/dev/code/vibestation-VBSN-9", "branch": "ada/VBSN-9-fix"}
               ]},
              {"name": "api", "path": "/home/dev/code/api", "worktrees": []},
              {"name": "notes", "path": "/home/dev/notes", "worktrees": []}
            ]"#,
        )
        .succeeds(
            LIST,
            "0\t/home/dev/code/vibestation\tnvim\tada/VBSN-4-picker\n\
             1\t/etc\tzsh\tnotes-scratch\n",
        )
        .succeeds(
            &format!("/home/dev/code/vibestation $ {BRANCH}"),
            "ada/VBSN-4-picker\n",
        )
        .fails(
            &format!("/etc $ {BRANCH}"),
            128,
            "fatal: not a git repository",
        )
}

/// The picker's rows, by driving the tool and aborting at the prompt.
fn rows(host: &FakeHost) -> Vec<String> {
    vibestation::run(host).unwrap_err();
    host.options()
}

#[test]
fn sessions_come_first_then_projects_with_their_worktrees_beneath() {
    let host = host().answer(Answer::Abort);

    assert_eq!(
        rows(&host),
        [
            "ada/VBSN-4-picker  ~/code/vibestation  ada/VBSN-4-picker  nvim",
            "notes-scratch      /etc  zsh  (attached)",
            "────────────────────────",
            "  └ vibestation-VBSN-1  ada/VBSN-1-init",
            "  └ vibestation-VBSN-9  ada/VBSN-9-fix",
            "api    ~/code/api",
            "notes  ~/notes",
        ],
        "the project that already has a session is not listed twice, its \
         worktrees still stand where it would have been, and each worktree \
         row carries its branch"
    );
}

#[test]
fn projects_are_ranked_by_frecency() {
    let host = host()
        .file(
            STATE,
            r#"[{"path": "/home/dev/notes", "count": 3, "last": 1699999000}]"#,
        )
        .answer(Answer::Abort);

    let rows = rows(&host);

    assert!(rows[3].starts_with("notes"), "rows were {rows:?}");
    assert!(
        rows.last().unwrap().starts_with("api"),
        "the never-opened projects keep their scan order behind it: {rows:?}"
    );
}

#[test]
fn choosing_a_session_outside_tmux_attaches_to_it() {
    let host = host().answer(Answer::Select(1));

    vibestation::run(&host).unwrap();

    assert_eq!(host.prompts(), ["Open"]);
    assert_eq!(
        host.log().last().unwrap(),
        "tmux attach-session -t notes-scratch",
        "the name is taken from the chosen row, not its position"
    );
}

#[test]
fn choosing_a_session_inside_tmux_switches_the_client_instead() {
    let host = host().in_tmux(true).answer(Answer::Select(0));

    vibestation::run(&host).unwrap();

    assert_eq!(
        host.log().last().unwrap(),
        "tmux switch-client -t ada/VBSN-4-picker",
        "attaching inside tmux would nest a client in its own session"
    );
}

#[test]
fn the_separator_and_an_abort_both_emit_nothing() {
    let separator = host().answer(Answer::Select(2));
    let abort = host().answer(Answer::Abort);

    vibestation::run(&separator).unwrap();
    let error = vibestation::run(&abort).unwrap_err();

    assert!(
        aborted(&error),
        "an abort is distinguishable from a failure"
    );
    for host in [&separator, &abort] {
        assert!(
            !host
                .log()
                .iter()
                .any(|c| c.contains("attach-session") || c.contains("switch-client")),
            "nothing was joined: {:?}",
            host.log()
        );
    }
}

#[test]
fn no_tmux_server_leaves_a_picker_of_projects_alone() {
    let host = host()
        .fails(LIST, 1, "no server running on /tmp/tmux-1000/default")
        .answer(Answer::Abort);

    assert_eq!(
        rows(&host),
        [
            "vibestation  ~/code/vibestation",
            "  └ vibestation-VBSN-1  ada/VBSN-1-init",
            "  └ vibestation-VBSN-9  ada/VBSN-9-fix",
            "api          ~/code/api",
            "notes        ~/notes",
        ],
        "a cold start opens the picker rather than erroring, and with no \
         sessions above there is no separator"
    );
}
