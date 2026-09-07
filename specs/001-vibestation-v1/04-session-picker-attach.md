# 04: Session picker, attach vs. switch-client

**What to build:** A working session switcher — the first genuinely useful
version of the tool. Running `vibestation` opens a fuzzy-filterable picker of
live sessions; typing narrows it, the keyboard selects, and selecting attaches.
Run from inside tmux it switches the current client rather than nesting tmux
inside tmux; run outside it attaches. Aborting the picker does nothing at all.

**Blocked by:** 03.

**Status:** ready-for-agent

- [ ] Running the bare command opens the picker over live sessions
- [ ] Rows are fuzzy-filterable by typing and selectable entirely by keyboard
- [ ] The embedded picker requires no external `fzf` installation
- [ ] Sessions with a client attached elsewhere are marked, but attaching is neither blocked nor confirmed
- [ ] Running inside tmux emits switch-client; running outside emits attach
- [ ] Aborting the picker exits without emitting any command
- [ ] With no tmux server, the picker opens rather than erroring
