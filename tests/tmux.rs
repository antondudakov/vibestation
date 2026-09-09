//! Listing live tmux sessions, including the states that look like failures
//! but aren't: no server, no sessions, no git repository.

use std::path::PathBuf;
use vibestation::fake::FakeHost;
use vibestation::tmux::{self, Session};

const LIST: &str = "tmux list-sessions -F #{session_attached}\t#{pane_current_path}\t#{pane_current_command}\t#{session_name}";
const BRANCH: &str = "git rev-parse --abbrev-ref HEAD";

#[test]
fn one_call_yields_a_row_per_session_and_a_branch_read_per_path() {
    let host = FakeHost::new()
        .succeeds(
            LIST,
            "0\t/home/dev/code/vibestation\tnvim\tada/VBSN-3-tmux\n\
             2\t/home/dev/code/api\tcargo\tapi-server\n",
        )
        .succeeds(
            &format!("/home/dev/code/vibestation $ {BRANCH}"),
            "ada/VBSN-3-tmux\n",
        )
        .succeeds(&format!("/home/dev/code/api $ {BRANCH}"), "main\n");

    let sessions = tmux::list(&host).unwrap();

    assert_eq!(
        sessions,
        [
            Session {
                name: "ada/VBSN-3-tmux".to_string(),
                path: PathBuf::from("/home/dev/code/vibestation"),
                command: "nvim".to_string(),
                attached: false,
                branch: "ada/VBSN-3-tmux".to_string(),
            },
            Session {
                name: "api-server".to_string(),
                path: PathBuf::from("/home/dev/code/api"),
                command: "cargo".to_string(),
                attached: true,
                branch: "main".to_string(),
            },
        ],
        "an attached client is marked, and tmux's ordering is kept"
    );
    assert_eq!(
        host.log(),
        [
            LIST.to_string(),
            format!("/home/dev/code/vibestation $ {BRANCH}"),
            format!("/home/dev/code/api $ {BRANCH}"),
        ],
        "one list-sessions call; no window or pane enumeration"
    );
}

#[test]
fn a_session_outside_a_repository_lists_with_an_empty_branch() {
    let host = FakeHost::new()
        .succeeds(LIST, "1\t/etc\tvim\tnotes\n")
        .fails(BRANCH, 128, "fatal: not a git repository");

    assert_eq!(tmux::list(&host).unwrap()[0].branch, "");
}

#[test]
fn no_server_and_no_sessions_are_both_an_empty_list() {
    let no_server = FakeHost::new().fails(LIST, 1, "no server running on /tmp/tmux-1000/default");
    let no_sessions = FakeHost::new().succeeds(LIST, "");

    assert!(tmux::list(&no_server).unwrap().is_empty());
    assert!(tmux::list(&no_sessions).unwrap().is_empty());
}

#[test]
fn unusual_session_names_parse() {
    let host = FakeHost::new()
        .succeeds(
            LIST,
            "0\t/w\tzsh\tname with spaces\n\
             0\t/w\tzsh\ttab\there\n\
             0\t/w\tzsh\t\n\
             \n\
             0\t/w\tzsh\tada/VBSN-9-fix\n",
        )
        .succeeds(BRANCH, "main\n");

    let names: Vec<String> = tmux::list(&host)
        .unwrap()
        .into_iter()
        .map(|s| s.name)
        .collect();

    assert_eq!(
        names,
        ["name with spaces", "tab\there", "ada/VBSN-9-fix"],
        "separators and slashes survive; a nameless or blank row is dropped"
    );
}
