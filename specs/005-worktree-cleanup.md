# Spec 005 — Clean up worktrees whose work is done

## Problem Statement

Every piece of work gets a worktree beside the checkout, and nothing takes them
away again. → can remove one, after you have decided it is finished — which
means remembering which of `api-VBSN-1` to `api-VBSN-9` merged, one menu at a
time. And a worktree whose directory was deleted by hand stays in git's list,
invisible to the picker, holding its branch.

## Solution

A project row's → menu gets **Clean up worktrees**, offered when the project
has worktrees. It prunes the entries whose directories are gone, then asks
about each worktree whose work is done, one at a time, through the same
`remove` the menu's own entry uses: refused with a session in it or a dirty
tree, confirmed defaulting to no, never forced, the branch left.

A worktree's work is done when either holds:

- **Merging its HEAD into the default branch would change nothing.**
  `git merge-tree --write-tree origin/<default> HEAD` gives the tree
  `origin/<default>` already has. That covers a merge, and a squash or rebase
  of exactly this work, which an ancestor check or `git cherry` would miss.
  The local default branch stands in when there is no remote one.
- **Its branch's remote is gone** — `[gone]` in `%(upstream:track)` — which is
  what a merged pull request usually leaves once a fetch has pruned.

Neither catches a branch whose own tip was superseded by a later version on its
remote that was then merged: that one is still removed by hand. A branch cut
and never committed to is merged by the first measure, which is why each
worktree is asked about rather than removed in a batch.

The worktrees are read from `git worktree list`, not the cache, so ones made
since the last refresh are included.

## Constitution

§1 holds: two more git calls behind the one `Host`. §3 holds: nothing is
fetched; the answer is as current as the last fetch. §4 holds: removal goes
through `remove`, which refuses a dirty tree up front and never forces git.
`merge-tree --write-tree` needs git 2.38; an older git fails it, and a failure
counts as not merged.

**Status:** done

- [x] Project menu offers Clean up worktrees when the project has worktrees
- [x] Missing directories pruned; merged or gone worktrees asked about one by one
- [x] A session in it or a dirty tree refuses, as removing by hand does
- [x] Nothing fetched
- [x] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` all pass
