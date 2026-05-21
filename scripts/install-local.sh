#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo was not found in PATH" >&2
  echo "Install Rust first: https://rustup.rs/" >&2
  exit 1
fi

cargo install --path . --force

echo "Installed v8q to ~/.cargo/bin/v8q"

case ":${PATH}:" in
  *":${HOME}/.cargo/bin:"*) ;;
  *)
    echo
    echo "warning: ~/.cargo/bin is not in PATH"
    echo 'Add this to your shell rc: export PATH="$HOME/.cargo/bin:$PATH"'
    echo "Then restart your shell or source the file."
    ;;
esac

if command -v v8q >/dev/null 2>&1; then
  v8q doctor || true
else
  "$HOME/.cargo/bin/v8q" doctor || true
fi
