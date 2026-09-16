#!/usr/bin/env bash
set -euo pipefail

REPO="Krishmal2004/MTR"
BIN_NAME="tui-runner"
INSTALL_DIR="${TUI_RUNNER_INSTALL_DIR:-$HOME/.local/bin}"

os="$(uname -s)"
case "$os" in
  Linux)
    asset="tui-runner-linux-x86_64"
    ;;
  Darwin)
    asset="tui-runner-macos-universal"
    ;;
  *)
    echo "Unsupported OS: $os" >&2
    exit 1
    ;;
esac

url="https://github.com/${REPO}/releases/latest/download/${asset}"

echo "Downloading ${asset} from latest release..."
mkdir -p "$INSTALL_DIR"
tmp_file="$(mktemp)"
curl -fsSL "$url" -o "$tmp_file"
chmod +x "$tmp_file"
mv "$tmp_file" "$INSTALL_DIR/$BIN_NAME"

echo "Installed to $INSTALL_DIR/$BIN_NAME"

case ":$PATH:" in
  *":$INSTALL_DIR:"*)
    ;;
  *)
    echo
    echo "$INSTALL_DIR is not on your PATH. Add this to your shell profile:"
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    ;;
esac

echo "Run it with: $BIN_NAME"
