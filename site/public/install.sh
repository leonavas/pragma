#!/bin/sh
set -eu

ORIGIN="${PRAGMA_ORIGIN:-https://pragma.leonavas.workers.dev}"
INSTALL_DIR="${PRAGMA_INSTALL_DIR:-$HOME/.local/bin}"

OS="$(uname -s)"
ARCH="$(uname -m)"

if [ "$OS" = "Linux" ] && [ "$ARCH" = "x86_64" ]; then
  URL="$ORIGIN/downloads/pragma-linux-x64.tar.gz"
  TMP_DIR="$(mktemp -d)"
  trap 'rm -rf "$TMP_DIR"' EXIT
  echo "Downloading pragma (linux-x64) from $URL..."
  curl -fsSL "$URL" -o "$TMP_DIR/pragma.tar.gz"
  tar -xzf "$TMP_DIR/pragma.tar.gz" -C "$TMP_DIR"
  mkdir -p "$INSTALL_DIR"
  mv "$TMP_DIR/pragma" "$INSTALL_DIR/pragma"
  chmod +x "$INSTALL_DIR/pragma"
  echo "Installed pragma to $INSTALL_DIR/pragma"
  case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) echo "Add to your PATH: export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
  esac
  "$INSTALL_DIR/pragma" -V
else
  echo "Prebuilt binary is currently available for Linux x86_64 (detected $OS $ARCH)."
  echo "Install from source with Cargo:"
  echo "  git clone https://github.com/leonavas/pragma && cd pragma && cargo install --path core/cli/crates/pragma-cli"
  exit 1
fi
