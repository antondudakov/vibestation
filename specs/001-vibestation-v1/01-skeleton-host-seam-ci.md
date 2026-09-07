# 01: Cargo skeleton, `Host` seam and CI

**What to build:** `vibestation --version` runs from a built binary and reports
the installed version. Every subsequent ticket has a place to put its tests: the
single `Host` trait covering process execution, filesystem, user interaction and
the clock, with one real implementation and one test fake. CI checks formatting,
lints and tests on every pull request.

The fake takes three scripted inputs — a virtual filesystem, a map of command
invocations to their outputs, and a queue of answers the user gives to prompts —
and records the command log and file writes for assertions. This is the entire
testing convention for the project, so it is worth getting right here.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] `cargo build` produces a binary; `--version` reports the crate version
- [ ] `--help` exists and lists the flags that exist so far
- [ ] The `Host` trait covers process execution (argv, optional working directory, returning exit status, stdout and stderr), file read that reports absence rather than erroring, file write, existence test, bounded-depth directory traversal that does not follow symlinks, prompt (select from list, free-text with editable default, yes/no with default), and current time
- [ ] There is exactly one real implementation and one fake; the fake records the ordered command log and every file write
- [ ] A test drives the fake end to end and asserts on the recorded command log
- [ ] CI runs `cargo fmt --check`, `cargo clippy` and `cargo test` on every PR
- [ ] Licence and README stub are present
