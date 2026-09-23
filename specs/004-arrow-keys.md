# Spec 004 — ← and → in the picker

## Problem Statement

Everything the picker could do besides joining and starting work lived
somewhere else. Refreshing the list meant scrolling to its second-to-last row.
Killing a session, renaming one, removing a worktree you had finished with or
opening a project in your editor meant leaving the tool for a shell and typing
the tmux or git command yourself.

The two keys that would carry this — ← for the list, → for the row — were
spoken for. inquire's select hands every key it does not bind to its filter
line, from a hardcoded `match` with no hook, so ← and → could only ever move a
cursor through the three characters you had typed. Spec 003 met the same wall
in the text prompt and declined to own the picker, calling it "a different spec
and a much worse trade". This is that spec.

## Solution

The picker gets its own list, in `src/select.rs`, on the same shape as
`src/line.rs`: one pure `apply` holding every binding, and a thin loop that
reads keys and redraws. The filter line *is* `line.rs`'s line, so it edits with
the same keys as every other prompt. The scorer is inquire's own — skim's fuzzy
matcher, case-insensitive — named directly now, at the version inquire already
pulls. Ties keep list order, which inquire's unstable sort did not promise.

`Host::select` keeps its signature and uses the new list with the arrows
unbound, so the branch prompt behaves as it did. The picker and its menus use
a new `Host::pick`, which returns which of three keys chose:

| key | in the picker | in a row's menu |
|---|---|---|
| Enter | the row's first action | that action |
| → | the row's menu | that action |
| ← | refresh | back to the picker |
| Esc | cancel | cancel |

← needs no row, so it works with a filter that matches nothing.

### The menus

Each row's actions, the first being what Enter already did:

| row | actions |
|---|---|
| session | Join · Kill · Rename |
| project | New session · Open in *editor* · Remove from the list (added by hand only) |
| worktree | New session · Open in *editor* · Remove the worktree |
| separator, add row | none: → does nothing |

- **Kill** asks first, defaulting to no, then re-reads the session list.
- **Rename** offers the current name to edit, sanitised as every session name
  is. A name tmux refuses — one already taken — is reported and the picker
  comes back.
- **Open in *editor*** runs `$VISUAL`, else `$EDITOR`, else `vi`, on `.` from
  inside the directory, replacing vibestation as an attach does. It counts as
  an open for the frecency ranking.
- **Remove from the list** is offered only for a project in `extra_projects`:
  a scanned one would be found again by the next refresh. The repository is not
  touched.
- **Remove the worktree** refuses when a session is sitting in it, refuses when
  the tree is dirty — untracked files included — and otherwise asks, defaulting
  to no. `git worktree remove` is never forced, and the branch stays.

### Refresh

The ↻ row is gone; ← is refresh. The picker is cleared, and a status line
takes its place: `scanning ~/code…` while the directories are walked — one call
with no count until it returns — then a bar while git is asked about each
repository, `▕█████░░░░░▏ 23/41 repositories`. The line is cleared and the
picker redrawn in the same place. To make that pass one bar, `group` now asks
git everything about a repository before moving to the next, rather than
sweeping the list twice.

The status line goes through a new `Host::status`, so the fake records it and a
test asserts the bar.

## Constitution

§1 holds: `pick`, `status` and `editor` are methods on the one `Host`, and
`exec` gains the working directory `run` already had. No new trait. §4 holds:
removing a worktree refuses a dirty tree up front and never forces git. §8:
`fuzzy-matcher` becomes a direct dependency at the version inquire already
builds, so it adds nothing to the build.

**Status:** done

- [x] ← refreshes, with a status line and a bar, from the picker
- [x] → opens a row's menu; ← in it goes back, Esc cancels
- [x] Session: join, kill (confirmed), rename (sanitised; a refusal is reported)
- [x] Project and worktree: new session, open in the editor from the directory
- [x] Project added by hand: remove from the list, config rewritten
- [x] Worktree: remove, refused with a session in it or a dirty tree, confirmed
- [x] `Host::select` unchanged in behaviour for the branch prompt
- [x] Every line of the list is cut to the terminal, so none wraps
- [x] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` all pass
