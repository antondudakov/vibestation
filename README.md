# vibestation

One command, one fuzzy picker: your live tmux sessions on top, your git
projects below, and a session waiting at the end of either.

Usable today: `vibestation` opens one picker over your live tmux sessions and,
below them, your git projects ranked by how often and how recently you open
them, each with its worktrees indented beneath it. Choosing a session joins it.
Choosing a project or a worktree settles on a session name — `ada/VBSN-1-init`
from the branch you are already on, or from one line like `VBSN-1 initialize the
project` when you are on the default branch — creates the session in the right
directory and drops you in. Branching and worktree creation are still to come —
see
[`specs/001-vibestation-v1.md`](specs/001-vibestation-v1.md) for what v1 does
and [`specs/001-vibestation-v1/README.md`](specs/001-vibestation-v1/README.md)
for the tickets that get it there.

## Build

```sh
cargo build --release
./target/release/vibestation --version
```

`dist/` holds checked-in builds, so the tool can be tried on a machine with no
Rust toolchain. `vibestation-macos` is a universal binary for Intel and Apple
Silicon; `vibestation-x86_64-linux` is statically linked against musl and runs
on any x86_64 Linux whatever its glibc.

Install or update from a clone with
[`scripts/install-linux.sh`](scripts/install-linux.sh) or
[`scripts/install-macos.sh`](scripts/install-macos.sh). Both pull, then install
into `~/.local/bin` (override with `BIN_DIR=`); pass `--no-pull` to install what
the working tree already has. The macOS one also ad-hoc signs the binary — that
build is cross-compiled on Linux, so it arrives unsigned and Apple Silicon will
not run it otherwise.

Rebuild both with one command, then commit `dist/` to ship:

```sh
./scripts/build.sh
```

It runs the same checks CI does, cross-builds both targets and copies them into
`dist/`. Cross-building macOS from Linux needs `zig` and `cargo-zigbuild`, since
there is no macOS linker otherwise. Pass `--quick` to skip the check gate.

Each binary is stamped with a build number — the commit count — so
`vibestation --version` reports `0.1.0 (build 43)` and says which commit it came
from. A plain `cargo build` is unstamped and reports `0.1.0`, so a dev build
never claims to be a shipped one. The commit's own hash can't serve here: the
binaries are part of the commit.

To rebuild `dist/` automatically, enable the checked-in hook once per clone:

```sh
git config core.hooksPath .githooks
```

It rebuilds and stages `dist/` before each commit that touches `src/`,
`Cargo.toml`, `Cargo.lock` or the build script — a docs-only commit is left
alone, since it would otherwise add 4.3M of identical binaries to history.
On a machine with no toolchain the commit still goes through with a warning.
`git commit --no-verify` skips it once.

This is a convenience for the pre-release tickets; ticket 12 replaces it with
GitHub Releases and a Homebrew tap.

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

See [`CONSTITUTION.md`](CONSTITUTION.md) for the principles this is built to.

## Licence

MIT.
