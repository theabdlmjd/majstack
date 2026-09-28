#!/usr/bin/env sh
set -eu

REPO="${MAJSTACK_REPO:-theabdlmjd/majstack}"
VERSION="${MAJSTACK_VERSION:-latest}"
BIN_DIR="${MAJSTACK_BIN_DIR:-$HOME/.local/bin}"

OS="$(uname -s)"
ARCH="$(uname -m)"
PLATFORM=""
case "$OS" in
  Linux) PLATFORM="x86_64-unknown-linux-gnu" ;;
  Darwin)
    if [ "$ARCH" = "arm64" ]; then PLATFORM="aarch64-apple-darwin"; else PLATFORM="x86_64-apple-darwin"; fi
    ;;
esac

if [ -n "$PLATFORM" ]; then
  if [ "$VERSION" = "latest" ]; then
    URL="https://github.com/$REPO/releases/latest/download/majstack-$PLATFORM.tar.gz"
  else
    URL="https://github.com/$REPO/releases/download/$VERSION/majstack-$PLATFORM.tar.gz"
  fi
  TMP="$(mktemp -d)"
  if curl -fsSL "$URL" -o "$TMP/majstack.tar.gz" 2>/dev/null; then
    tar -xzf "$TMP/majstack.tar.gz" -C "$TMP"
    mkdir -p "$BIN_DIR"
    install -m 0755 "$TMP/majstack" "$BIN_DIR/majstack"
    install -m 0755 "$TMP/majstackd" "$BIN_DIR/majstackd"
    echo "installed majstack to $BIN_DIR"
    case ":$PATH:" in
      *":$BIN_DIR:"*) ;;
      *) echo "add $BIN_DIR to your PATH: export PATH=\"$BIN_DIR:\$PATH\"" ;;
    esac
    exit 0
  fi
fi

if command -v cargo >/dev/null 2>&1; then
  cargo install --git "https://github.com/$REPO" majstack-cli majstack-daemon
  exit 0
fi

echo "Could not download a release and cargo is not installed." >&2
echo "Install Rust from https://rustup.rs or download from https://github.com/$REPO/releases" >&2
exit 1
