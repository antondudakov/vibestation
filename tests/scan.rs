//! Discovering repositories and caching them: what the walk keeps, what it
//! drops, and what lands in the cache file.

use std::path::PathBuf;
use vibestation::config::Config;
use vibestation::fake::FakeHost;
use vibestation::scan::{self, Project};

const CACHE: &str = "/home/dev/.vibestation/projects-cache.json";

/// Two roots holding a same-named repository each, a vendored repository below
/// one of them, and a repository past the depth cap.
fn host() -> FakeHost {
    FakeHost::new()
        .file("/home/dev/code/vibestation/.git/HEAD", "")
        .file("/home/dev/code/api/.git/HEAD", "")
        .file("/home/dev/code/api/vendor/parser/.git/HEAD", "")
        .file("/home/dev/code/notes/README.md", "not a repo")
        .file("/home/dev/code/a/b/c/d/buried/.git/HEAD", "")
        .file("/home/dev/work/api/.git/HEAD", "")
        // Grouping asks git about every repository it found; here they are all
        // plain main checkouts with no worktrees. Worktrees are tests/group.rs.
        .succeeds("git rev-parse --git-dir --git-common-dir", ".git\n.git\n")
        .succeeds("git worktree list --porcelain", "")
}

fn config() -> Config {
    Config {
        projects_dirs: vec![
            PathBuf::from("/home/dev/code"),
            PathBuf::from("/home/dev/work"),
        ],
        scan_depth: 3,
        ..Config::default()
    }
}

fn found(projects: &[Project]) -> Vec<(&str, &str)> {
    projects
        .iter()
        .map(|p| (p.name.as_str(), p.path.to_str().unwrap()))
        .collect()
}

#[test]
fn every_root_is_scanned_and_nested_or_buried_repositories_are_left_out() {
    let host = host();

    let projects = scan::scan(&host, &config()).unwrap();

    assert_eq!(
        found(&projects),
        [
            ("code/api", "/home/dev/code/api"),
            ("vibestation", "/home/dev/code/vibestation"),
            ("work/api", "/home/dev/work/api"),
        ],
        "the vendored repository stops at its parent's .git, the one four \
         levels down is past the depth cap, and only the colliding name \
         carries its parent"
    );
    assert!(
        host.log()
            .iter()
            .all(|command| command.contains("git rev-parse") || command.contains("git worktree")),
        "scanning reads the disk and asks git about what it found; nothing \
         reaches the network: {:?}",
        host.log()
    );
}

#[test]
fn the_depth_cap_is_the_configured_one() {
    let deeper = Config {
        scan_depth: 6,
        ..config()
    };

    let projects = scan::scan(&host(), &deeper).unwrap();

    assert!(found(&projects).contains(&("buried", "/home/dev/code/a/b/c/d/buried")));
}

#[test]
fn extra_projects_are_merged_in_and_deduplicated() {
    let config = Config {
        extra_projects: vec![
            PathBuf::from("/opt/vendor/tool"),
            PathBuf::from("/home/dev/code/api"),
        ],
        ..config()
    };

    let projects = scan::scan(&host(), &config).unwrap();

    assert_eq!(
        found(&projects),
        [
            ("code/api", "/home/dev/code/api"),
            ("vibestation", "/home/dev/code/vibestation"),
            ("work/api", "/home/dev/work/api"),
            ("tool", "/opt/vendor/tool"),
        ],
        "the one already discovered appears once"
    );
}

#[test]
fn the_first_scan_writes_the_cache_and_the_next_load_reads_it() {
    let host = host();

    let scanned = scan::load_or_scan(&host, &config()).unwrap();

    assert_eq!(
        host.writes().iter().map(|(p, _)| p).collect::<Vec<_>>(),
        [&PathBuf::from(CACHE)]
    );

    // A host with the cache but none of the repositories on disk: anything
    // returned now came from the file rather than a second walk.
    let cached = FakeHost::new().file(CACHE, &host.file_contents(CACHE).unwrap());

    assert_eq!(scan::load_or_scan(&cached, &config()).unwrap(), scanned);
    assert!(cached.writes().is_empty(), "the cache is not rewritten");
}

#[test]
fn an_unreadable_cache_is_rescanned_rather_than_fatal() {
    let host = host().file(CACHE, "{ this is not json");

    let projects = scan::load_or_scan(&host, &config()).unwrap();

    assert_eq!(projects.len(), 3);
    assert_eq!(host.writes().len(), 1, "the cache is replaced");
}
