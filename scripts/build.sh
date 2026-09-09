#!/usr/bin/env bash
# Build the shippable binaries into dist/.
#
#   ./scripts/build.sh
#
# Checks the tree the way CI does, then cross-builds both targets:
#
#   dist/vibestation-x86_64-linux   static musl, any glibc
#   dist/vibestation-macos          universal2, Intel and Apple Silicon
#
# Commit dist/ afterwards to ship; on the Mac, scripts/install-macos.sh
# installs from the clone. The pre-commit hook in .githooks/ runs this for you.
#
# Each binary is stamped with a build number — the commit count — so
# `vibestation --version` says which commit it was built from.
#
# Pass --quick to skip the fmt/clippy/test gate.
# Set BUILD_NUMBER to override the stamp; the hook does, since the commit it
# is building for does not exist yet.

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo"

# Neither the rust toolchain nor zig — the macOS link step's linker — is
# necessarily on PATH in a non-login shell, so look where each installs itself.
[[ -d "$HOME/.cargo/bin" ]] && PATH="$HOME/.cargo/bin:$PATH"
if ! command -v zig >/dev/null; then
  for dir in "$HOME"/.zig-*; do
    if [[ -x "$dir/zig" ]]; then
      PATH="$dir:$PATH"
      break
    fi
  done
fi
for tool in cargo cargo-zigbuild zig; do
  command -v "$tool" >/dev/null || {
    echo "$tool not found; the macOS build needs zig and cargo-zigbuild." >&2
    # 127 so the pre-commit hook can tell "no toolchain here" from "build broke"
    # and let the commit through rather than blocking it.
    exit 127
  }
done

# The package version lives only in Cargo.toml; cargo reports it as
# `path+file:///...#0.1.0`, or `...#name@0.1.0` when the directory is named
# differently, so take whatever follows the last # or @.
pkgid="$(cargo pkgid)"
export VIBESTATION_VERSION="${pkgid##*[#@]} (build ${BUILD_NUMBER:-$(git rev-list --count HEAD)})"
echo "==> vibestation $VIBESTATION_VERSION"

if [[ "${1:-}" != "--quick" ]]; then
  echo "==> Checking"
  cargo fmt --all --check
  cargo clippy --all-targets -- -D warnings
  cargo test --quiet
fi

echo "==> Building x86_64-unknown-linux-musl"
cargo build --release --target x86_64-unknown-linux-musl
cp target/x86_64-unknown-linux-musl/release/vibestation dist/vibestation-x86_64-linux

echo "==> Building universal2-apple-darwin"
cargo zigbuild --release --target universal2-apple-darwin
cp target/universal2-apple-darwin/release/vibestation dist/vibestation-macos

# The Linux build is the only one this machine can run, so it is the only one
# that gets more than a size check.
echo "==> $(./dist/vibestation-x86_64-linux --version)"
ls -lh dist/vibestation-x86_64-linux dist/vibestation-macos | awk '{print "    " $9 "  " $5}'
