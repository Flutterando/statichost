#!/usr/bin/env sh
set -e

HOST="{{HOST}}"
INSTALL_DIR="${STATICHOST_INSTALL_DIR:-/usr/local/bin}"

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
case "$OS" in
  linux)  OS=linux ;;
  darwin) OS=darwin ;;
  *) echo "unsupported OS: $OS" >&2; exit 1 ;;
esac

ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64) ARCH=amd64 ;;
  aarch64|arm64) ARCH=arm64 ;;
  *) echo "unsupported arch: $ARCH" >&2; exit 1 ;;
esac

BIN="statichost-${OS}-${ARCH}"
URL="${HOST}/dl/${BIN}"
TMP="$(mktemp -t statichost.XXXXXX)"

echo "Downloading ${URL}"
if command -v curl >/dev/null 2>&1; then
  curl -fSL "$URL" -o "$TMP"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$TMP" "$URL"
else
  echo "need curl or wget" >&2
  exit 1
fi

chmod +x "$TMP"

if [ -w "$INSTALL_DIR" ]; then
  mv "$TMP" "$INSTALL_DIR/statichost"
else
  echo "Installing to $INSTALL_DIR (sudo)"
  sudo mv "$TMP" "$INSTALL_DIR/statichost"
fi

echo "Installed: $($INSTALL_DIR/statichost --version 2>/dev/null || echo statichost)"
echo "Run: statichost login --host ${HOST} --token <your-token>"
