#!/usr/bin/env bash
# Update the vibestation binary on a Mac.
#
# Run this on the Mac, from a clone of this repository:
#
#   ./scripts/install-macos.sh
#
# It pulls the latest commit, installs the checked-in universal binary into
# ~/.local/bin (override with BIN_DIR=...), ad-hoc signs it so Apple Silicon
# will execute it, and checks that it runs.
#
# Pass --no-pull to install whatever the working tree already has.

set -euo pipefail

BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source_binary="$repo/dist/vibestation-macos"
target="$BIN_DIR/vibestation"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This installs the macOS build; on Linux use dist/vibestation-x86_64-linux." >&2
  exit 1
fi

if [[ "${1:-}" != "--no-pull" ]]; then
  echo "==> Pulling $repo"
  git -C "$repo" pull --ff-only
fi

[[ -f "$source_binary" ]] || { echo "No binary at $source_binary" >&2; exit 1; }

echo "==> Installing to $target"
mkdir -p "$BIN_DIR"
# Replace by rename so a running copy is never written into.
cp "$source_binary" "$target.new"
chmod +x "$target.new"

# Cleared in case the file ever arrived via a browser or AirDrop; harmless otherwise.
xattr -d com.apple.quarantine "$target.new" 2>/dev/null || true

# Apple Silicon refuses to run an unsigned binary, and this one is cross-built
# on Linux, so it arrives without even an ad-hoc signature.
if command -v codesign >/dev/null; then
  codesign --force --sign - "$target.new"
else
  echo "!! codesign not found — install the Xcode command line tools with" >&2
  echo "   xcode-select --install if the binary is killed on launch." >&2
fi

mv "$target.new" "$target"

echo "==> $("$target" --version)"

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) echo "note: $BIN_DIR is not on your PATH; add it to your shell profile." ;;
esac
