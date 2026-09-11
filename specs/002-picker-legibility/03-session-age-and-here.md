# 03: What tmux knows: order, age, and which session is yours

**What to build:** Session rows say when you were last in them, and which one you
are in now.

`tmux list-sessions` gains one format field, `#{session_last_attached}`, at no
extra call. Sessions sort by it, most recent first; a session never attached
sorts last and shows no age. The age cell reads `now` under a minute, then `5m`,
`3h`, `12d`.

The session this client is attached to is marked `▶` rather than `●`, which is
the distinction `(attached)` could never draw: another terminal has this session
versus this is the session you are sitting in. It costs one call, `tmux
display-message -p '#{session_name}'`, made only when running inside tmux.

**Blocked by:** 02

- [ ] Sessions sort by when they were last attached, most recent first, ties keeping tmux's order
- [ ] A never-attached session sorts last and its age cell is empty
- [ ] Age renders `now`, `5m`, `3h`, `12d` across the boundaries
- [ ] Inside tmux, the current session is marked `▶` and every other attached session `●`
- [ ] Outside tmux, `display-message` is never called and no row is marked `▶`
