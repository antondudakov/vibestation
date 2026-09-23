//! Listing live tmux sessions, including the states that look like failures
//! but aren't: no server, no sessions, no git repository.

use std::path::PathBuf;
use vibestation::fake::FakeHost;
use vibestation::tmux::{self, Session};

const LIST: &str = "tmux list-sessions -F #{session_attached}\t#{session_last_attached}\t#{pane_current_path}\t#{pane_current_command}\t#{@note}\t#{session_name}";
const BRANCH: &str = "git rev-parse --abbrev-ref HEAD";

#[test]
fn one_call_yields_a_row_per_session_and_a_branch_read_per_path() {
    let host = FakeHost::new()
        .succeeds(
            LIST,
            "0\t0\t/home/dev/code/vibestation\tnvim\t\tada/VBSN-3-tmux\n\
             2\t0\t/home/dev/code/api\tcargo\tserving the api\tapi-server\n",
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
                current: false,
                branch: "ada/VBSN-3-tmux".to_string(),
                age: String::new(),
                note: String::new(),
            },
            Session {
                name: "api-server".to_string(),
                path: PathBuf::from("/home/dev/code/api"),
                command: "cargo".to_string(),
                attached: true,
                current: false,
                branch: "main".to_string(),
                age: String::new(),
                note: "serving the api".to_string(),
            },
        ],
        "an attached client is marked, a note is read, and tmux's ordering is kept"
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
        .succeeds(LIST, "1\t0\t/etc\tvim\t\tnotes\n")
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
            "0\t0\t/w\tzsh\t\tname with spaces\n\
             0\t0\t/w\tzsh\t\ttab\there\n\
             0\t0\t/w\tzsh\t\t\n\
             \n\
             0\t0\t/w\tzsh\t\tada/VBSN-9-fix\n",
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

/// The fake's clock, which every timestamp below is measured back from.
const NOW: u64 = 1_700_000_000;
const DISPLAY: &str = "tmux display-message -p #{session_name}";

#[test]
fn sessions_come_back_most_recently_left_first_and_say_how_long_ago() {
    let host = FakeHost::new()
        .succeeds(
            LIST,
            &format!(
                "0\t{}\t/w\tzsh\t\tthree-days\n\
                 0\t0\t/w\tzsh\t\tnever\n\
                 0\t{}\t/w\tzsh\t\tfive-minutes\n\
                 0\t{}\t/w\tzsh\t\ttwo-hours\n\
                 0\t{}\t/w\tzsh\t\tjust-now\n",
                NOW - 3 * 24 * 60 * 60,
                NOW - 5 * 60,
                NOW - 2 * 60 * 60,
                NOW - 10,
            ),
        )
        .succeeds(BRANCH, "main\n");

    let listed: Vec<(String, String)> = tmux::list(&host)
        .unwrap()
        .into_iter()
        .map(|s| (s.name, s.age))
        .collect();

    assert_eq!(
        listed,
        [
            ("just-now".to_string(), "now".to_string()),
            ("five-minutes".to_string(), "5m".to_string()),
            ("two-hours".to_string(), "2h".to_string()),
            ("three-days".to_string(), "3d".to_string()),
            ("never".to_string(), String::new()),
        ],
        "the one you were just in leads, and one no client ever had sorts last \
         with nothing to say"
    );
}

#[test]
fn the_session_you_are_in_is_told_apart_from_one_attached_elsewhere() {
    let host = FakeHost::new()
        .in_tmux(true)
        .succeeds(
            LIST,
            "1\t0\t/w\tzsh\t\there\n1\t0\t/w\tzsh\t\televsewhere\n",
        )
        .succeeds(DISPLAY, "here\n")
        .succeeds(BRANCH, "main\n");

    let sessions = tmux::list(&host).unwrap();

    assert!(
        sessions[0].current,
        "the client running vibestation is here"
    );
    assert!(
        sessions[1].attached && !sessions[1].current,
        "attached is true of a session left open in another terminal too"
    );
}

#[test]
fn outside_tmux_no_session_is_current_and_tmux_is_not_asked() {
    let host = FakeHost::new()
        .succeeds(LIST, "1\t0\t/w\tzsh\t\televsewhere\n")
        .succeeds(BRANCH, "main\n");

    assert!(!tmux::list(&host).unwrap()[0].current);
    assert!(
        !host
            .log()
            .iter()
            .any(|call| call.contains("display-message")),
        "there is no current session to ask about: {:?}",
        host.log()
    );
}
