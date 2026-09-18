//! The one picker end to end: what the rows say, how they line up, how they
//! fit the terminal, what a choice emits, and the two ways of joining a
//! session.

use vibestation::fake::{Answer, FakeHost};
use vibestation::host::aborted;

const LIST: &str = "tmux list-sessions -F #{session_attached}\t#{session_last_attached}\t#{pane_current_path}\t#{pane_current_command}\t#{session_name}";
const BRANCH: &str = "git rev-parse --abbrev-ref HEAD";
const DISPLAY: &str = "tmux display-message -p #{session_name}";
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
            "0\t0\t/home/dev/code/vibestation\tnvim\tada/VBSN-4-picker\n\
             1\t0\t/etc\tzsh\tnotes-scratch\n",
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
            "○  ada/VBSN-4-picker   ~/code/vibestation  VBSN-4-picker  nvim",
            "●  notes-scratch       /etc                               zsh",
            "──────────────────────────────────────────────────────────────",
            "   vibestation         ~/code/vibestation",
            "└  vibestation-VBSN-1                      VBSN-1-init",
            "└  vibestation-VBSN-9                      VBSN-9-fix",
            "   api                 ~/code/api",
            "   notes               ~/notes",
            "↻  refresh the project list",
            "✚  add a project by path",
        ],
        "a project with a session of its own is still listed below it — the \
         session row resumes that work, the project row starts new work in the \
         same repository — each worktree row carries its branch in the same \
         column as the sessions above, and the two actions come last"
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

    assert!(
        rows[3].trim_start().starts_with("notes"),
        "rows were {rows:?}"
    );
    assert!(
        rows[rows.len() - 3].trim_start().starts_with("api"),
        "the never-opened projects keep their scan order behind it, above the \
         two action rows: {rows:?}"
    );
}

#[test]
fn choosing_a_session_outside_tmux_attaches_to_it() {
    let host = host().answer(Answer::Select(1));

    vibestation::run(&host).unwrap();

    assert_eq!(
        host.prompts(),
        ["Open  2 running · 3 projects"],
        "the prompt line never scrolls, so it carries what the list is made of"
    );
    assert_eq!(
        host.log().last().unwrap(),
        "tmux attach-session -t notes-scratch",
        "the name is taken from the chosen row, not its position"
    );
}

#[test]
fn choosing_a_session_inside_tmux_switches_the_client_instead() {
    let host = host()
        .in_tmux(true)
        .succeeds(DISPLAY, "ada/VBSN-4-picker\n")
        .answer(Answer::Select(0));

    vibestation::run(&host).unwrap();

    assert_eq!(
        host.log().last().unwrap(),
        "tmux switch-client -t ada/VBSN-4-picker",
        "attaching inside tmux would nest a client in its own session"
    );
}

#[test]
fn the_separator_reopens_the_picker_and_an_abort_leaves_it() {
    let separator = host().answers([Answer::Select(2), Answer::Abort]);
    let abort = host().answer(Answer::Abort);

    vibestation::run(&separator).unwrap_err();
    let error = vibestation::run(&abort).unwrap_err();

    assert_eq!(
        separator.prompts(),
        [
            "Open  2 running · 3 projects",
            "Open  2 running · 3 projects"
        ],
        "choosing decoration costs nothing: the same picker comes back"
    );

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
            "   vibestation         ~/code/vibestation",
            "└  vibestation-VBSN-1                      VBSN-1-init",
            "└  vibestation-VBSN-9                      VBSN-9-fix",
            "   api                 ~/code/api",
            "   notes               ~/notes",
            "↻  refresh the project list",
            "✚  add a project by path",
        ],
        "a cold start opens the picker rather than erroring, and with no \
         sessions above there is no separator"
    );
}

/// Where `needle` starts, counted in cells: a glyph is one column and three
/// bytes, so byte offsets do not compare across rows.
fn column(row: &str, needle: &str) -> usize {
    row[..row.find(needle).expect(needle)].chars().count()
}

#[test]
fn columns_line_up_across_sessions_projects_and_worktrees() {
    let host = host().answer(Answer::Abort);

    let rows = rows(&host);

    assert_eq!(
        column(&rows[0], "~/code/vibestation"),
        column(&rows[6], "~/code/api"),
        "a session's directory and a project's start in the same column: {rows:?}"
    );
    assert_eq!(
        column(&rows[0], "VBSN-4-picker  nvim"),
        column(&rows[4], "VBSN-1-init"),
        "and so do a session's branch and a worktree's: {rows:?}"
    );
    assert_eq!(
        rows[2].chars().count(),
        rows[0].chars().count().max(rows[3].chars().count()),
        "the separator spans the grid it separates: {rows:?}"
    );
}

/// Everything long at once: a deep path, an owner-prefixed branch, a command,
/// and a session name past its share of the width. This is the row that used
/// to wrap into two.
fn long() -> FakeHost {
    FakeHost::new()
        .file(
            CONFIG,
            "projects_dirs = [\"/home/dev/projects\"]\nusername = \"antondudakov\"\n",
        )
        .file(
            CACHE,
            r#"[{"name": "android-monorepo-3",
                 "path": "/home/dev/projects/sports/android-monorepo-3",
                 "worktrees": []}]"#,
        )
        .succeeds(
            LIST,
            "0\t0\t/home/dev/projects/sports/android-monorepo-3\tclaude\tandroid3 | Ana input everywhere\n",
        )
        .succeeds(
            "/home/dev/projects/sports/android-monorepo-3 $ git rev-parse --abbrev-ref HEAD",
            "antondudakov/ana_input_everywhere\n",
        )
}

#[test]
fn a_row_that_used_to_wrap_now_fits_the_terminal() {
    let host = long().answer(Answer::Abort);

    let rows = rows(&host);

    for row in &rows {
        assert!(
            row.chars().count() <= 78,
            "80 columns less the two inquire writes before every option: {row:?}"
        );
    }
    assert_eq!(rows[0].chars().count(), 78, "and it uses what it is given");
}

#[test]
fn a_narrow_terminal_gives_up_the_command_before_it_destroys_the_path() {
    let host = long().terminal(60, 24).answer(Answer::Abort);

    let rows = rows(&host);

    for row in &rows {
        assert!(row.chars().count() <= 58, "{row:?}");
    }
    assert!(
        !rows[0].contains("claude"),
        "the command goes before the branch or the name: {rows:?}"
    );
}

#[test]
fn your_own_prefix_comes_off_a_branch_and_another_owners_stays() {
    let host = host()
        .succeeds(
            LIST,
            "0\t0\t/home/dev/code/vibestation\tnvim\tmine\n1\t0\t/home/dev/notes\tzsh\ttheirs\n",
        )
        .succeeds(
            &format!("/home/dev/notes $ {BRANCH}"),
            "grace/VBSN-2-review\n",
        )
        .answer(Answer::Abort);

    let rows = rows(&host);

    assert!(
        rows[0].contains("VBSN-4-picker") && !rows[0].contains("ada/"),
        "your own name on every row is your own name, repeated: {rows:?}"
    );
    assert!(
        rows[1].contains("grace/VBSN-2-review"),
        "someone else's prefix is information: {rows:?}"
    );
}

#[test]
fn the_glyph_says_what_each_row_is() {
    let host = host().answer(Answer::Abort);

    let glyphs: Vec<char> = rows(&host)
        .iter()
        .map(|row| row.chars().next().unwrap())
        .collect();

    assert_eq!(
        glyphs,
        ['○', '●', '─', ' ', '└', '└', ' ', ' ', '↻', '✚'],
        "detached, attached elsewhere, the separator, a project with its two \
         worktrees, two more projects, and the two escape hatches — readable \
         at any scroll position"
    );
}

#[test]
fn one_long_worktree_name_does_not_widen_the_name_column() {
    let host = host()
        .file(
            CACHE,
            r#"[{"name": "vibestation", "path": "/home/dev/code/vibestation",
                 "worktrees": [
                   {"path": "/home/dev/code/vibestation-VBSN-7-prod-error-rate-0c4faa-again",
                    "branch": "ada/VBSN-7-prod"}
                 ]},
                {"name": "api", "path": "/home/dev/code/api", "worktrees": []}]"#,
        )
        .answer(Answer::Abort);

    let rows = rows(&host);

    assert!(
        rows[4].contains('…'),
        "the long name is the one that gives way: {rows:?}"
    );
    assert_eq!(
        column(&rows[5], "~/code/api"),
        column(&rows[0], "~/code/vibestation"),
        "and the column it is in stays where it was: {rows:?}"
    );
}

#[test]
fn the_session_you_are_in_leads_the_list_and_says_how_long_it_has_been() {
    let host = host()
        .in_tmux(true)
        .succeeds(
            LIST,
            "1\t1699996400\t/home/dev/code/vibestation\tnvim\tada/VBSN-4-picker\n\
             1\t0\t/etc\tzsh\tnotes-scratch\n",
        )
        .succeeds(
            "tmux display-message -p #{session_name}",
            "ada/VBSN-4-picker\n",
        )
        .answer(Answer::Abort);

    let rows = rows(&host);

    assert!(
        rows[0].starts_with('▶') && rows[0].ends_with("1h"),
        "the session you are in, an hour since you were last in it: {rows:?}"
    );
    assert!(
        rows[1].starts_with('●'),
        "and the one someone left attached elsewhere: {rows:?}"
    );
}

#[test]
fn the_prompt_line_counts_what_the_list_is_made_of() {
    let sessions = host().answer(Answer::Abort);
    let cold = host()
        .fails(LIST, 1, "no server running on /tmp/tmux-1000/default")
        .answer(Answer::Abort);
    let one = FakeHost::new()
        .file(
            CONFIG,
            "projects_dirs = [\"/home/dev/code\"]\nusername = \"ada\"\n",
        )
        .file(
            CACHE,
            r#"[{"name": "api", "path": "/home/dev/code/api", "worktrees": []}]"#,
        )
        .fails(LIST, 1, "no server running")
        .answer(Answer::Abort);

    for host in [&sessions, &cold, &one] {
        vibestation::run(host).unwrap_err();
    }

    assert_eq!(sessions.prompts(), ["Open  2 running · 3 projects"]);
    assert_eq!(
        cold.prompts(),
        ["Open  3 projects"],
        "with nothing running the half that would say so is left out"
    );
    assert_eq!(one.prompts(), ["Open  1 project"], "singular at one");
}
