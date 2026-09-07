# 06: Worktree grouping

**What to build:** Three sibling directories on disk that are really one project
in three states stop reading as three unrelated projects. Because worktrees are
created as ordinary sibling directories, the scanner finds them as ordinary
repositories and the relationship has to be reconstructed from git rather than
inferred from directory names. This ticket does that reconstruction and writes
the grouped shape into the cache; ticket 07 displays it.

Grouping keys off the git common directory, never off the `<project>-<branch>`
directory naming convention — a worktree created by hand under any other name
must still group correctly.

**Blocked by:** 05.

**Status:** ready-for-agent

- [ ] For each discovered repository the git directory and the git common directory are resolved; repositories sharing a common directory form one group
- [ ] The group's main checkout is the one where git dir and common dir are equal
- [ ] Once the main checkout is known, `git worktree list` against it provides the authoritative worktree set — deliberately a superset of what the scan found
- [ ] A worktree living outside the configured projects directories is still listed under its project
- [ ] A worktree entry whose directory no longer exists on disk is dropped, not shown as a broken row
- [ ] A discovered worktree whose main checkout lies outside the configured roots is listed as a standalone project rather than hidden
- [ ] Grouping happens at scan time and is written into the cache, so the cost is paid on refresh rather than on every picker open
- [ ] Grouping performs no network access
