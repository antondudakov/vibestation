# vibestation

One command, one fuzzy picker: your live tmux sessions on top, your git
projects below, and a session waiting at the end of either.

```
? Open  2 running · 2 projects
▶  ada/VBSN-4-picker   ~/code/vibestation  VBSN-4-picker   nvim  2m
●  notes-scratch       /etc                                zsh   1h
───────────────────────────────────────────────────────────────────
   api                 ~/code/api
└  api-VBSN-7-retries                      VBSN-7-retries
   notes               ~/notes
↻  refresh the project list
✚  add a project by path
[↑↓ move · type to filter · enter select · esc cancel]
```

Pick a session and you are in it. Pick a project and vibestation names the
work, offers the branch that name implies — in a worktree beside the checkout
or in place — creates the session in whichever directory the work now lives in,
and drops you there.

## Install

macOS, via the tap:

```sh
brew install antondudakov/vibestation/vibestation
```

Linux — the release carries a static binary that runs on any x86_64 Linux
whatever its glibc:

```sh
curl -fsSLo ~/.local/bin/vibestation \
  https://github.com/antondudakov/vibestation/releases/latest/download/vibestation-x86_64-linux
chmod +x ~/.local/bin/vibestation
```

Or from a clone: `cargo build --release`.

`tmux` and `git` are invoked as subprocesses, so both need to be on your PATH.
macOS and Linux only.

First run asks one question — where your projects live — and writes
`~/.vibestation/config.toml` with every other setting commented in it.

## The picker

One picker, always, as tall as your terminal. Live tmux sessions first, each
with its directory, the branch checked out there, what is running in the active
pane, and how long since a client last had it — most recently left first, so
the session you want is usually the first row. Then a separator, then your git
projects ranked by how often and how recently you open them, each with its
worktrees beneath it. Typing filters every row at once, so a ticket identifier
reaches its worktree directly.

Every row is laid out in the same columns, padded to the widths of the whole
list, so sessions, projects and worktrees read as one grid rather than three.
The grid is fitted to your terminal rather than wrapped: a path gives up its
front — `~/…/android-monorepo-3` — and on a narrow terminal the columns
disappear in the order you would have deleted them yourself, the command first.

The glyph at the front says what a row is, which is what keeps a scrolled list
readable:

| glyph | the row is |
|---|---|
| `▶` | the session you are in |
| `●` | a session attached somewhere else |
| `○` | a session running with nobody in it |
| _blank_ | a project |
| `└` | one of that project's worktrees |

Your own username is stripped from every branch shown, since it is on all of
them. The prompt line above the list — the one line that never scrolls — says
how much of the list is running work.

A project that already has a session appears only as that session — the same
work is never listed twice.

Choosing a session joins it: `switch-client` when you are already inside tmux,
`attach-session` when you are not. Esc costs nothing and emits nothing, and so
does the separator — choosing decoration just reopens the list.

The last two rows are the escape hatches. **Refresh** rescans your projects
directories and rewrites the cache, which is how a repository you just cloned
appears. **Add a project by path** takes a repository that lives outside those
directories, refuses anything that is not a git repository, and writes what it
accepts into your config, so it survives every later refresh.

## Starting work

Choosing a project or a worktree settles on a session name first.

A checkout already on a feature branch, or a worktree, is work in progress:
its branch names it, `username/<branch>`, and nothing is asked. That makes
resuming a single keypress.

A checkout sitting on its default branch is starting something, so it asks
once — "What are you working on?" — and turns one line into a name:

| you type | you get |
|---|---|
| `VBSN-1 initialize the project` | `ada/VBSN-1-initialize-the-project` |
| `fix the flaky retry test` | `ada/fix-the-flaky-retry-test` |
| `VBSN-1` | `ada/VBSN-1` |

A leading ticket-shaped token — uppercase letters, a hyphen, digits — is kept
as typed; the rest is lowercased, reduced to alphanumerics and hyphens, and
truncated to the last whole word inside 50 characters. The result is offered as
an editable default, never applied silently. `.` and `:` become `-`, because
tmux forbids them in session names; slashes stay, because tmux allows them and
they carry the convention.

### The branch

When the accepted name differs from the branch you are on, vibestation offers
to cut it, three ways:

1. **A new worktree** at `../<project>-<branch>` — `vibestation` and
   `ada/VBSN-1-init` give the sibling directory `vibestation-VBSN-1-init`. This
   is the default, because it alters no existing checkout and so is always safe.
2. **In place** in the current checkout. Withheld, with an explanation, when
   the tree has uncommitted changes.
3. **Neither** — the session is created on the current branch and no git state
   changes. Saying no is not the same as cancelling.

A fetch is offered before the cut, pre-answered from `fetch_before_branch`. On
success the branch comes off `origin/<default>`, so your local default branch
ref is never touched and the cut cannot fail on divergence. A fetch that fails
— offline, no remote, no auth — warns and cuts from the local ref instead.
Never fatal.

The default branch is your config override if you set one, else what
`origin/HEAD` points at, else whichever of `main` and `master` exists.

### Worktrees

Worktrees are created as siblings of the main checkout, so every other tool
sees an ordinary repository and you can find them from a shell prompt. They are
grouped back under their project by asking git which repository they belong to,
not by their directory names, which is why a worktree living outside your
projects directories still lists under its project. A worktree whose directory
is gone is dropped rather than shown as a broken row.

If the derived directory already exists and is this repository's worktree on
that branch, it is reused rather than recreated — repeating an action is
harmless. If it exists as anything else, the work stops with a message naming
it: vibestation never writes into a directory it did not create.

## Configuration

`~/.vibestation/config.toml`, written on first run with a comment on every
field. Delete any field to get its default back.

| setting | what it does | default |
|---|---|---|
| `projects_dirs` | Directories scanned for git repositories. | the one question first run asks |
| `extra_projects` | Repositories added by hand, or by the picker's add-manually row. | `[]` |
| `username` | Prefix for generated session and branch names. | `git config user.name`, slugified |
| `default_branch` | Override for the branch new branches are cut from. | detected per repository |
| `scan_depth` | How deep below each projects directory to look. | `10` |
| `fetch_before_branch` | The pre-selected answer to "Fetch origin first?". | `true` |

Two data files sit beside it. `projects-cache.json` holds the discovered
projects, so opening the picker never waits on your disk; it is rewritten only
when you choose refresh, there is no expiry, and a stale list is always one
keypress from correct. `state.json` holds one open count and timestamp per
project, which is the frecency ranking. Both are derived data and can be
deleted at any time.

The scan walks each root to `scan_depth`, stops descending at the first `.git`
it finds — so submodules and vendored repositories stay out of the list — and
does not follow symlinks. Nothing in the picker touches the network; the only
network call the tool ever makes is the fetch you confirm before a branch.

## Releasing

A tag is the whole procedure:

```sh
git tag v1.0.0 && git push origin v1.0.0
```

[`.github/workflows/release.yml`](.github/workflows/release.yml) creates the
release, builds the Linux binary on Linux and the universal macOS binary on
macOS — natively, so Apple's linker ad-hoc signs it — runs each one to check it
reports the tagged version, attaches both, and points the formula in
[`homebrew-vibestation`](https://github.com/antondudakov/homebrew-vibestation)
at the new assets. The tag must match `version` in `Cargo.toml` or the build
stops, since the binary takes its version from there.

That last step needs a `TAP_TOKEN` repository secret — a fine-grained personal
access token granting Contents: write on the tap repository alone — because a
workflow's own token cannot write to another repository.

## Development

```sh
cargo fmt --all      # CI runs --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Everything impure lives behind the `Host` trait in
[`src/host.rs`](src/host.rs) — process execution, filesystem, prompts and the
clock — with one real implementation and one fake in
[`src/fake.rs`](src/fake.rs). Tests script the fake with a virtual filesystem,
a map of commands to their output and a queue of prompt answers, then assert on
the command log and the file writes, which are the tool's entire effect on the
world. [`tests/host_convention.rs`](tests/host_convention.rs) shows the shape.

`dist/` holds checked-in builds for trying unreleased work on a machine with no
Rust toolchain; [`scripts/build.sh`](scripts/build.sh) rebuilds both targets and
[`scripts/install-linux.sh`](scripts/install-linux.sh) and
[`scripts/install-macos.sh`](scripts/install-macos.sh) install from a clone into
`~/.local/bin`. Cross-building macOS from Linux needs `zig` and
`cargo-zigbuild`. To rebuild `dist/` on every commit that touches the build,
enable the checked-in hook once per clone:

```sh
git config core.hooksPath .githooks
```

Released versions come from GitHub Releases and the tap instead.

See [`CONSTITUTION.md`](CONSTITUTION.md) for the principles this is built to,
[`specs/001-vibestation-v1.md`](specs/001-vibestation-v1.md) for what v1 is and
[`specs/001-vibestation-v1/README.md`](specs/001-vibestation-v1/README.md) for
the thirteen tickets that built it.

## Licence

MIT.
