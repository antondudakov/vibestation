//! The picker's two escape hatches: refresh, which is ←, and the add-manually
//! row, driven through the picker the way a developer reaches them.

use vibestation::fake::{Answer, FakeHost};

const LIST: &str = "tmux list-sessions -F #{session_attached}\t#{session_last_attached}\t#{pane_current_path}\t#{pane_current_command}\t#{session_name}";
const CONFIG: &str = "/home/dev/.vibestation/config.toml";
const CACHE: &str = "/home/dev/.vibestation/projects-cache.json";

/// A settled config with every field spelled out, a cache that predates a
/// freshly cloned repository, and a repository outside the configured root.
/// No tmux server, so the picker is projects and the add row alone.
fn host() -> FakeHost {
    FakeHost::new()
        .file(
            CONFIG,
            "projects_dirs = [\"/home/dev/code\"]\n\
             extra_projects = []\n\
             username = \"ada\"\n\
             default_branch = \"develop\"\n\
             scan_depth = 3\n\
             fetch_before_branch = false\n",
        )
        .file(
            CACHE,
            r#"[{"name": "api", "path": "/home/dev/code/api", "worktrees": []}]"#,
        )
        .file("/home/dev/code/api/.git/HEAD", "")
        .file("/home/dev/code/fresh/.git/HEAD", "just cloned")
        .file("/home/dev/vendor/tool/.git/HEAD", "")
        .file("/home/dev/notes/README.md", "not a repo")
        .fails(LIST, 1, "no server running on /tmp/tmux-1000/default")
        .succeeds("git rev-parse --git-dir --git-common-dir", ".git\n.git\n")
        .succeeds("git worktree list --porcelain", "")
}

/// The rows the picker is left showing, by aborting at its last prompt.
fn rows(host: &FakeHost) -> Vec<String> {
    vibestation::run(host).unwrap_err();
    host.options()
}

#[test]
fn adding_is_the_last_row_and_refreshing_is_a_key() {
    let host = host().answer(Answer::Abort);

    assert_eq!(rows(&host), ["api  ~/code/api", "✚  add a project by path"]);
}

#[test]
fn left_rescans_the_roots_and_rewrites_the_cache() {
    let host = host().answers([Answer::Left, Answer::Abort]);

    assert_eq!(
        rows(&host),
        [
            "api    ~/code/api",
            "fresh  ~/code/fresh",
            "✚  add a project by path",
        ],
        "the picker reopens over what the rescan found"
    );
    assert!(
        host.file_contents(CACHE).unwrap().contains("/code/fresh"),
        "the cache was rewritten, so the next run starts warm: {:?}",
        host.file_contents(CACHE)
    );
}

#[test]
fn add_manually_refuses_a_path_that_is_not_a_repository() {
    let host = host().answers([
        Answer::Select(1),
        Answer::text("/home/dev/notes"),
        Answer::Abort,
    ]);

    let rows = rows(&host);

    assert_eq!(host.prompts()[1], "Path to the repository []");
    assert!(
        !rows.iter().any(|row| row.contains("notes")),
        "the picker reopens without it: {rows:?}"
    );
    assert!(
        !host
            .writes()
            .iter()
            .any(|(path, _)| path.ends_with("config.toml")),
        "and nothing was written to the config: {:?}",
        host.writes()
    );
}

#[test]
fn an_added_repository_lands_in_the_config_and_survives_a_refresh() {
    let host = host().answers([
        Answer::Select(1),
        Answer::text("~/vendor/tool"),
        // A refresh after the addition is what proves it is not merely in
        // memory.
        Answer::Left,
        Answer::Abort,
    ]);

    let rows = rows(&host);

    assert_eq!(
        host.file_contents(CONFIG).unwrap(),
        "# vibestation configuration. Delete any field to get its default back.\n\
         \n\
         # Directories scanned for git repositories.\n\
         projects_dirs = [\"/home/dev/code\"]\n\
         \n\
         # Repositories added by hand, or through the picker's add-manually action.\n\
         extra_projects = [\"/home/dev/vendor/tool\"]\n\
         \n\
         # Prefix for generated session and branch names: username/TICKET-description.\n\
         username = \"ada\"\n\
         \n\
         # The branch new branches are cut from. Left out, it is detected per\n\
         # repository: origin/HEAD, then main, then master.\n\
         default_branch = \"develop\"\n\
         \n\
         # How deep below each projects directory to look for repositories.\n\
         scan_depth = 3\n\
         \n\
         # The pre-selected answer to \"fetch before branching?\".\n\
         fetch_before_branch = false\n",
        "the typed ~ is resolved and every other setting is still there"
    );
    assert_eq!(
        rows,
        [
            "api    ~/code/api",
            "fresh  ~/code/fresh",
            "tool   ~/vendor/tool",
            "✚  add a project by path",
        ],
        "the added project is still listed after the rescan"
    );
}

#[test]
fn a_refresh_says_how_far_it_has_got_and_clears_the_line_after() {
    let host = host().answers([Answer::Left, Answer::Abort]);

    rows(&host);

    assert_eq!(
        host.statuses(),
        [
            "scanning /home/dev/code…",
            "▕██████████░░░░░░░░░░▏ 1/2 repositories",
            "▕████████████████████▏ 2/2 repositories",
            "",
        ],
        "the walk has no count, so it is named; the git pass has one, so it is \
         a bar; and the line is gone before the picker comes back"
    );
}
