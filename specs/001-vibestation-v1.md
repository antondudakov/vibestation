# Spec 001 — Vibestation v1

## Problem Statement

A developer working across many git repositories loses time on the mechanics of
getting *into* work rather than doing it.

Each morning, or each context switch, they have to remember which tmux sessions
they left running and what was in them — `tmux ls` gives them names and nothing
else, so a list of six sessions says nothing about which directory each is in,
which branch it's on, or which one has the dev server. Reconstructing that means
attaching to each in turn.

When the session they want doesn't exist, the cost is higher. They have to
remember where the project lives on disk, `cd` there, decide on a branch name
that matches their team's convention, create the branch off an up-to-date default
branch, create a tmux session, and name it something they'll recognise later. It's
half a dozen commands, every one of them muscle memory that breaks the moment
they're on an unfamiliar machine — and the branch-naming step is the one most
often skipped or done inconsistently.

Existing tools solve fragments. `tmux ls` lists sessions without context. Session
switchers attach to what exists but don't know about projects. Project launchers
open a directory but know nothing about branches or naming conventions. Nothing
connects "which repos do I have" to "what should this branch be called" to "put
me in a tmux session for it".

## Solution

`vibestation` — a single command that is the only entry point to a developer's
work.

Running it opens one fuzzy-filterable picker. At the top are the developer's live
tmux sessions, each row showing the session name, its working directory, the git
branch of its active pane, the command currently running there, and a marker if a
client is already attached elsewhere. Below a separator are all git repositories
discovered under the developer's configured projects directories, ranked by how
often and how recently they've been opened.

Selecting a session attaches to it — switching the current client if vibestation
was run from inside tmux, attaching if it was run outside.

Selecting a project starts new work. If the repository is already on a feature
branch, vibestation suggests `username/that-branch` as the session name and gets
out of the way. If it's on the default branch, it asks one question — ticket and
description — and builds `username/TICKET-kebab-description` from the answer. The
developer can edit the suggestion. Vibestation then offers to create that branch
from the local default branch (defaulting to yes, since the developer has just
signalled new work), creates a detached tmux session rooted in the project
directory, and attaches to it.

The picker also carries two escape hatches: a refresh entry that rescans the
projects directories, and an add-manually entry for a repository that lives
outside them.

The result: one command, a keyboard, and the developer is working — with a
correctly named branch and a session they'll recognise tomorrow.

## User Stories

### Launching and listing sessions

1. As a developer, I want to run a single command `vibestation`, so that I can get to my work without remembering tmux syntax.
2. As a developer, I want to see all my current tmux sessions listed, so that I can see what I was working on at a glance.
3. As a developer, I want each session row to show its working directory, so that I can tell which project it belongs to.
4. As a developer, I want each session row to show the git branch of its active pane, so that I can distinguish two sessions on the same project.
5. As a developer, I want each session row to show the command running in its active pane, so that I can spot the session that has my dev server in it.
6. As a developer, I want sessions that already have a client attached to be marked, so that I know I'm about to join a session I have open elsewhere.
7. As a developer, I want to attach to a marked session anyway without an extra confirmation, so that opening the same session in a second terminal isn't obstructed.
8. As a developer, I want to filter the list by typing, so that I can find a session without scrolling.
9. As a developer, I want to select entirely with the keyboard, so that I never have to reach for the mouse.
10. As a developer running vibestation from inside tmux, I want it to switch my current client to the chosen session, so that I don't nest tmux inside tmux.
11. As a developer running vibestation outside tmux, I want it to attach to the chosen session, so that one command gets me in.
12. As a developer with no tmux server running, I want the picker to open normally showing just my projects, so that a cold start isn't an error.
13. As a developer, I want a session whose directory is not a git repository to still list cleanly with an empty branch column, so that non-repo sessions aren't broken rows.
14. As a developer, I want to abort the picker without selecting anything, so that opening it by accident costs nothing.

### Discovering projects

15. As a developer, I want my projects listed in the same picker below my sessions, so that I can start new work without running a second command.
16. As a developer, I want projects ranked by how often and how recently I've opened them, so that my current work is at the top.
17. As a developer, I want the project list to come from directories I configure, so that the tool finds my repos wherever I keep them.
18. As a developer, I want the tool to find git repositories nested several levels deep, so that a grouped folder layout still works.
19. As a developer, I want the scan to stop descending at the first `.git` it finds, so that submodules and vendored repositories don't pollute the list.
20. As a developer, I want symlinks skipped during the scan, so that the scan can't loop or wander outside my projects directories.
21. As a developer, I want the discovered project list cached, so that opening the picker is instant rather than rescanning my disk.
22. As a developer who just cloned a new repository, I want a "refresh" entry in the picker, so that I can pick up new projects without editing config or clearing a cache by hand.
23. As a developer whose project still isn't listed after refreshing, I want an "add manually" entry, so that a repository outside my projects directories is still reachable.
24. As a developer, I want a manually added path rejected if it isn't a git repository, so that I don't add a broken entry.
25. As a developer, I want manually added projects written to my config file, so that they persist across refreshes and I can see and edit them.
26. As a developer with two same-named directories in different roots, I want the parent directory shown to disambiguate them, so that I can tell which is which.
27. As a developer with unambiguous project names, I want just the bare directory name shown, so that the list stays readable.
28. As a developer, I want to configure more than one projects directory, so that work and personal repositories can both be listed.
29. As a developer, I want a project that already has a session to appear only as its session, so that the list doesn't show the same work twice.

### First run and configuration

30. As a new user, I want to be asked for my projects directory on first run, so that the tool works immediately without me reading documentation first.
31. As a new user, I want a config file written for me with commented defaults, so that I can discover the other settings later.
32. As a new user, I want my username defaulted from my git config, so that suggested session names are correct without me typing it.
33. As a developer, I want my config in a plain TOML file at a predictable path, so that I can edit it in an editor or keep it in my dotfiles.
34. As a developer, I want a malformed config file to produce a clear error naming the file, so that I can fix it rather than guess.

### Naming and creating sessions

35. As a developer opening a project that has no session, I want a session name suggested for me, so that I don't have to invent one under pressure.
36. As a developer whose repository is already on a feature branch, I want the suggestion to be `username/<that-branch>` with no questions asked, so that resuming existing work is a single keypress.
37. As a developer whose repository is on the default branch, I want to be asked for a ticket and description, so that the session name matches the work I'm about to start.
38. As a developer, I want the ticket and description asked in a single prompt, so that starting work isn't a questionnaire.
39. As a developer, I want a leading ticket identifier recognised automatically, so that I can type "VBSN-1 initialize the project" the way I'd say it.
40. As a developer with no ticket number, I want to type just a description, so that unticketed work isn't blocked.
41. As a developer, I want my description turned into a lowercase kebab-case slug, so that the resulting branch name is valid and readable.
42. As a developer, I want an overly long description truncated, so that the session name stays scannable in the picker.
43. As a developer, I want to edit the suggested name before it's used, so that I can override the generated one when it's wrong.
44. As a developer, I want characters tmux forbids in session names replaced automatically, so that session creation never fails on a name I was offered.
45. As a developer, I want the new session created with its working directory set to the project directory, so that my shell starts where the work is.
46. As a developer, I want to be dropped into the new session immediately, so that creating and attaching are one step.
47. As a developer, I want opening a project to count towards its frecency, so that the ranking reflects my actual habits over time.
48. As a developer, I want a plain shell in the new session with no preset layout, so that the tool doesn't impose a workflow on me.

### Creating a branch

49. As a developer starting ticketed work, I want to be offered a new branch created from the default branch, so that I don't start work on main by accident.
50. As a developer, I want that offer to default to yes, so that the common case is a single keypress.
51. As a developer resuming an existing feature branch, I want no branch prompt at all, so that resuming stays frictionless.
52. As a developer, I want the repository's default branch detected automatically, so that repositories still using `master` work without configuration.
53. As a developer, I want to override the default branch in config, so that a repository using `develop` works too.
54. As a developer with uncommitted changes, I want branch creation skipped with a clear warning, so that my in-progress work is never disturbed.
55. As a developer whose branch creation was skipped, I want the tmux session created anyway on the current branch, so that a dirty tree doesn't block me from working entirely.
56. As a developer, I want branching to use my local default branch with no network fetch, so that the tool stays fast and works offline.
57. As a developer, I want to decline the branch offer and still get my session, so that saying no is not the same as cancelling.

### Installing and contributing

58. As a macOS developer, I want to install via Homebrew, so that installing is one command.
59. As a Linux developer, I want a prebuilt binary attached to the GitHub release, so that I don't need a Rust toolchain to try it.
60. As a user, I want `--help` to explain what the tool does and what its flags are, so that I can orient myself without the README.
61. As a user, I want `--version` to report the installed version, so that I can file a useful bug report.
62. As a contributor, I want CI running formatting, lints and tests on every PR, so that I know a change is safe to merge.

## Implementation Decisions

### Language, platform, distribution

- **Rust**, single static binary, built from scratch rather than wrapping or forking an existing session manager (`sesh`, `tmux-sessionizer`). The branch-naming workflow is the tool's reason to exist and threading it through another tool's config and hook model costs more than the session-listing plumbing it would save.
- **macOS and Linux only.** No Windows accommodation in code. `$HOME` is read from the environment directly rather than taking a directory-resolution dependency.
- **Distribution:** GitHub Releases with prebuilt macOS and Linux binaries, plus a Homebrew tap. Public open-source from the first commit.

### The seam

There is exactly one seam: a **`Host` trait** capturing every impure interaction the application has with the outside world. Application logic is a pure function over it.

The trait covers:

- **Process execution** — running a command with arguments and an optional working directory, returning exit status, stdout and stderr. Both `tmux` and `git` go through this; there is no separate tmux or git seam.
- **Filesystem** — reading a file (returning absence rather than erroring), writing a file, and directory traversal to a bounded depth without following symlinks.
- **User interaction** — a single prompt operation covering the three interaction shapes the tool needs: selecting from a list (the fuzzy picker), free-text input with an editable default, and yes/no confirmation with a default.
- **Clock** — current time, for frecency scoring.

Two implementations exist: the real one, and a fake used by every test. Assertions in tests are made against the sequence of commands the application emitted and the files it wrote — which is precisely the tool's externally observable behaviour, since vibestation's entire effect on the world is the argv it hands to `tmux` and `git` and the files it leaves in `~/.vibestation/`.

This is a deliberately wide trait. It is preferred over four narrow ones (`TmuxClient`, `GitClient`, `Store`, `Ui`) because it makes the highest possible seam — every test drives the whole application end to end through one fake, rather than testing modules in isolation and leaving their composition untested.

### Modules

- **`config`** — load, create and write `config.toml`; the first-run flow.
- **`tmux`** — build tmux argv; parse `list-sessions` output into session records; decide attach vs. switch-client.
- **`git`** — build git argv; read current branch; detect default branch; detect dirty working tree; create a branch.
- **`scan`** — walk configured roots for repositories; merge in manually added projects; resolve display names and collisions.
- **`state`** — read and write frecency data; compute ranking.
- **`naming`** — parse the ticket/description prompt input; generate and sanitise session and branch names.
- **`picker`** — assemble picker rows from sessions, projects and the two action entries; interpret the selection.
- **`app`** — the orchestration that composes the above; the single entry point tests call.

### Configuration

Location: **`~/.vibestation/config.toml`**. TOML.

Fields:

- `projects_dirs` — list of directory paths to scan. Modelled as a list from the first version even though first run populates a single entry, to avoid a breaking config change later.
- `extra_projects` — list of repository paths added manually through the picker. Written by the tool, editable by hand.
- `username` — the prefix in generated session and branch names. Defaulted on first run from `git config user.name`, slugified.
- `default_branch` — optional override for the branch new branches are cut from. When absent, detection falls back through `origin/HEAD`, then `main`, then `master`.
- `scan_depth` — maximum traversal depth, default 10.

First run asks exactly one question: the projects directory. Everything else is defaulted and written into the file with explanatory comments.

### Data files

Both under `~/.vibestation/`, both JSON.

- **`state.json`** — frecency records, each holding a project path, an open count, and a last-opened timestamp. Ranking uses zoxide's frecency algorithm: the open count scaled by a decay factor derived from the age of the last open (heavily favouring the last hour, then the last day, then the last week, then everything older). An "open" is recorded when a project selection results in a session being created or attached.
- **`projects-cache.json`** — the discovered project list. Written on first scan; rewritten only when the user chooses the refresh action. There is no time-based expiry — refresh is explicit, surfaced in the picker, so a stale cache is always one keypress from correct.

### tmux interaction

- Sessions are read with a single `list-sessions` call using a format string that emits session name, attached-client count, and the active pane's current path and current command. Only the active pane is inspected; other panes and windows are not enumerated.
- Branch is not available from tmux, so it is read per session from the active pane's path via git.
- A session with a client already attached is marked in the list but attaching is not blocked or confirmed — tmux's own default is to share, and that is the desired behaviour.
- New sessions are created detached, with the working directory set to the project directory, running the login shell. No startup command or window layout.
- After creation or on selection of an existing session, the tool switches the current client if it detects it is running inside tmux, and attaches otherwise.
- Absence of a tmux server is a normal state, not an error: the session portion of the list is simply empty.

### Naming rules

- When the repository's current branch is not the default branch, the suggested session name is `username/<current-branch>` and no prompt is shown.
- Otherwise a single prompt collects ticket and description together. Input leading with a ticket-shaped token (uppercase letters, hyphen, digits) has that token split off as the ticket; otherwise the entire input is treated as the description and no ticket prefix is applied.
- The description is lowercased, reduced to alphanumerics and hyphens, has runs of hyphens collapsed, and is truncated to roughly 50 characters.
- The result is `username/TICKET-description`, or `username/description` where there is no ticket.
- The generated name is presented as an editable default, not applied silently.
- Characters tmux forbids in session names (`.` and `:`) are replaced with hyphens. Forward slashes are preserved — tmux permits them and they carry the convention.

### Branch creation

- Offered only when the accepted session name differs from the repository's current branch — that is, after the ticket/description path, never after the resume path.
- The confirmation defaults to **yes** in that case, because reaching it means the developer has just described new work.
- The branch is created from the **local** default branch reference. No fetch is performed. Keeping the default branch current is the developer's responsibility and is assumed by their workflow.
- If the working tree is dirty, branch creation is skipped entirely with a warning naming what was skipped. The tmux session is still created, on the current branch.
- Declining the offer is not cancelling: the session is created regardless.

### Dependencies

Deliberately short: argument parsing, serde with TOML and JSON, an embedded fuzzy picker (so users are not required to have `fzf` installed), an interactive prompt library for text input and confirmation, a directory walker, and an error-handling crate. Home directory resolution uses the `HOME` environment variable rather than a crate.

### Delivery

Ten sequential PRs, each independently shippable and tested:

1. Cargo skeleton, CLI entry point, `--version`, CI (format, lint, test), licence, README stub.
2. Config module: load and write `config.toml`; first-run prompt; username defaulting.
3. tmux read path: session listing and parsing; branch resolution; graceful handling of no server.
4. Picker over the session list; attach vs. switch-client; attached-elsewhere marker. *(Working session switcher from here.)*
5. Project scanner: bounded-depth walk, stop-on-first-`.git`, symlink skipping, `extra_projects` merge, cache write, display-name collision handling.
6. Frecency state and ranking; unified picker with sessions above projects.
7. Name suggester and the ticket/description prompt; session creation and attach.
8. Branch-from-default: detection fallback chain, dirty-tree guard, confirmation.
9. Picker actions: refresh and add-manually, including repository validation and config write-back.
10. Homebrew tap, release workflow for macOS and Linux binaries, README and `--help` polish.

## Testing Decisions

### What makes a good test here

A good test drives the whole application through the `Host` fake and asserts on
what the application did to the outside world: which commands it emitted, in
what order, with what arguments and working directory, and what it wrote to
config, state and cache files. It never reaches into internal structures or
asserts that a particular function was called.

This is unusually tractable for this project because vibestation's external
behaviour is almost entirely argv. A test that asserts a `tmux new-session`
was emitted detached, with the expected session name and the expected working
directory, is testing user-visible behaviour, not implementation.

Tests set up a fake host with three scripted inputs — a virtual filesystem, a
map of command invocations to their outputs, and a queue of answers the user
gives to prompts — then run the application and assert on the resulting command
log and file writes.

There is no prior art in this repository; it is greenfield. This spec establishes
the convention, and PR 1 should land the `Host` trait and its fake so that every
subsequent PR has somewhere to put its tests.

### Coverage by area

- **Config** — round-tripping a written file; parsing a hand-written file with fields omitted; the first-run path producing a valid file from a single answer; a malformed file producing an error that names the file.
- **tmux parsing** — session records parsed from fixture format-string output, including sessions with unusual names, sessions in non-repository directories, and the empty output of a running server with no sessions; the no-server case treated as an empty list rather than a failure.
- **Attach decision** — switch-client emitted when the environment indicates the tool is running inside tmux, attach otherwise.
- **Scanner** — a fixture directory tree exercising nested repositories (asserting the walk stops at the first `.git`), the depth cap, symlink skipping, `extra_projects` merging and deduplication, and display-name collision producing a parent-directory suffix while unambiguous names stay bare.
- **Frecency** — a table of records with known counts and timestamps producing a known ordering; the decay boundaries; state file round-trip; an open recording bumping both count and timestamp.
- **Naming** — a table of prompt inputs to expected names covering ticket detection, absent tickets, slugification, hyphen collapsing, truncation, and replacement of tmux-forbidden characters; the resume path producing `username/<branch>` without consuming a prompt answer.
- **Default branch detection** — the full fallback chain, each stage exercised by scripting which git invocations succeed.
- **Branch creation** — the branch command emitted with the right base after a confirmed prompt; no branch command emitted when the tree is dirty, with the session still created; no branch command emitted on the resume path; declining the prompt still producing a session.
- **Picker actions** — refresh rewriting the cache; add-manually rejecting a non-repository path and accepting a valid one, with the accepted path appearing in the config file and the rest of the file preserved.

## Out of Scope

- **Git worktrees.** The originating discussion included grouping a project's worktrees under one entry and offering worktree selection or creation when starting a session. This is deferred entirely and is not designed for here beyond the forward-compatibility note below.
- **Mouse support.** Keyboard selection only.
- **Windows.**
- **A full-screen TUI.** The interaction is a fuzzy picker and a prompt or two, not a persistent multi-pane application.
- **Automatic fetching** before branch creation.
- **Stashing** or any other automated handling of a dirty working tree.
- **Inspecting panes other than the active one** for the running-command and directory columns.
- **Time-based cache expiry.** Refresh is user-triggered only.
- **zoxide integration.** Frecency is computed from vibestation's own state file.
- **Session templates, startup commands, or window layouts** on session creation.
- **Killing, renaming, or otherwise managing existing sessions.** Vibestation lists and attaches; it does not administer.
- **Remote or SSH sessions.**

## Further Notes

### Worktrees, deferred

The intended follow-on phase groups a project's worktrees visually under a single
project entry, and offers a choice between existing worktrees or creating a new
one when starting a session. Two decisions in this spec were made with that
future in mind:

- The scanner matches both a `.git` **directory** and a `.git` **file**, so
  linked worktrees are already discovered rather than invisible.
- Forward slashes are preserved in session names.

When worktrees land, the branch-creation behaviour specified here — switching the
project's single checkout onto a new branch — is expected to be *replaced* by
creating a worktree instead. That is an intentional behaviour swap in a later
phase, not something v1 should try to abstract over now.

### On the dirty-tree guard

Skipping branch creation rather than stashing is a firm decision, not a
simplification to revisit casually. Vibestation runs at the moment a developer is
switching context and least attentive to what a tool is doing to their working
tree; the guard is in the constitution for that reason.
