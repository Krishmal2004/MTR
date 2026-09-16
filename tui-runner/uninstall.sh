#!/usr/bin/env bash
set -euo pipefail

BIN_NAME="tui-runner"
INSTALL_DIR="${TUI_RUNNER_INSTALL_DIR:-$HOME/.local/bin}"
TARGET="$INSTALL_DIR/$BIN_NAME"

if [ -f "$TARGET" ]; then
  rm -f "$TARGET"
  echo "Removed $TARGET"
else
  echo "Nothing to remove at $TARGET (already uninstalled?)"
fi
