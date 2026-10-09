# Spec 008 — Fullscreen

## Problem Statement

Every prompt was drawn inline, where the cursor happened to be, and found its
way back to redraw by moving up a counted number of lines. The picker was laid
out once, for the width the terminal had when it opened. Resize it and the rows
wrapped, the count was wrong, and every redraw left a smeared copy behind. And
each answered prompt left a line behind it, so a few actions in, the picker
sat under a trail of them.

## Solution

vibestation takes the alternate screen, as vim and mc do, and gives it back as
it found it however the run ends: joining a session, opening the editor, Esc,
an error, a panic, Ctrl-C mid-fetch, a kill. Every frame is drawn whole at
absolute rows.

The picker fills the screen, its keys on the last line. A resize under it
hands its rows back to be laid out at the new size — `Pick::Resize` — and it
reopens with what was typed and the row it was on.

Every other prompt — a row's menu, the branch question, yes or no, a line of
text — is a box centred over the picker as last drawn, dimmed, with its keys
on the last line. A box's options fit any width as they are, so a resize just
draws it again. Once answered, the box goes and the dimmed picker stays while
the answer is acted on, the cursor on the last line where anything git or ssh
asks of the terminal will show.

Raw mode is held only while a prompt reads keys, as before, so between
prompts Ctrl-C and the tty behave as they always did.

## Constitution

§1 holds: the terminal is the real host's business; the fake answers
`Answer::Resize` for the one thing the application does about it. §8:
`signal-hook` becomes a direct dependency, for giving the terminal back on
Ctrl-C. It was already built, for crossterm, so nothing new is compiled.

**Status:** done

- [x] Alternate screen taken on the first prompt and given back on every way out
- [x] The picker fills the screen and is laid out again on a resize, keeping its place
- [x] Menus, questions and text prompts are boxes over the dimmed picker
- [x] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` all pass

Not done: the panel stacked under a narrow list, and key hints shortened on a
narrow terminal rather than cut. Both are layout inside `frame` now, with no
terminal mechanics left in the way.
