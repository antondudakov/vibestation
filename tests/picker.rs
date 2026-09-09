//! The session picker end to end: what the rows say, what gets emitted when one
//! is chosen, and the two ways of joining a session.

use vibestation::fake::{Answer, FakeHost};
use vibestation::host::aborted;
use vibestation::picker;
use vibestation::tmux;

const LIST: &str = "tmux list-sessions -F #{session_attached}\t#{pane_current_path}\t#{pane_current_command}\t#{session_name}";
const BRANCH: &str = "git rev-parse --abbrev-ref HEAD";
const CONFIG: &str = "/home/dev/.vibestation/config.toml";

/// Two live sessions, one of them attached elsewhere, and a settled config so
/// the first-run prompt stays out of the way.
fn host() -> FakeHost {
    FakeHost::new()
        .file(
            CONFIG,
            "projects_dirs = [\"/home/dev/code\"]\nusername = \"ada\"\n",
        )
        .succeeds(
            LIST,
            "0\t/home/dev/code/vibestation\tnvim\tada/VBSN-4-picker\n\
             1\t/etc\tzsh\tnotes\n",
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

#[test]
fn rows_carry_directory_branch_command_and_the_attached_marker() {
    let host = host();
    let sessions = tmux::list(&host).unwrap();

    assert_eq!(
        picker::rows(&sessions, std::path::Path::new("/home/dev")),
        [
            "ada/VBSN-4-picker  ~/code/vibestation  ada/VBSN-4-picker  nvim",
            "notes              /etc  zsh  (attached)",
        ],
        "the home prefix collapses, a missing branch leaves no gap"
    );
}

#[test]
fn choosing_a_session_outside_tmux_attaches_to_it() {
    let host = host().answer(Answer::Select(1));

    vibestation::run(&host).unwrap();

    assert_eq!(host.prompts(), ["Session"]);
    assert_eq!(
        host.log().last().unwrap(),
        "tmux attach-session -t notes",
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
fn aborting_the_picker_emits_nothing_and_is_not_a_failure() {
    let host = host().answer(Answer::Abort);

    let error = vibestation::run(&host).unwrap_err();

    assert!(
        aborted(&error),
        "an abort is distinguishable from a failure"
    );
    assert!(
        !host
            .log()
            .iter()
            .any(|c| c.starts_with("tmux attach") || c.starts_with("tmux switch-client")),
        "nothing was joined: {:?}",
        host.log()
    );
}

#[test]
fn no_tmux_server_is_not_an_error_and_prompts_for_nothing() {
    let host = FakeHost::new()
        .file(CONFIG, "projects_dirs = [\"/home/dev/code\"]\n")
        .fails(LIST, 1, "no server running on /tmp/tmux-1000/default");

    vibestation::run(&host).unwrap();

    assert!(host.prompts().is_empty());
    assert_eq!(host.log(), [LIST]);
}
