//! → on a row: the menu of what else can be done to it, what each entry emits,
//! and ← back out of it.

use vibestation::fake::{Answer, FakeHost};

const LIST: &str = "tmux list-sessions -F #{session_attached}\t#{session_last_attached}\t#{pane_current_path}\t#{pane_current_command}\t#{@note}\t#{session_name}";
const BRANCH: &str = "git rev-parse --abbrev-ref HEAD";
const STATUS: &str = "git status --porcelain";
const CONFIG: &str = "/home/dev/.vibestation/config.toml";
const CACHE: &str = "/home/dev/.vibestation/projects-cache.json";
const STATE: &str = "/home/dev/.vibestation/state.json";

/// One session, sitting in a subdirectory of `api`'s second worktree, and two
/// projects: `api` with two worktrees, found by the scan, and `tool`, added by
/// hand. The rows are:
///
/// 0 the session · 1 the separator · 2 `api` · 3 its first worktree ·
/// 4 its second, the busy one · 5 `tool` · 6 add a project
fn host() -> FakeHost {
    FakeHost::new()
        .file(
            CONFIG,
            "projects_dirs = [\"/home/dev/code\"]\n\
             extra_projects = [\"/home/dev/vendor/tool\"]\n\
             username = \"ada\"\n\
             scan_depth = 3\n",
        )
        .file(
            CACHE,
            r#"[
              {"name": "api", "path": "/home/dev/code/api",
               "worktrees": [
                 {"path": "/home/dev/code/api-VBSN-1", "branch": "ada/VBSN-1-init"},
                 {"path": "/home/dev/code/api-VBSN-2", "branch": "ada/VBSN-2-busy"}
               ]},
              {"name": "tool", "path": "/home/dev/vendor/tool", "worktrees": []}
            ]"#,
        )
        .file("/home/dev/code/api/.git/HEAD", "")
        .file("/home/dev/vendor/tool/.git/HEAD", "")
        .succeeds(
            LIST,
            "0\t0\t/home/dev/code/api-VBSN-2/src\tnvim\t\tada/VBSN-2-busy\n",
        )
        .succeeds(
            &format!("/home/dev/code/api-VBSN-2/src $ {BRANCH}"),
            "ada/VBSN-2-busy\n",
        )
        // What a rescan asks: every repository its own main checkout.
        .succeeds("git rev-parse --git-dir --git-common-dir", ".git\n.git\n")
        .succeeds("git worktree list --porcelain", "")
}

/// Run to the end of the scripted answers, however the tool ended.
fn drive(host: &FakeHost) {
    let _ = vibestation::run(host);
}

fn ran(host: &FakeHost, command: &str) -> bool {
    host.log().iter().any(|logged| logged == command)
}

#[test]
fn right_on_a_session_offers_it_and_left_goes_back_to_the_list() {
    let host = host().answers([Answer::Right(0), Answer::Left, Answer::Abort]);

    drive(&host);

    assert_eq!(
        host.prompts(),
        [
            "Open  1 running · 2 projects",
            "ada/VBSN-2-busy",
            "Open  1 running · 2 projects",
        ],
        "the menu is titled with the row it is for, and ← leaves it for the \
         list rather than for the shell"
    );
}

#[test]
fn a_session_can_be_joined_killed_or_renamed() {
    let host = host().answers([Answer::Right(0), Answer::Abort]);

    drive(&host);

    assert_eq!(host.options(), ["Join", "Kill", "Rename"]);
}

#[test]
fn join_from_the_menu_is_what_enter_on_the_row_does() {
    let host = host().answers([Answer::Right(0), Answer::Select(0)]);

    vibestation::run(&host).unwrap();

    assert_eq!(
        host.log().last().unwrap(),
        "tmux attach-session -t ada/VBSN-2-busy"
    );
}

#[test]
fn kill_asks_first_and_the_list_is_read_again_after() {
    let kept = host().answers([
        Answer::Right(0),
        Answer::Select(1),
        Answer::Confirm(false),
        Answer::Abort,
    ]);
    let killed = host()
        .succeeds("tmux kill-session -t ada/VBSN-2-busy", "")
        .answers([
            Answer::Right(0),
            Answer::Select(1),
            Answer::Confirm(true),
            Answer::Abort,
        ]);

    drive(&kept);
    drive(&killed);

    assert_eq!(kept.prompts()[2], "Kill ada/VBSN-2-busy? [y/N]");
    assert!(
        !kept.log().iter().any(|c| c.contains("kill-session")),
        "declined, so nothing was killed: {:?}",
        kept.log()
    );
    assert!(ran(&killed, "tmux kill-session -t ada/VBSN-2-busy"));
    assert_eq!(
        killed.log().iter().filter(|c| *c == LIST).count(),
        2,
        "the reopened picker lists what is running now: {:?}",
        killed.log()
    );
}

#[test]
fn rename_offers_the_name_to_edit_and_tmux_is_told_the_new_one() {
    let host = host()
        .succeeds("tmux rename-session -t ada/VBSN-2-busy ada/VBSN-2-v1-2", "")
        .answers([
            Answer::Right(0),
            Answer::Select(2),
            Answer::text("ada/VBSN-2-v1.2 "),
            Answer::Abort,
        ]);

    drive(&host);

    assert_eq!(host.prompts()[2], "Session name [ada/VBSN-2-busy]");
    assert!(
        ran(
            &host,
            "tmux rename-session -t ada/VBSN-2-busy ada/VBSN-2-v1-2"
        ),
        "trimmed, and the dot tmux forbids replaced: {:?}",
        host.log()
    );
}

#[test]
fn a_rename_tmux_refuses_leaves_the_picker_open() {
    let host = host()
        .fails(
            "tmux rename-session -t ada/VBSN-2-busy taken",
            1,
            "duplicate session: taken",
        )
        .answers([
            Answer::Right(0),
            Answer::Select(2),
            Answer::text("taken"),
            Answer::Abort,
        ]);

    drive(&host);

    assert_eq!(
        host.prompts().last().unwrap(),
        "Open  1 running · 2 projects"
    );
}

#[test]
fn tab_on_a_session_offers_its_note_to_edit_and_tmux_keeps_it() {
    let host = host()
        .succeeds(
            "tmux set-option -t ada/VBSN-2-busy @note fixing the arrows",
            "",
        )
        .answers([
            Answer::Tab(0),
            Answer::text("  fixing\tthe  arrows "),
            Answer::Abort,
        ]);

    drive(&host);

    assert_eq!(host.prompts()[1], "Note for ada/VBSN-2-busy []");
    assert!(
        ran(
            &host,
            "tmux set-option -t ada/VBSN-2-busy @note fixing the arrows"
        ),
        "one line, so it comes back as one field: {:?}",
        host.log()
    );
    assert_eq!(
        host.log().iter().filter(|logged| **logged == LIST).count(),
        2,
        "and the sessions are read again to show it"
    );
}

#[test]
fn an_emptied_note_is_taken_off_and_tab_elsewhere_does_nothing() {
    let host = host()
        .succeeds("tmux set-option -u -t ada/VBSN-2-busy @note", "")
        .answers([
            Answer::Tab(2),
            Answer::Tab(0),
            Answer::text(" "),
            Answer::Abort,
        ]);

    drive(&host);

    assert_eq!(
        host.prompts()[..3],
        [
            "Open  1 running · 2 projects",
            "Open  1 running · 2 projects",
            "Note for ada/VBSN-2-busy []",
        ],
        "Tab on a project reopens the list: only a session holds a note"
    );
    assert!(ran(&host, "tmux set-option -u -t ada/VBSN-2-busy @note"));
}

#[test]
fn the_row_under_the_cursor_is_previewed_in_full_with_its_note() {
    let host = host()
        .terminal(120, 40)
        .succeeds(
            LIST,
            "0\t0\t/home/dev/code/api-VBSN-2/src\tnvim\tfixing the arrows\tada/VBSN-2-busy\n",
        )
        .answer(Answer::Abort);

    drive(&host);
    let previews = host.previews();

    assert_eq!(
        previews[0],
        [
            "ada/VBSN-2-busy",
            "dir     ~/code/api-VBSN-2/src",
            "branch  ada/VBSN-2-busy",
            "running nvim",
            "",
            "→ Join · Kill · Rename",
            "",
            "Notes: fixing the arrows",
        ]
    );
    assert!(previews[1].is_empty(), "the separator has nothing to show");
    assert_eq!(
        previews[4][..4],
        [
            "api-VBSN-2",
            "dir     ~/code/api-VBSN-2",
            "branch  ada/VBSN-2-busy",
            "of      api",
        ]
    );
    assert!(
        host.options().iter().all(|row| row.chars().count() <= 78),
        "the grid gives the panel its third of the terminal: {:?}",
        host.options()
    );
}

#[test]
fn a_scanned_project_offers_a_session_or_the_editor() {
    let host = host()
        .editor("code -w")
        .answers([Answer::Right(2), Answer::Abort]);

    drive(&host);

    assert_eq!(host.prompts()[1], "api");
    assert_eq!(
        host.options(),
        ["New session", "Open in code", "Clean up worktrees"],
        "no way to take it off the list, since the next refresh would find it \
         again"
    );
}

#[test]
fn the_editor_opens_in_the_directory_and_counts_as_an_open() {
    let host = host()
        .editor("/usr/bin/nvim")
        .answers([Answer::Right(3), Answer::Select(1)]);

    vibestation::run(&host).unwrap();

    assert_eq!(
        host.log().last().unwrap(),
        "/home/dev/code/api-VBSN-1 $ /usr/bin/nvim ."
    );
    assert!(
        host.file_contents(STATE)
            .unwrap()
            .contains("\"/home/dev/code/api\""),
        "credited to the project, as a session in its worktree would be"
    );
}

#[test]
fn a_project_added_by_hand_can_be_taken_off_the_list() {
    let host = host().answers([Answer::Right(5), Answer::Select(2), Answer::Abort]);

    drive(&host);

    assert_eq!(host.prompts()[1], "tool");
    assert!(
        host.file_contents(CONFIG)
            .unwrap()
            .contains("extra_projects = []"),
        "{:?}",
        host.file_contents(CONFIG)
    );
    assert!(
        !host.options().iter().any(|row| row.contains("tool")),
        "and the rescan no longer finds it: {:?}",
        host.options()
    );
    assert!(
        host.file_contents("/home/dev/vendor/tool/.git/HEAD")
            .is_some(),
        "the repository itself is not touched"
    );
}

#[test]
fn a_clean_worktree_is_removed_after_asking_and_its_branch_is_left() {
    let host = host()
        .succeeds(&format!("/home/dev/code/api-VBSN-1 $ {STATUS}"), "")
        .succeeds(
            "/home/dev/code/api $ git worktree remove /home/dev/code/api-VBSN-1",
            "",
        )
        .answers([
            Answer::Right(3),
            Answer::Select(2),
            Answer::Confirm(true),
            Answer::Abort,
        ]);

    drive(&host);

    assert_eq!(
        host.prompts()[1],
        "api-VBSN-1",
        "titled as its row names it"
    );
    assert_eq!(host.prompts()[2], "Remove /home/dev/code/api-VBSN-1? [y/N]");
    assert!(
        ran(
            &host,
            "/home/dev/code/api $ git worktree remove /home/dev/code/api-VBSN-1"
        ),
        "never --force: {:?}",
        host.log()
    );
    assert!(
        !host.log().iter().any(|c| c.contains("branch -d")),
        "the branch keeps its commits"
    );
    assert!(
        host.writes()
            .iter()
            .any(|(path, _)| path.ends_with("projects-cache.json")),
        "the list is rescanned without it"
    );
}

#[test]
fn a_declined_removal_removes_nothing() {
    let host = host()
        .succeeds(&format!("/home/dev/code/api-VBSN-1 $ {STATUS}"), "")
        .answers([
            Answer::Right(3),
            Answer::Select(2),
            Answer::Confirm(false),
            Answer::Abort,
        ]);

    drive(&host);

    assert!(!host.log().iter().any(|c| c.contains("worktree remove")));
}

#[test]
fn a_dirty_worktree_is_not_removed_and_nothing_is_asked() {
    let host = host()
        .succeeds(
            &format!("/home/dev/code/api-VBSN-1 $ {STATUS}"),
            "?? notes.txt\n",
        )
        .answers([Answer::Right(3), Answer::Select(2), Answer::Abort]);

    drive(&host);

    assert!(!host.log().iter().any(|c| c.contains("worktree remove")));
    assert_eq!(
        host.prompts().last().unwrap(),
        "Open  1 running · 2 projects",
        "straight back to the picker, no confirmation offered"
    );
}

#[test]
fn a_worktree_with_a_session_in_it_is_not_removed() {
    let host = host().answers([Answer::Right(4), Answer::Select(2), Answer::Abort]);

    drive(&host);

    assert!(
        !host
            .log()
            .iter()
            .any(|c| c.contains("git status") || c.contains("worktree remove")),
        "refused before git is even asked: {:?}",
        host.log()
    );
}

/// `api` as git sees it now: the two cached worktrees and two made since the
/// last refresh, against an `origin/main` whose tree is `base`. The first is
/// merged, the second — the busy one — and the third have lost their remote
/// branches, and the fourth is neither.
fn cleanable() -> FakeHost {
    host()
        .file("/home/dev/code/api-VBSN-1/.git", "")
        .file("/home/dev/code/api-VBSN-2/.git", "")
        .file("/home/dev/code/api-VBSN-3/.git", "")
        .file("/home/dev/code/api-VBSN-4/.git", "")
        .succeeds(
            "/home/dev/code/api $ git worktree list --porcelain",
            "worktree /home/dev/code/api\nbranch refs/heads/main\n\n\
             worktree /home/dev/code/api-VBSN-1\nbranch refs/heads/ada/VBSN-1-init\n\n\
             worktree /home/dev/code/api-VBSN-2\nbranch refs/heads/ada/VBSN-2-busy\n\n\
             worktree /home/dev/code/api-VBSN-3\nbranch refs/heads/ada/VBSN-3-done\n\n\
             worktree /home/dev/code/api-VBSN-4\nbranch refs/heads/ada/VBSN-4-wip\n",
        )
        .succeeds("git worktree prune", "")
        .succeeds(
            "git symbolic-ref --short refs/remotes/origin/HEAD",
            "origin/main\n",
        )
        .succeeds(
            "git rev-parse --verify --quiet origin/main^{tree}",
            "base\n",
        )
        .succeeds(
            "git for-each-ref --format=%(refname:short)\t%(upstream:track) refs/heads",
            "ada/VBSN-1-init\t\nada/VBSN-2-busy\t[gone]\nada/VBSN-3-done\t[gone]\n\
             ada/VBSN-4-wip\t[ahead 2]\nmain\t\n",
        )
        .succeeds(
            "/home/dev/code/api-VBSN-1 $ git merge-tree --write-tree origin/main HEAD",
            "base\n",
        )
        .succeeds("git merge-tree --write-tree origin/main HEAD", "other\n")
}

#[test]
fn clean_up_asks_about_each_outdated_worktree_and_only_those() {
    let host = cleanable()
        .succeeds(STATUS, "")
        .succeeds(
            "/home/dev/code/api $ git worktree remove /home/dev/code/api-VBSN-1",
            "",
        )
        .answers([
            Answer::Right(2),
            Answer::Select(2),
            Answer::Confirm(true),
            Answer::Confirm(false),
            Answer::Abort,
        ]);

    drive(&host);

    assert!(ran(&host, "/home/dev/code/api $ git worktree prune"));
    assert_eq!(
        host.prompts()
            .iter()
            .filter(|prompt| prompt.starts_with("Remove"))
            .collect::<Vec<_>>(),
        [
            "Remove /home/dev/code/api-VBSN-1? [y/N]",
            "Remove /home/dev/code/api-VBSN-3? [y/N]"
        ],
        "the merged one and the one whose remote is gone; not the one with a \
         session in it, nor the one still being worked on"
    );
    assert!(ran(
        &host,
        "/home/dev/code/api $ git worktree remove /home/dev/code/api-VBSN-1"
    ));
    assert_eq!(
        host.log()
            .iter()
            .filter(|c| c.contains("worktree remove"))
            .count(),
        1,
        "declined, so the other stays: {:?}",
        host.log()
    );
}

#[test]
fn clean_up_with_nothing_outdated_asks_nothing() {
    let host = cleanable()
        .succeeds(
            "/home/dev/code/api-VBSN-1 $ git merge-tree --write-tree origin/main HEAD",
            "other\n",
        )
        .succeeds(
            "git for-each-ref --format=%(refname:short)\t%(upstream:track) refs/heads",
            "",
        )
        .answers([Answer::Right(2), Answer::Select(2), Answer::Abort]);

    drive(&host);

    assert!(!host.log().iter().any(|c| c.contains("worktree remove")));
    assert_eq!(
        host.prompts().last().unwrap(),
        "Open  1 running · 2 projects",
        "straight back to the picker"
    );
}

#[test]
fn right_on_a_row_with_nothing_more_to_offer_opens_nothing() {
    let separator = host().answers([Answer::Right(1), Answer::Abort]);
    let add = host().answers([Answer::Right(6), Answer::Abort]);

    for host in [&separator, &add] {
        drive(host);
        assert_eq!(
            host.prompts(),
            [
                "Open  1 running · 2 projects",
                "Open  1 running · 2 projects"
            ]
        );
    }
}
