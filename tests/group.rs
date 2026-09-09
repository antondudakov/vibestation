//! Three sibling directories that are one project: what git says about them
//! decides the grouping, never their names.

use std::path::PathBuf;
use vibestation::config::Config;
use vibestation::fake::FakeHost;
use vibestation::group::Worktree;
use vibestation::scan;

const REV_PARSE: &str = "git rev-parse --git-dir --git-common-dir";
const LIST: &str = "git worktree list --porcelain";

fn config() -> Config {
    Config {
        projects_dirs: vec![PathBuf::from("/home/dev/code")],
        scan_depth: 3,
        ..Config::default()
    }
}

fn worktree(path: &str, branch: &str) -> Worktree {
    Worktree {
        path: PathBuf::from(path),
        branch: branch.to_string(),
    }
}

/// A main checkout with two worktrees — one scanned, one living in `/tmp`
/// where the scan never looks — plus a worktree whose main checkout is outside
/// the roots entirely, and an entry git still lists for a directory that has
/// been deleted.
fn host() -> FakeHost {
    FakeHost::new()
        .file("/home/dev/code/api/.git/HEAD", "")
        .file("/home/dev/code/api-VBSN-1/.git", "gitdir: ...")
        .file("/home/dev/code/orphan-wt/.git", "gitdir: ...")
        .file("/tmp/hotfix/.git", "gitdir: ...")
        .succeeds(&format!("/home/dev/code/api $ {REV_PARSE}"), ".git\n.git\n")
        .succeeds(
            &format!("/home/dev/code/api-VBSN-1 $ {REV_PARSE}"),
            "/home/dev/code/api/.git/worktrees/api-VBSN-1\n/home/dev/code/api/.git\n",
        )
        .succeeds(
            &format!("/home/dev/code/orphan-wt $ {REV_PARSE}"),
            "/elsewhere/proj/.git/worktrees/orphan\n/elsewhere/proj/.git\n",
        )
        .succeeds(
            &format!("/home/dev/code/api $ {LIST}"),
            "worktree /home/dev/code/api\nHEAD abc\nbranch refs/heads/main\n\
             \n\
             worktree /home/dev/code/api-VBSN-1\nHEAD def\nbranch refs/heads/ada/VBSN-1-init\n\
             \n\
             worktree /tmp/hotfix\nHEAD 123\ndetached\n\
             \n\
             worktree /home/dev/code/gone\nHEAD 456\nbranch refs/heads/ada/VBSN-2-gone\n",
        )
}

#[test]
fn siblings_collapse_into_one_project_with_git_as_the_authority() {
    let projects = scan::scan(&host(), &config()).unwrap();

    let names: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names,
        ["api", "orphan-wt"],
        "the scanned worktree is absorbed; the one whose main checkout is \
         outside the roots stands alone rather than vanishing"
    );
    assert_eq!(
        projects[0].worktrees,
        [
            worktree("/home/dev/code/api-VBSN-1", "ada/VBSN-1-init"),
            worktree("/tmp/hotfix", ""),
        ],
        "a worktree outside the scanned roots still lists, a detached one \
         lists without a branch, the deleted one is dropped, and the main \
         checkout is not a worktree of itself"
    );
    assert!(projects[1].worktrees.is_empty());
}

#[test]
fn grouping_asks_git_locally_and_only_about_what_it_found() {
    let host = host();

    scan::scan(&host, &config()).unwrap();

    assert_eq!(
        host.log(),
        [
            format!("/home/dev/code/api $ {REV_PARSE}"),
            format!("/home/dev/code/api-VBSN-1 $ {REV_PARSE}"),
            format!("/home/dev/code/orphan-wt $ {REV_PARSE}"),
            format!("/home/dev/code/api $ {LIST}"),
        ],
        "one rev-parse per repository and one worktree list per main checkout \
         — the standalone worktree has no main here to enumerate from — and \
         nothing that reaches the network"
    );
}

#[test]
fn a_path_git_will_not_answer_for_still_lists() {
    let host = host()
        .file("/opt/not-a-repo/README.md", "")
        .fails(REV_PARSE, 128, "fatal: not a git repository")
        .fails(LIST, 128, "fatal: not a git repository");
    let config = Config {
        extra_projects: vec![PathBuf::from("/opt/not-a-repo")],
        ..config()
    };

    let projects = scan::scan(&host, &config).unwrap();

    assert_eq!(projects.last().unwrap().name, "not-a-repo");
    assert!(projects.last().unwrap().worktrees.is_empty());
}

#[test]
fn the_grouped_shape_is_what_lands_in_the_cache() {
    let host = host();

    scan::load_or_scan(&host, &config()).unwrap();

    let (path, cache) = host.writes().into_iter().next().unwrap();
    assert_eq!(
        path,
        PathBuf::from("/home/dev/.vibestation/projects-cache.json")
    );
    assert!(
        cache.contains("\"ada/VBSN-1-init\""),
        "worktrees are cached with their branches, so a picker open needs no \
         git at all: {cache}"
    );
}
