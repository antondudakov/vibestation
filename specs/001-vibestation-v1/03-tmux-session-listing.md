# 03: tmux session listing

**What to build:** The developer can see what they were working on. The tool
reads their live tmux sessions and reports, for each one, its name, its working
directory, the git branch of its active pane, the command running in that pane,
and whether a client is already attached elsewhere. Having no tmux server
running is a normal cold start, not an error.

**Blocked by:** 01.

**Status:** ready-for-agent

- [ ] Sessions are read with a single `list-sessions` call using a format string emitting session name, attached-client count, and the active pane's current path and current command
- [ ] Only the active pane is inspected; other panes and windows are not enumerated
- [ ] Branch is resolved per session from the active pane's path via git, since tmux cannot supply it
- [ ] A session whose directory is not a git repository lists with an empty branch rather than a broken row
- [ ] A session with a client attached is marked
- [ ] No tmux server yields an empty session list, not a failure
- [ ] A running server with no sessions yields an empty list
- [ ] Sessions with unusual names parse correctly
