# 01: Terminal size behind the seam, and a list that fills it

**What to build:** The picker stops showing seven rows. `Host` gains
`terminal() -> (usize, usize)` — width and height in cells — answered by
`crossterm::terminal::size()` in `RealHost`, falling back to `80×24`, and by a
`.terminal(w, h)` builder on `FakeHost` defaulting to the same. `crossterm 0.25`
becomes a direct dependency: inquire 0.7 already pulls that exact version, so it
costs a line in `Cargo.toml` and nothing in the build.

`Host::select` sets `with_page_size(height - 4)`, floored at 5 — the prompt line,
the help line and two rows of margin. The same seam sets the frame once: `❯` as
the highlighted-option prefix and the help line
`↑↓ move · type to filter · enter open · esc cancel`.

**Blocked by:** —

**Status:** done

- [x] `Host::terminal` returns the real terminal size, and `80×24` when the terminal will not answer
- [x] `FakeHost::terminal(w, h)` scripts it, defaulting to `80×24`
- [x] The picker's page size follows the terminal height, floored at 5
- [x] The highlighted-option prefix and the help line are set once, for every prompt the tool shows
