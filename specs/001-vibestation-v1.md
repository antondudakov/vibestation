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
that matches their team's convention, remember to update the default branch
before cutting from it, create the branch, create a tmux session, and name it
something they'll recognise later. It's half a dozen commands, every one of them
muscle memory that breaks the moment they're on an unfamiliar machine — and the
two steps most often skipped are the two that hurt later: naming the branch
consistently, and making sure it was cut from an up-to-date default branch rather
than a `main` that's four days stale.

Worktrees make the bookkeeping worse before they make it better. A developer who
uses them ends up with `project`, `project-VBSN-1-init` and `project-VBSN-4-picker`
sitting as three unrelated-looking directories, when they are really one project
in three states. Any tool that lists directories shows them as three projects;
the developer has to hold the relationship in their head.

Existing tools solve fragments. `tmux ls` lists sessions without context. Session
switchers attach to what exists but don't know about projects. Project launchers
open a directory but know nothing about branches, naming conventions, or
worktrees. Nothing connects "which repos do I have" to "which of these
directories are the same project" to "what should this branch be called" to "put
me in a tmux session for it".

## Solution

`vibestation` — a single command that is the only entry point to a developer's
work.

Running it opens one fuzzy-filterable picker. At the top are the developer's live
tmux sessions, each row showing the session name, its working directory, the git
branch of its active pane, the command currently running there, and a marker if a
client is already attached elsewhere. Below a separator are all git repositories
discovered under the developer's configured projects directories, ranked by how
often and how recently they've been opened — and each project's worktrees appear
as indented child rows beneath it, so three sibling directories on disk read as
one project in three states.

Selecting a session attaches to it — switching the current client if vibestation
was run from inside tmux, attaching if it was run outside.

Selecting a project starts new work. If the repository is already on a feature
branch, vibestation suggests `username/that-branch` as the session name and gets
out of the way. If it's on the default branch, it asks one question — ticket and
description — and builds `username/TICKET-kebab-description` from the answer. The
developer can edit the suggestion.

Vibestation then asks how to create that branch: as a **new worktree** alongside
the main checkout, or **in place** in the existing checkout. Either way it offers
to `git fetch origin` first, so the branch is cut from a current
`origin/<default>` rather than a stale local ref. It then creates a detached tmux
session rooted in whichever directory the work now lives in, and attaches to it.

Selecting an existing worktree row is just resuming: no prompts, no branch
creation, straight into a session named for that worktree's branch.

The picker also carries two escape hatches: a refresh entry that rescans the
projects directories, and an add-manually entry for a repository that lives
outside them.

The result: one command, a keyboard, and the developer is working — in the right
directory, on a correctly named branch, cut from a current default branch, in a
session they'll recognise tomorrow.

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

### Worktrees

30. As a developer using worktrees, I want a project's worktrees grouped under that project rather than listed as separate top-level projects, so that one project reads as one thing.
31. As a developer, I want each worktree shown as an indented child row of its project, so that the relationship is visible without opening anything.
32. As a developer, I want to fuzzy-filter across worktree rows too, so that typing a ticket number jumps me straight to that worktree.
33. As a developer, I want the main checkout distinguished from its worktrees in the list, so that I know which directory is the primary one.
34. As a developer, I want each worktree row to show its branch, so that I can pick the right one without remembering directory names.
35. As a developer selecting an existing worktree, I want a session created for it with no prompts, so that resuming existing work is a single keypress.
36. As a developer, I want worktrees that live outside my configured projects directories still listed under their project, so that grouping is based on the repository, not on where I happened to scan.
37. As a developer, I want worktrees created as sibling directories next to the main checkout, so that they're where I'd expect to find them and every other tool sees them normally.
38. As a developer, I want the worktree directory named after the project and the branch, so that I can identify it from a shell prompt or a file picker.
39. As a developer, I want to be told and stopped if the worktree directory already exists as something else, so that vibestation never writes over an unrelated directory.
40. As a developer whose worktree for a branch already exists, I want it reused rather than recreated, so that repeating an action is harmless.
41. As a developer, I want a session created in the new worktree's directory, so that my shell starts in the code I'm about to change.
42. As a developer, I want a stale worktree entry whose directory no longer exists to be omitted from the list, so that deleted worktrees don't linger.

### First run and configuration

43. As a new user, I want to be asked for my projects directory on first run, so that the tool works immediately without me reading documentation first.
44. As a new user, I want a config file written for me with commented defaults, so that I can discover the other settings later.
45. As a new user, I want my username defaulted from my git config, so that suggested session names are correct without me typing it.
46. As a developer, I want my config in a plain TOML file at a predictable path, so that I can edit it in an editor or keep it in my dotfiles.
47. As a developer, I want a malformed config file to produce a clear error naming the file, so that I can fix it rather than guess.

### Naming and creating sessions

48. As a developer opening a project that has no session, I want a session name suggested for me, so that I don't have to invent one under pressure.
49. As a developer whose repository is already on a feature branch, I want the suggestion to be `username/<that-branch>` with no questions asked, so that resuming existing work is a single keypress.
50. As a developer whose repository is on the default branch, I want to be asked for a ticket and description, so that the session name matches the work I'm about to start.
51. As a developer, I want the ticket and description asked in a single prompt, so that starting work isn't a questionnaire.
52. As a developer, I want a leading ticket identifier recognised automatically, so that I can type "VBSN-1 initialize the project" the way I'd say it.
53. As a developer with no ticket number, I want to type just a description, so that unticketed work isn't blocked.
54. As a developer, I want my description turned into a lowercase kebab-case slug, so that the resulting branch name is valid and readable.
55. As a developer, I want an overly long description truncated, so that the session name stays scannable in the picker.
56. As a developer, I want to edit the suggested name before it's used, so that I can override the generated one when it's wrong.
57. As a developer, I want characters tmux forbids in session names replaced automatically, so that session creation never fails on a name I was offered.
58. As a developer, I want to be dropped into the new session immediately, so that creating and attaching are one step.
59. As a developer, I want opening a project to count towards its frecency, so that the ranking reflects my actual habits over time.
60. As a developer, I want a plain shell in the new session with no preset layout, so that the tool doesn't impose a workflow on me.

### Creating a branch

61. As a developer starting ticketed work, I want to be offered a new branch rather than being left on the default branch, so that I don't start work on main by accident.
62. As a developer, I want to choose between a new worktree and branching in place, so that I can use worktrees for parallel work and in-place branching when I don't need one.
63. As a developer, I want the worktree option offered as the default, so that the safer choice is the one keypress choice.
64. As a developer, I want to decline both and still get my session on the current branch, so that saying no isn't the same as cancelling.
65. As a developer resuming an existing feature branch, I want no branch prompt at all, so that resuming stays frictionless.
66. As a developer, I want the repository's default branch detected automatically, so that repositories still using `master` work without configuration.
67. As a developer, I want to override the default branch in config, so that a repository using `develop` works too.
68. As a developer, I want to be offered a fetch before the branch is cut, so that my new branch starts from a current default branch rather than a stale local one.
69. As a developer, I want that fetch offer pre-answered from a config setting, so that I can make "always fetch" my default and stop being asked.
70. As a developer working offline, I want a failed fetch to fall back to the local ref with a warning, so that no network doesn't mean no work.
71. As a developer with uncommitted changes, I want the in-place branching option withheld with an explanation, so that my work in progress is never disturbed.
72. As a developer with uncommitted changes, I want the worktree option still available, so that a dirty main checkout doesn't block me from starting something new.
73. As a developer, I want the branch cut from the remote default branch after fetching, so that my local default branch ref is never touched and the operation can't fail on divergence.

### Installing and contributing

74. As a macOS developer, I want to install via Homebrew, so that installing is one command.
75. As a Linux developer, I want a prebuilt binary attached to the GitHub release, so that I don't need a Rust toolchain to try it.
76. As a user, I want `--help` to explain what the tool does and what its flags are, so that I can orient myself without the README.
77. As a user, I want `--version` to report the installed version, so that I can file a useful bug report.
78. As a contributor, I want CI running formatting, lints and tests on every PR, so that I know a change is safe to merge.

## Implementation Decisions

### Language, platform, distribution

- **Rust**, single static binary, built from scratch rather than wrapping or forking an existing session manager (`sesh`, `tmux-sessionizer`). The branch-naming and worktree workflow is the tool's reason to exist and threading it through another tool's config and hook model costs more than the session-listing plumbing it would save.
- **macOS and Linux only.** No Windows accommodation in code. `$HOME` is read from the environment directly rather than taking a directory-resolution dependency.
- **Distribution:** GitHub Releases with prebuilt macOS and Linux binaries, plus a Homebrew tap.

### The seam

There is exactly one seam: a **`Host` trait** capturing every impure interaction the application has with the outside world. Application logic is a pure function over it.

The trait covers:

- **Process execution** — running a command with arguments and an optional working directory, returning exit status, stdout and stderr. Both `tmux` and `git` go through this; there is no separate tmux or git seam.
- **Filesystem** — reading a file (returning absence rather than erroring), writing a file, testing existence, and directory traversal to a bounded depth without following symlinks.
- **User interaction** — a single prompt operation covering the interaction shapes the tool needs: selecting from a list (the fuzzy picker and the branch-strategy choice), free-text input with an editable default, and yes/no confirmation with a default.
- **Clock** — current time, for frecency scoring.

Two implementations exist: the real one, and a fake used by every test. Assertions in tests are made against the sequence of commands the application emitted and the files it wrote — which is precisely the tool's externally observable behaviour, since vibestation's entire effect on the world is the argv it hands to `tmux` and `git` and the files it leaves in `~/.vibestation/`.

This is a deliberately wide trait. It is preferred over four narrow ones (`TmuxClient`, `GitClient`, `Store`, `Ui`) because it makes the highest possible seam — every test drives the whole application end to end through one fake, rather than testing modules in isolation and leaving their composition untested.

### Modules

- **`config`** — load, create and write `config.toml`; the first-run flow.
- **`tmux`** — build tmux argv; parse `list-sessions` output into session records; decide attach vs. switch-client.
- **`git`** — build git argv; read current branch; detect default branch; detect dirty working tree; fetch; create branches; create and enumerate worktrees.
- **`scan`** — walk configured roots for repositories; merge in manually added projects; resolve display names and collisions.
- **`group`** — collapse discovered repositories into projects with their worktrees; identify the main checkout.
- **`state`** — read and write frecency data; compute ranking.
- **`naming`** — parse the ticket/description prompt input; generate and sanitise session names, branch names and worktree directory names.
- **`picker`** — assemble picker rows from sessions, projects, worktree children and the two action entries; interpret the selection.
- **`app`** — the orchestration that composes the above; the single entry point tests call.

### Configuration

Location: **`~/.vibestation/config.toml`**. TOML.

Fields:

- `projects_dirs` — list of directory paths to scan. Modelled as a list from the first version even though first run populates a single entry, to avoid a breaking config change later.
- `extra_projects` — list of repository paths added manually through the picker. Written by the tool, editable by hand.
- `username` — the prefix in generated session and branch names. Defaulted on first run from `git config user.name`, slugified.
- `default_branch` — optional override for the branch new branches are cut from. When absent, detection falls back through `origin/HEAD`, then `main`, then `master`.
- `scan_depth` — maximum traversal depth, default 10.
- `fetch_before_branch` — boolean, default true. Supplies the pre-selected answer to the fetch confirmation shown before a branch is created.

First run asks exactly one question: the projects directory. Everything else is defaulted and written into the file with explanatory comments.

### Data files

Both under `~/.vibestation/`, both JSON.

- **`state.json`** — frecency records, each holding a project path, an open count, and a last-opened timestamp. Ranking uses zoxide's frecency algorithm: the open count scaled by a decay factor derived from the age of the last open (heavily favouring the last hour, then the last day, then the last week, then everything older). An "open" is recorded when a project or worktree selection results in a session being created or attached; frecency is tracked against the project, not the individual worktree.
- **`projects-cache.json`** — the discovered and grouped project list: projects, each with its main checkout path and its worktrees. Written on first scan; rewritten only when the user chooses the refresh action. There is no time-based expiry — refresh is explicit, surfaced in the picker, so a stale cache is always one keypress from correct.

### tmux interaction

- Sessions are read with a single `list-sessions` call using a format string that emits session name, attached-client count, and the active pane's current path and current command. Only the active pane is inspected; other panes and windows are not enumerated.
- Branch is not available from tmux, so it is read per session from the active pane's path via git.
- A session with a client already attached is marked in the list but attaching is not blocked or confirmed — tmux's own default is to share, and that is the desired behaviour.
- New sessions are created detached, with the working directory set to the project or worktree directory, running the login shell. No startup command or window layout.
- After creation or on selection of an existing session, the tool switches the current client if it detects it is running inside tmux, and attaches otherwise.
- Absence of a tmux server is a normal state, not an error: the session portion of the list is simply empty.

### Worktree grouping

Worktrees are created as **sibling directories** of the main checkout, which means the scanner discovers them as ordinary repositories and the grouping must be derived rather than inferred from the directory layout.

- For each discovered repository, the git directory and the git *common* directory are resolved. Linked worktrees of the same repository share a common directory; a main checkout is the one where the two are equal. Repositories are grouped by common directory, and the group's main checkout is identified by that equality.
- Once a group's main checkout is known, `git worktree list` against it provides the **authoritative** set of worktrees. This is deliberately a superset of what the scan found: it picks up worktrees living outside the configured projects directories, which is why grouping keys off the repository rather than off where the scan happened to reach.
- Worktree entries whose directory no longer exists on disk are dropped from the list rather than shown as broken rows.
- A discovered worktree whose main checkout lies outside the configured roots, and is therefore never scanned, is listed as a standalone project rather than being hidden.
- All of this happens at scan time and is written into the cache, so the cost is paid on explicit refresh rather than on every picker open. It performs no network access.

### Picker layout

One picker, always. Rows in order:

- Live tmux sessions.
- A separator.
- Projects ranked by frecency, each followed by its worktrees as indented child rows. The main checkout is visually distinguished from its worktrees; each worktree row shows its branch.
- The refresh and add-manually action entries.

Every row is fuzzy-filterable, including worktree children, so typing a ticket identifier reaches its worktree directly.

### Naming rules

- When the repository's current branch is not the default branch, the suggested session name is `username/<current-branch>` and no prompt is shown.
- Otherwise a single prompt collects ticket and description together. Input leading with a ticket-shaped token (uppercase letters, hyphen, digits) has that token split off as the ticket; otherwise the entire input is treated as the description and no ticket prefix is applied.
- The description is lowercased, reduced to alphanumerics and hyphens, has runs of hyphens collapsed, and is truncated to roughly 50 characters.
- The result is `username/TICKET-description`, or `username/description` where there is no ticket.
- The generated name is presented as an editable default, not applied silently.
- Characters tmux forbids in session names (`.` and `:`) are replaced with hyphens. Forward slashes are preserved — tmux permits them and they carry the convention.
- The **worktree directory name** is derived as the main checkout's directory name joined to the branch name with the `username/` prefix stripped and any remaining slashes replaced by hyphens — so branch `antondudakov/VBSN-1-init` in project `vibestation` yields the sibling directory `vibestation-VBSN-1-init`.

### Branch and worktree creation

Offered only when the accepted session name differs from the repository's current branch — that is, after the ticket/description path, never after the resume path or when an existing worktree was selected.

The developer is asked how to create the branch, as a three-way choice:

1. **A new worktree** at the derived sibling path. This is the default.
2. **In place** in the existing checkout. This option is withheld, with an explanation, when the checkout has uncommitted changes.
3. **Neither** — create the session on the current branch and change no git state.

If a branching option is chosen, a fetch confirmation follows, pre-answered from `fetch_before_branch`. When accepted, `git fetch origin` runs and the branch is cut from `origin/<default-branch>`; the local default branch ref is never modified, so the operation cannot fail on divergence or on the state of the main checkout. When the fetch fails — offline, no remote, authentication — the failure is reported as a warning and the branch is cut from the local default ref instead. A failed fetch never aborts the work.

Worktree creation and branch creation are a single git operation. If the derived path already exists and is a worktree of this repository on the requested branch, it is reused rather than recreated. If it exists as anything else, the operation stops with a message naming the path; vibestation never writes into a directory it did not create.

The session is then created in whichever directory the work now lives in — the new or reused worktree, or the main checkout for the in-place and neither options.

### Dependencies

Deliberately short: argument parsing, serde with TOML and JSON, an embedded fuzzy picker (so users are not required to have `fzf` installed), an interactive prompt library for text input, selection and confirmation, a directory walker, and an error-handling crate. Home directory resolution uses the `HOME` environment variable rather than a crate.

### Delivery

Twelve sequential PRs, each independently shippable and tested:

1. Cargo skeleton, CLI entry point, `--version`, CI (format, lint, test), licence, README stub, and the `Host` trait with its test fake.
2. Config module: load and write `config.toml`; first-run prompt; username defaulting.
3. tmux read path: session listing and parsing; branch resolution; graceful handling of no server.
4. Picker over the session list; attach vs. switch-client; attached-elsewhere marker. *(Working session switcher from here.)*
5. Project scanner: bounded-depth walk, stop-on-first-`.git`, symlink skipping, `extra_projects` merge, cache write, display-name collision handling.
6. Worktree grouping: common-directory grouping, main-checkout identification, `git worktree list` enumeration, stale-entry pruning, grouped cache structure.
7. Frecency state and ranking; unified picker with sessions, projects and indented worktree children.
8. Name suggester and the ticket/description prompt; session creation and attach; the resume path for existing worktrees.
9. Default-branch detection with its fallback chain; the fetch confirmation and fetch-then-branch-from-`origin/<default>` behaviour; in-place branch creation with the dirty-checkout guard.
10. Worktree creation: the three-way strategy choice, sibling path derivation, existing-worktree reuse, occupied-path refusal.
11. Picker actions: refresh and add-manually, including repository validation and config write-back.
12. Homebrew tap, release workflow for macOS and Linux binaries, README and `--help` polish.

## Testing Decisions

### What makes a good test here

A good test drives the whole application through the `Host` fake and asserts on
what the application did to the outside world: which commands it emitted, in
what order, with what arguments and working directory, and what it wrote to
config, state and cache files. It never reaches into internal structures or
asserts that a particular function was called.

This is unusually tractable for this project because vibestation's external
behaviour is almost entirely argv. A test that asserts a `git worktree add` was
emitted with the expected branch, path and base ref, followed by a `tmux
new-session` rooted in that path, is testing user-visible behaviour, not
implementation.

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
- **Grouping** — sibling worktrees collapsed under one project by scripting their common directory; the main checkout identified by git-dir/common-dir equality; a worktree outside the scanned roots surfaced by `git worktree list` and grouped correctly; a worktree whose directory is absent pruned from the list; a worktree whose main checkout is outside the roots listed standalone.
- **Frecency** — a table of records with known counts and timestamps producing a known ordering; the decay boundaries; state file round-trip; an open recording bumping both count and timestamp; a worktree selection crediting its parent project.
- **Naming** — a table of prompt inputs to expected names covering ticket detection, absent tickets, slugification, hyphen collapsing, truncation, and replacement of tmux-forbidden characters; the resume path producing `username/<branch>` without consuming a prompt answer; worktree directory names derived from project and branch with the username prefix stripped.
- **Default branch detection** — the full fallback chain, each stage exercised by scripting which git invocations succeed.
- **Fetch** — fetch emitted before branch creation when confirmed and suppressed when declined; the confirmation pre-answered from config; the branch cut from `origin/<default>` on success; a failing fetch producing a warning and a branch cut from the local ref rather than an abort; no fetch emitted anywhere on the path to an existing session.
- **Branch strategy** — the worktree option offered as default; the in-place option withheld when the checkout is dirty while the worktree option remains available; choosing neither emitting no git mutation but still creating a session; no strategy prompt at all on the resume path.
- **Worktree creation** — `git worktree add` emitted with the expected branch, derived sibling path and base ref, followed by a session rooted in that path; an existing worktree for the branch reused with no add emitted; an occupied path stopping the operation with no add and no session.
- **Picker actions** — refresh rewriting the cache; add-manually rejecting a non-repository path and accepting a valid one, with the accepted path appearing in the config file and the rest of the file preserved.

## Out of Scope

- **Mouse support.** Keyboard selection only.
- **Windows.**
- **A full-screen TUI.** The interaction is a fuzzy picker and a prompt or two, not a persistent multi-pane application.
- **Removing, pruning or moving worktrees.** Vibestation creates and lists them; tidying them up is `git worktree` territory.
- **Bare-repository worktree layouts.** Grouping assumes a normal main checkout with linked worktrees beside it.
- **Fetching from remotes other than `origin`**, or branching from a remote other than the detected default.
- **Updating the local default branch ref.** The fetch path deliberately branches from `origin/<default>` and leaves local refs alone.
- **Stashing** or any other automated handling of a dirty working tree.
- **Inspecting panes other than the active one** for the running-command and directory columns.
- **Time-based cache expiry.** Refresh is user-triggered only.
- **zoxide integration.** Frecency is computed from vibestation's own state file.
- **Session templates, startup commands, or window layouts** on session creation.
- **Killing, renaming, or otherwise managing existing sessions.** Vibestation lists and attaches; it does not administer.
- **Remote or SSH sessions.**

## Further Notes

### Why grouping is derived rather than structural

Sibling worktree directories were chosen over a nested `.worktrees/` layout
because every other tool — shells, editors, file managers, other git tooling —
sees them as ordinary directories in an ordinary place. The cost is that the
project-to-worktree relationship is invisible on disk and has to be reconstructed
from git itself, which is why the `group` module exists at all and why grouping
keys off the git common directory rather off directory names. Naming a worktree
directory `<project>-<branch-slug>` is a convenience for humans reading a shell
prompt; nothing in the grouping logic depends on that convention holding, and a
worktree created by hand under any other name still groups correctly.

### Why the fetch cannot fail the operation

The fetch exists to stop branches being cut from a stale default branch, which is
a real and common problem. It is not important enough to stand between a
developer and their work. Every failure mode — offline, VPN down, credentials
expired, no remote at all — degrades to branching from the local ref with a
warning. This is the reason the branch is cut from `origin/<default>` rather than
from a fast-forwarded local `main`: it achieves the same freshness with no
possibility of failing on divergence, and it never touches a ref the developer
might care about.

### On the dirty-checkout guard

With worktrees available, a dirty main checkout no longer blocks starting new
work — it only removes the in-place option, and the worktree option is unaffected
because it touches no existing checkout. Withholding rather than warning is
deliberate: an option that is offered and then refused is worse than one that was
never offered, and the alternative is right there.
