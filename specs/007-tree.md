# Spec 007 — The picker is a tree

## Problem Statement

A worktree's row started where its project's did, with `└` in the glyph column
and nothing in the directory column, so it read as a sibling of the project
rather than a child, and where it lived was only in the panel. The sessions and
the projects were told apart by an unlabelled rule the cursor could land on.
And Enter on a worktree asked `Open ada/VBSN-1-init?` — a yes or no whose no
meant "something else", unsaid.

## Solution

The picker is drawn as a tree. Two blocks, each headed — `── sessions ──` and
`── projects ──` — when there are two; one needs no heading. A project's
worktrees are indented beneath it, `├─` and `└─` as `tree` draws them, each in
the directory column like every other row.

A heading is a row with nothing to preview: the list passes over it with the
arrows, a page, Home and End, never starts on one, and hides it once a filter
is typed. Nothing can choose it.

Enter on a worktree on a branch of its own, with no session in it, offers two
ways in: `Open ada/VBSN-1-init`, named for that branch, or `New session…`,
which asks everything a project asks — what you are working on, the name, the
branch.

The line a chosen row leaves behind is the row with the grid's padding taken
out.

## Constitution

§1 holds: no new seam. The fake chooses rows by index, a heading included,
which the real list never lets happen.

**Status:** done

- [x] Sessions and projects headed when both are listed; a heading is never chosen
- [x] Worktrees indented under their project, with their directory
- [x] Enter on a worktree offers its name or a new session
- [x] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` all pass

Not done: folding a project's worktrees away. It changes what a filter must
find — a ticket typed should still reach a folded worktree — so it waits for
someone to want it.
