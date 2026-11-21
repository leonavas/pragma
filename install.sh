#!/bin/sh
set -eu

REPO="${PRAGMA_REPO:-leonavas/pragma}"
INSTALL_DIR="${PRAGMA_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${PRAGMA_VERSION:-latest}"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS:$ARCH" in
  Linux:x86_64)        TARGET="x86_64-unknown-linux-gnu" ;;
  Linux:aarch64|Linux:arm64) TARGET="aarch64-unknown-linux-gnu" ;;
  Darwin:x86_64)       TARGET="x86_64-apple-darwin" ;;
  Darwin:arm64|Darwin:aarch64) TARGET="aarch64-apple-darwin" ;;
  *)
    echo "No prebuilt binary for $OS $ARCH."
    echo "Install from source with Cargo:"
    echo "  git clone https://github.com/$REPO && cd pragma && cargo install --path core/cli/crates/pragma-cli"
    exit 1
    ;;
esac

ARCHIVE="pragma-${TARGET}.tar.gz"
if [ "$VERSION" = "latest" ]; then
  BASE_URL="https://github.com/$REPO/releases/latest/download"
else
  case "$VERSION" in
    v*) TAG="$VERSION" ;;
    *)  TAG="v$VERSION" ;;
  esac
  BASE_URL="https://github.com/$REPO/releases/download/$TAG"
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

URL="$BASE_URL/$ARCHIVE"
echo "Downloading pragma ($TARGET, $VERSION)..."
if curl -fsSL "$URL" -o "$TMP_DIR/pragma.tar.gz" 2>/dev/null; then
  if curl -fsSL "$BASE_URL/SHA256SUMS.txt" -o "$TMP_DIR/SHA256SUMS.txt" 2>/dev/null; then
    EXPECTED="$(awk -v f="$ARCHIVE" '$2 == f || $2 == "*" f { print $1; exit }' "$TMP_DIR/SHA256SUMS.txt")"
    if [ -n "$EXPECTED" ]; then
      if command -v sha256sum >/dev/null 2>&1; then
        ACTUAL="$(sha256sum "$TMP_DIR/pragma.tar.gz" | awk '{ print $1 }')"
      elif command -v shasum >/dev/null 2>&1; then
        ACTUAL="$(shasum -a 256 "$TMP_DIR/pragma.tar.gz" | awk '{ print $1 }')"
      else
        ACTUAL=""
      fi
      if [ -n "$ACTUAL" ] && [ "$ACTUAL" != "$EXPECTED" ]; then
        echo "Checksum mismatch for $ARCHIVE (expected $EXPECTED, got $ACTUAL)" >&2
        exit 1
      fi
    fi
  fi
else
  echo "Failed to download $URL" >&2
  echo "See the releases at https://github.com/$REPO/releases" >&2
  exit 1
fi

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
