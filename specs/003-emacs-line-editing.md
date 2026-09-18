# Spec 003 — Text prompts you can edit

## Problem Statement

Every text prompt in vibestation arrives with an answer already in it. "Session
name" is pre-filled with the name the tool derived, the first-run question is
pre-filled with the projects directory it guessed, and both are offered
precisely so you can change them. Changing them is the problem.

inquire's text input binds arrows, Home, End, Ctrl-arrows, Backspace and
Delete — and nothing else. `InputAction::from_key` is a hardcoded match with no
hook, in 0.7.5 and in 0.9.4 alike, so there is no configuration that fixes
this and no version to upgrade to. A developer who has typed `C-a` for thirty
years gets a literal `a` at the cursor, or nothing at all.

The keys that matter here are the ones for editing a line you did not type:
jump to an end, kill to an end, kill the word you disagree with. Those are
exactly the ones missing.

## Solution

`Host::input` stops going through inquire and gets its own line editor, in
`src/line.rs`, built on crossterm — already a direct dependency, and the one
inquire itself reads keys with. It is one function around one pure key handler:
the terminal loop is thin enough to read, and the bindings are a `match` with a
unit test.

`select` and `confirm` keep inquire. The picker's filter line is where you type
three characters to narrow a list, not where you edit prose, and its `C-n`/`C-p`
already work; giving it emacs keys means owning the list, the paging and the
fuzzy scoring too, which is a different spec and a much worse trade.

### The bindings

Readline's, plus everything inquire already bound, so nothing regresses:

| key | does |
|---|---|
| `C-a` / Home | to the start of the line |
| `C-e` / End | to the end |
| `C-b` / `C-f` / ←→ | one character |
| `M-b` / `M-f` / `C-←` / `C-→` | one word |
| Backspace / `C-h` | delete the character to the left |
| `C-d` / Delete | delete the character to the right |
| `C-w` / `M-Backspace` | kill the word to the left |
| `M-d` | kill the word to the right |
| `C-k` | kill to the end of the line |
| `C-u` | kill to the start of the line |
| Enter | submit |
| Esc / `C-c` / `C-g` / `C-d` on an empty line | cancel |

A word is a run of alphanumerics, which is what inquire's own word motion means
by it — the filter line and the text prompt should not disagree about where a
word starts.

Esc cancels, as it does at every other prompt in this tool and as the picker's
help line promises. That costs the Meta-as-Esc-prefix spelling of `M-b`: a
terminal that sends Alt as an ESC prefix lands on whatever crossterm makes of
it. The Control bindings are the ones that carry the feature, and they are
unambiguous.

No kill ring, so no `C-y`. No `C-t`. Nothing has a history to search.

### What it looks like

Unchanged: `? <message> <answer>` while editing, `> <message> <answer>` once
answered, the prefix in light green unless `NO_COLOR` is set — inquire's own
default render config, which the picker still uses two lines away.

## Constitution

§1 holds: the seam is `Host::input`, whose signature does not change, and
`line::edit` is a free function inside the one production `Host`. `FakeHost` is
untouched and every existing test passes unchanged. §8 holds harder than
before: this removes a use of inquire rather than adding a dependency.

**Status:** done

- [x] `C-a`, `C-e`, `C-b`, `C-f` move by end and character
- [x] `M-b`, `M-f` move by word, over a line with punctuation in it
- [x] `C-w` and `M-d` kill a word each way; `C-k` and `C-u` kill to each end
- [x] `C-d` deletes forward, and cancels on an empty line
- [x] Backspace, Delete, arrows, Home and End still do what they did
- [x] Esc, `C-c` and `C-g` cancel, returning `Aborted` as inquire did
- [x] Enter submits what is on the line, including nothing
- [x] The prompt renders `?` while editing and `>` once answered
- [x] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` all pass
