# 12: Release workflow and Homebrew tap

**What to build:** Installing does not require a Rust toolchain. A tagged release
produces prebuilt macOS and Linux binaries attached to the GitHub release, and a
Homebrew tap makes macOS installation one command.

This ticket only needs the Cargo skeleton to exist, so it can run in parallel
with all feature work — landing it early means the release path is exercised long
before it is needed rather than being discovered broken at the end.

**Blocked by:** 01.

**Status:** done
