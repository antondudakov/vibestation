# vibestation

One command, one fuzzy picker: your live tmux sessions on top, your git
projects below, and a session waiting at the end of either.

Not usable yet — this is the skeleton. See
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

On a Mac, install or update with [`scripts/install-macos.sh`](scripts/install-macos.sh),
run from a clone. It pulls, installs into `~/.local/bin` and ad-hoc signs the
binary — the macOS build is cross-compiled on Linux, so it arrives unsigned and
Apple Silicon will not run it otherwise.

Rebuild both:

```sh
cargo build --release --target x86_64-unknown-linux-musl
cp target/x86_64-unknown-linux-musl/release/vibestation dist/vibestation-x86_64-linux

# needs zig and cargo-zigbuild; macOS has no linker on Linux without them
cargo zigbuild --release --target universal2-apple-darwin
cp target/universal2-apple-darwin/release/vibestation dist/vibestation-macos
```

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
