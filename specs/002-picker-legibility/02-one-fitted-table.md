# 02: One table, fitted to the terminal

**What to build:** The picker's rows become a grid. `picker::rows` produces cells
rather than a pre-joined string — glyph, name, dir, branch, command, age — and
every column is padded to the widest cell in it *across the whole list*, so
sessions, projects and worktrees line up with each other. Empty cells stay as
padding; v1's `columns()` helper, which closed the gap and made every row ragged,
goes away. Trailing padding is trimmed.

The glyph column carries what a row is: `●` a session attached elsewhere, `○` a
live session with no client, blank a project, `└` a worktree, `↻` and `✚` the two
actions. The trailing `(attached)` text goes. (`▶`, the session you are in, needs
ticket 03.)

No row wraps. The budget is the terminal width less the two cells inquire writes
before every option. Columns are capped before padding — name and dir a third of
the budget each, branch a quarter, command twelve cells, age four — and over-long
cells truncate with `…`, dir from the left on whole path segments
(`~/…/android-monorepo-3`). If the padded row still overruns, in order: elide dir
harder, drop command, drop age, truncate branch, truncate name.

Two smaller things that belong to the same pass: the `username/` prefix is
stripped from branch cells when it matches the configured username, and the
separator spans the fitted width instead of a fixed twenty-four. Choosing the
separator reopens the picker rather than exiting the tool silently.

**Blocked by:** 01

**Status:** done

- [x] Columns line up across sessions, projects and worktrees in one list
- [x] A long worktree name truncates rather than widening the name column
- [x] A 130-character session row fits in 78 cells at 80 columns, eliding the path from the left
- [x] At 60 columns the command and age columns are gone and the grid still lines up
- [x] The glyph distinguishes attached-elsewhere, detached, project, worktree and the two actions
- [x] `username/` is stripped from branch cells; another owner's prefix is left alone
- [x] The separator spans the fitted width, and choosing it reopens the picker and emits nothing
