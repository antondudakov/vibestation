# 07: Frecency ranking and the unified picker

**What to build:** The picker as the spec describes it. One list: live sessions
at the top, a separator, then projects ranked by how often and how recently the
developer has opened them, each followed by its worktrees as indented child rows.
The main checkout is visually distinguished from its worktrees and each worktree
row shows its branch. Every row is fuzzy-filterable, including worktree children,
so typing a ticket identifier reaches that worktree directly.

Ranking uses zoxide's frecency algorithm — the open count scaled by a decay
factor derived from the age of the last open, heavily favouring the last hour,
then the last day, then the last week, then everything older.

**Blocked by:** 04, 06.

**Status:** done

- [x] Frecency records (project path, open count, last-opened timestamp) round-trip through a JSON state file
- [x] A table of records with known counts and timestamps produces a known ordering, and the decay boundaries are exercised
- [x] Projects appear below the session separator, ranked by frecency
- [x] Each project's worktrees appear as indented child rows beneath it
- [x] The main checkout is visually distinguished from its worktrees; each worktree row shows its branch
- [x] Worktree child rows are fuzzy-filterable alongside every other row
- [x] A project that already has a live session appears only as its session, not twice
