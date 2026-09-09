#!/usr/bin/env bash
# Update the vibestation binary on Linux.
#
# Run this on the machine, from a clone of this repository:
#
#   ./scripts/install-linux.sh
#
# It pulls the latest commit, installs the checked-in static binary into
# ~/.local/bin (override with BIN_DIR=...) and checks that it runs.
#
# Pass --no-pull to install whatever the working tree already has.

set -euo pipefail

BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source_binary="$repo/dist/vibestation-x86_64-linux"
target="$BIN_DIR/vibestation"

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "This installs the Linux build; on a Mac use ./scripts/install-macos.sh." >&2
  exit 1
fi

if [[ "$(uname -m)" != "x86_64" ]]; then
  echo "dist/ only ships an x86_64 Linux build; on $(uname -m) build from source:" >&2
  echo "  cargo build --release && install -m755 target/release/vibestation $target" >&2
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
mv "$target.new" "$target"

echo "==> $("$target" --version)"

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) echo "note: $BIN_DIR is not on your PATH; add it to your shell profile." ;;
esac
