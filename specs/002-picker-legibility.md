# Spec 002 — A picker you can read

## Problem Statement

v1's picker lists the right things. Reading it is the problem.

On a real machine — twenty-six projects, four live sessions — the list arrives
seven rows at a time, because inquire's `DEFAULT_PAGE_SIZE` is 7 and nothing
overrides it. A fifty-row terminal shows seven rows and hides forty-three. So the
list scrolls when it never needed to, and the moment it scrolls, the separator
between live sessions and available projects goes with it. Scrolled down, nothing
on screen says which kind of row you are looking at:

```
^ cycling-das                                          ~/projects/sports/cycling-das
    └ cycling-dps-2  tmp
  cycling-service                                      ~/projects/sports/cycling-service
>   └ cycling-prod-error-rate-0c4faa
  sports-web                                           ~/projects/sports/sports-web
```

Those rows are not a table either. Only the name column is padded, and sessions
and projects are padded to two different widths, so the two blocks do not line up
with each other and every column after the first is ragged:

```
> android3 | Ana input everywhere                ~/projects/sports/android-monorepo-3  antondudakov/ana_input_everywhere  claude
  android4 | Enhanced device connection phase 1  ~/projects/sports/android-monorepo-4  release-sports/v1.2  claude  (attached)
```

That row is 130 characters. inquire's renderer wraps at the terminal width, one
character at a time, so on an eighty-column terminal every session becomes two
lines and the list turns to mush. Nothing truncates, because nothing knows how
wide the terminal is.

And the rows say less than tmux knows. `(attached)` cannot tell *another terminal
has this session* from *this is the session you are sitting in* — the two cases a
developer most needs to tell apart. tmux records when each session was last
attached, which is the closest thing to "what was I doing"; v1 never asks for it,
and lists sessions in tmux's own order instead.

## Solution

The same picker: one list, one keystroke, no new screen to exit from. Five
changes, all of them inside the rows.

1. **The list fills the terminal.** Page size comes from the terminal height, so
   a tall terminal shows thirty rows and the separator stays on screen.
2. **One table.** Every column is padded to its width across the *whole* list, so
   sessions, projects and worktrees line up as one grid.
3. **Every row fits.** The row is budgeted against the terminal width: the path
   is elided from the left to its last segments, and the lowest-value columns
   drop out before anything wraps.
4. **Every row says what it is, on its own.** A glyph column carries the state —
   the session you are in, a session attached elsewhere, a session running
   detached, a project, a worktree — so a scrolled list is still legible. The
   trailing `(attached)` text goes away.
5. **Sessions carry their age, and the prompt carries the counts.** Sessions sort
   by when they were last attached and show it. The prompt line, which never
   scrolls, reads `Open  4 running · 26 projects`.

```
? Open  3 running · 26 projects
❯ ▶ android3         ~/…/android-monorepo-3  ana_input_everywhere  claude  2m
  ● android4         ~/…/android-monorepo-4  release-sports/v1.2   claude  1h
  ○ selfhost         ~/…/sport-selfhost      main                  claude  3d
  ──────────────────────────────────────────────────────────────────────────
    cycling-dps      ~/projects/sports/cycling-dps
    └ cycling-dps-2  tmp
    cycling-service  ~/projects/sports/cycling-service
    └ cycling-prod…  prod-error-rate
    sports-web       ~/projects/sports/sports-web
  ↻ refresh the project list
  ✚ add a project by path
[↑↓ move · type to filter · enter open · esc cancel]
```

## User Stories

### Reading a list that scrolled

A developer with twenty-six projects scrolls past the separator. Every visible
row still says what it is: a glyph marks the live sessions, a blank column marks
the projects, a corner marks the worktrees. They never have to scroll back up to
work out what they are looking at, and the prompt line above the list has been
telling them the counts the whole time.

### Finding the session you were in

A developer comes back after lunch to four live sessions. The one they were in is
marked, and the rest are ordered by when they were last attached, with `2m`,
`1h`, `3d` beside them. The one they want is the first row.

### A session attached somewhere else

A developer has a session open on their desktop and picks it up from a laptop.
The row shows attached-elsewhere, not attached-by-you, so they know the other
terminal exists before they switch into it.

### An eighty-column terminal

A developer runs vibestation in a narrow split. No row wraps. Paths elide from
the left, keeping the directory name that identifies them; the command column
goes before the branch column, and the branch before the name. The list is still
a grid, just a narrower one.

### A long worktree name

`cycling-prod-error-rate-0c4faa` does not widen the name column for every other
row: no column may claim more than its share of the terminal, so the long name
truncates and the grid holds.

## Implementation Decisions

### Terminal size behind the seam

The terminal's width and height are impure, so they go behind `Host` like
everything else (constitution §1):

```rust
/// The terminal's width and height in cells. A terminal that will not answer
/// is 80×24, which is also what the fake reports until a test says otherwise.
fn terminal(&self) -> (usize, usize);
```

`RealHost` answers from `crossterm::terminal::size()`, falling back to `(80, 24)`.
`FakeHost` answers from a field with a `.terminal(w, h)` builder, default
`(80, 24)`.

### Page size

`Select::with_page_size(height - 4)`, floored at 5. The four are the prompt line,
the help line and two rows of margin so the terminal does not scroll the prompt
off its own screen. inquire already caps the page at the number of options.

### The columns

`picker::rows` stops producing one pre-joined string per row and produces cells
instead, laid out in six columns:

| column | session | project | worktree | action |
|---|---|---|---|---|
| glyph | `▶` `●` `○` | blank | `└` | `↻` `✚` |
| name | session name | project name | worktree directory name | the action's text |
| dir | active pane's directory | project path | — | — |
| branch | branch of that directory | — | worktree branch | — |
| command | active pane's command | — | — | — |
| age | time since last attached | — | — | — |

Every column is padded to the widest cell in it across the whole list, one
gutter of two spaces between columns, and a row is trimmed of trailing padding.
Empty cells stay as padding rather than collapsing, which is what makes the grid
a grid — v1's `columns()` helper, which dropped empty fields to close the gap,
goes away.

The `username/` prefix is stripped from every branch cell, session and worktree
alike, when it matches the configured username. It is thirteen identical columns
of noise on every row of the machine in the problem statement.

The separator spans the fitted row width instead of a fixed twenty-four.

### Fitting the row

The budget is `width - 2`: inquire writes a one-cell prefix and a space before
every option (`ui/backend.rs`, `render_options`). Two passes:

**Caps, before padding.** No column may exceed its share of the budget: name and
dir a third each, branch a quarter, command twelve cells, age four. Over-long
cells truncate with `…`; dir truncates from the *left*, keeping whole path
segments where it can — `~/…/android-monorepo-3` — because the end of a path is
what identifies it.

**Shrink, until it fits.** While the padded row still exceeds the budget, in
order: elide dir harder (last two segments, then one), drop the command column,
drop the age column, truncate branch, truncate name. A priority list, not a
layout engine.

### Glyphs

`▶` the session this client is attached to, `●` a session attached by some other
client, `○` a live session with no client, blank for a project, `└` for a
worktree, `↻` and `✚` for the two actions. One cell each, and no colour — the
reason is in Further Notes.

### Session age and the current session

`tmux list-sessions` gains one format field, `#{session_last_attached}`, which
costs no extra call. Sessions sort by it, most recent first; a session never
attached (`0`) sorts last, and ties keep tmux's order, since the sort is stable.
Age renders as `now` under a minute, then `5m`, `3h`, `12d`; a session never
attached has an empty age cell.

The current session needs one more tmux call, `display-message -p
'#{session_name}'`, made only when `in_tmux()` is true. Outside tmux there is no
current session and the call is skipped.

### The prompt line and the help line

The select message becomes `Open  4 running · 26 projects`, counting session rows
and project rows — worktrees credit their project, as they do everywhere else.
With no sessions it is `Open  26 projects`. Both halves are singularised at one.

The help line is set globally once: `↑↓ move · type to filter · enter open · esc
cancel`, with `❯` as the highlighted-option prefix. Those are the only styling
changes, and they are uniform across every prompt the tool shows.

### The separator

Selecting the separator currently returns from `run` and exits the tool
silently (`lib.rs`, `Row::Separator => return Ok(())`). It reopens the picker
instead, like the two escape hatches — the row is decoration, and choosing
decoration should cost nothing.

### Dependencies

`crossterm 0.25` becomes a direct dependency. inquire 0.7 already pulls exactly
that version, so it is already compiled and already in `Cargo.lock`; naming it
adds a line to `Cargo.toml` and nothing to the build. Constitution §8 asks that a
dependency save more code than it costs to understand, and the alternative —
`ioctl(TIOCGWINSZ)` through `libc`, or shelling out to `tput` on the hot path —
is more code and more surface for one number.

## Testing Decisions

Row layout is a pure function of sessions, projects, home, username and terminal
width, so it is tested by driving the fake to the picker and asserting on the
exact rows, which is what `tests/picker.rs` already does. What is new:

- Columns line up across sessions, projects and worktrees in one list.
- A long worktree name truncates rather than widening the name column.
- At 80 columns, a row that was 130 characters fits in 78 and elides the path.
- At 60 columns, the command and age columns are gone and the grid still lines up.
- The glyph distinguishes current, attached-elsewhere, detached, project,
  worktree.
- `username/` is stripped from branch cells; a branch with another owner's prefix
  is left alone.
- Sessions sort by last-attached, most recent first; a never-attached session
  sorts last with an empty age.
- Age boundaries: `now`, `5m`, `3h`, `12d`.
- The prompt message carries the counts, singular at one, and omits the running
  half when there are no sessions.
- Page size follows the fake's terminal height, floored at 5.
- Outside tmux, `display-message` is never called; inside, it is called once.
- Choosing the separator reopens the picker and emits nothing.

Existing assertions in `tests/picker.rs` and `tests/session.rs` change, because
the rows they assert on change. That churn is the test suite doing its job.

## Out of Scope

**Per-row colour.** Feasible, but inquire computes line width by counting the
characters of the option string (`ui/frame_renderer.rs`), so ANSI escapes inflate
the width and wrap rows early; and its fuzzy scorer matches against the same
string, so escapes pollute matching. Both are surmountable — budget the phantom
width when truncating, and supply `with_scorer` matching clean text by index —
but structure beats colour on a hundred-column row, and this spec is structure.
Revisit once the grid is in and still does not read.

**A full-screen TUI.** ratatui buys exactly two things this spec cannot: a
preview pane and per-row keybindings. It costs a heavy dependency against §8, a
hand-rolled fuzzy matcher, hand-rolled scrolling and text editing, and it
dissolves the `select(message, options) -> index` contract that lets one fake
drive the whole application (§1). It also changes what the tool is: something
that is gone in 300ms, not an app you exit from.

**A preview pane** — `capture-pane` of the live session, `git status` of the
project. The only honest reason to go full-screen, and not yet a reason.

**Per-row keybindings** — killing a session from the picker, opening it in an
editor. inquire has no key hook, and the value of the picker is that one keypress
ends it.

**Dirty-tree markers on project rows.** A `git status` per project puts the disk
on the picker's hot path, against the reason the cache exists at all.

**Delegating to `fzf`.** A runtime dependency, a second UI to keep consistent,
and inquire's built-in skim matcher is why v1 deliberately does not need it.

## Further Notes

### Why per-row state beats a header

A header labels a group only while the header is on screen. In a list that
scrolls — and with twenty-six projects it always scrolls — the label has to live
on the row, or it is not a label. This is why the glyph column earns a cell of
every row, and why the separator stays as decoration rather than being promoted
into two labelled headers.

### Why the counts sit in the prompt

The prompt line and the help line are the only two lines inquire never scrolls.
The summary that answers "how much of this list is running work?" belongs on one
of them, and the prompt already exists to be read first.

### Why the width budget is a priority list

A real layout engine would distribute slack proportionally. A priority list —
elide, drop, truncate, in a fixed order — is four lines of code, produces the
same answer on every terminal a developer actually uses, and fails legibly: the
columns disappear in the order you would have deleted them yourself.
