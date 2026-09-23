# Spec 006 — Windows

## Problem Statement

Every prompt looked different. The picker and its menus were `select.rs`'s own
list, the text prompts `line.rs`'s line, and yes or no was still inquire's
confirm — three looks, and a `? message` line with the answer typed after it
that said nothing about what the keys did beyond a bracketed line of help.

Claude Code asks its questions in a window that does: a rule, the question,
numbered options with `❯` on the one under the cursor, a preview of that one
beside them, a rule, and the keys spelt out. The digits choose, the arrows
move, a key adds notes. That is the look wanted here.

## Solution

Every prompt is drawn in that window. `select.rs` owns every list — the
picker, a row's menu, the branch question, and now yes or no — through one
`Ask`, and draws it with a pure `frame` the tests read as text. `line.rs`'s
text prompt is the same window with a line to type on. Answered, a window
collapses to the `> message answer` line it always left, so the answers stack
up above the next question.

Two kinds of list. The picker filters as you type, on the same line and
scorer as before: digits are part of every ticket identifier. Everything else
is a handful of options, so it is numbered and a digit chooses; a letter moves
to the one option it starts, so `y` Enter at a yes or no answers as it did
under inquire.

### The panel

On a terminal 100 columns or wider, the picker gives a third of it, up to 64
columns, to a panel previewing the row under the cursor: the row in full where
the grid cut it, what → offers, and a session's note. The grid is fitted to
what is left, as it is fitted to any terminal. A row with nothing to preview —
the separator — shows none. Below 100 columns there is no panel, and the grid
keeps every column it had.

### Notes

Tab on a session offers its note to edit, and stores it as the session's tmux
user option `@note`. tmux keeps it for as long as the session lives: it
follows a rename and goes with a kill, with no file of ours to tell. It comes
back in the `list-sessions` call the picker already makes, one more field
before the name. A note is kept to one line, since it is one field of one
line, and an emptied one is unset. Tab on any other row reopens the list.

| key | in the picker | in a menu or question |
|---|---|---|
| Enter | the row's first action | that option |
| ↑ ↓ | move, wrapping | move, wrapping |
| typing | filters | a letter moves to its option |
| 1–9 | filters | chooses that option |
| Tab | a session's note | nothing |
| ← | refresh | back to the picker, in a row's menu |
| → | the row's menu | that option, in a row's menu |
| Esc | cancel | cancel |

## Constitution

§1 holds: `Host::pick` gains the previews and `Pick` gains Tab; the fake
records the previews so a test can read them. `confirm` goes through the same
list, so no new method. §2 holds: a note is `tmux set-option`, read through
`list-sessions`. §8 holds harder than before: inquire is gone, and crossterm
and fuzzy-matcher, already direct, are all that draws a prompt.

**Status:** done

- [x] Every list and text prompt drawn as one window: rules, question, keys
- [x] Menus, the branch question and yes or no numbered; a digit chooses
- [x] A letter moves to its option, so `y` and `n` still answer
- [x] The picker's panel previews the row under the cursor, 100 columns and up
- [x] Tab leaves a note on a session in tmux's `@note`; empty unsets it
- [x] inquire removed
- [x] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` all pass
