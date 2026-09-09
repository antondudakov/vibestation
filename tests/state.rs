//! Frecency: what the state file holds, and the order it produces.

use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};
use vibestation::fake::FakeHost;
use vibestation::scan::Project;
use vibestation::state::{self, Open};

const STATE: &str = "/home/dev/.vibestation/state.json";
const NOW: u64 = 1_700_000_000;
const HOUR: u64 = 60 * 60;
const DAY: u64 = 24 * HOUR;
const WEEK: u64 = 7 * DAY;

fn open(path: &str, count: u32, ago: u64) -> Open {
    Open {
        path: PathBuf::from(path),
        count,
        last: NOW - ago,
    }
}

fn project(path: &str) -> Project {
    Project {
        name: path.rsplit('/').next().unwrap().to_string(),
        path: PathBuf::from(path),
        worktrees: Vec::new(),
    }
}

fn order(opens: &[Open], paths: &[&str]) -> Vec<String> {
    let projects = paths.iter().map(|path| project(path)).collect();
    state::rank(projects, opens, UNIX_EPOCH + Duration::from_secs(NOW))
        .into_iter()
        .map(|project| project.name)
        .collect()
}

#[test]
fn an_open_is_recorded_and_read_back() {
    let host = FakeHost::new();

    state::record(&host, &PathBuf::from("/home/dev/code/api")).unwrap();
    state::record(&host, &PathBuf::from("/home/dev/code/api")).unwrap();
    state::record(&host, &PathBuf::from("/home/dev/code/notes")).unwrap();

    assert_eq!(
        state::load(&host).unwrap(),
        [
            open("/home/dev/code/api", 2, 0),
            open("/home/dev/code/notes", 1, 0),
        ],
        "a second open bumps the count rather than adding a record"
    );
    assert_eq!(host.writes().last().unwrap().0, PathBuf::from(STATE));
}

#[test]
fn a_missing_or_broken_state_file_is_simply_no_history() {
    let broken = FakeHost::new().file(STATE, "}{");

    assert!(state::load(&FakeHost::new()).unwrap().is_empty());
    assert!(state::load(&broken).unwrap().is_empty());
}

#[test]
fn recency_outweighs_a_higher_count_across_each_decay_boundary() {
    let opens = [
        open("/p/hour", 1, HOUR - 1),
        open("/p/day", 3, DAY - 1),
        open("/p/week", 20, WEEK - 1),
        open("/p/older", 30, WEEK + 1),
    ];

    assert_eq!(
        order(&opens, &["/p/older", "/p/week", "/p/day", "/p/hour"]),
        ["week", "older", "day", "hour"],
        "scores are 10, 7.5, 6 and 4: twenty opens last week still beat one \
         in the last hour, but thirty older ones do not beat those twenty"
    );
}

#[test]
fn projects_never_opened_keep_the_order_they_were_found_in() {
    let opens = [open("/p/api", 1, DAY)];

    assert_eq!(
        order(&opens, &["/p/notes", "/p/api", "/p/tools"]),
        ["api", "notes", "tools"],
        "the one with history rises; the rest keep their scan order"
    );
}
