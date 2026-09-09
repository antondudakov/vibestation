# 08: Name suggestion, session creation and the resume path

**What to build:** Selecting a project starts work. If the repository is already
on a feature branch — or if an existing worktree row was selected — the developer
is dropped straight into a session named `username/<branch>` with no prompts at
all. If it is on the default branch, one prompt collects ticket and description
together, the way they'd say it out loud ("VBSN-1 initialize the project"), and
the generated name is offered as an editable default. Either way a plain shell
session is created in the right directory and attached to, and the open counts
towards the project's frecency.

**Blocked by:** 07.

**Status:** done

- [x] On a non-default branch the suggested name is `username/<current-branch>` and no prompt is shown
- [x] Selecting an existing worktree row creates a session with no prompts
- [x] On the default branch, a single prompt collects ticket and description together
- [x] Input leading with a ticket-shaped token (uppercase letters, hyphen, digits) has that token split off as the ticket; otherwise the whole input is the description and no ticket prefix is applied
- [x] The description is lowercased, reduced to alphanumerics and hyphens, hyphen runs collapsed, truncated to roughly 50 characters
- [x] The result is `username/TICKET-description`, or `username/description` with no ticket, presented as an editable default rather than applied silently
- [x] Characters tmux forbids in session names are replaced with hyphens; forward slashes are preserved
- [x] Sessions are created detached, rooted in the project or worktree directory, running the login shell, with no startup command or window layout
- [x] The tool switches the current client if inside tmux and attaches otherwise
- [x] The open bumps both count and timestamp in the state file; a worktree selection credits its parent project
