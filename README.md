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
