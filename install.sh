#!/bin/sh
# Install cairns from a GitHub release.
#
#   curl -fsSL https://raw.githubusercontent.com/Toyz/cairns/main/install.sh | sh
#
# Environment:
#   CAIRNS_VERSION   a tag such as v0.1.0. Default: the latest release.
#   CAIRNS_BIN_DIR   where to put the binary. Default: ~/.local/bin.
#
# The download is checked against the release's SHA256SUMS before anything is
# installed. A failure at any step leaves nothing behind.

set -eu

REPO="Toyz/cairns"
BIN_DIR="${CAIRNS_BIN_DIR:-$HOME/.local/bin}"

die() { printf 'install: %s\n' "$1" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "$1 is required"; }

need uname
need mkdir
need tar
if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL "$1" -o "$2"; }
  read_url() { curl -fsSL "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -qO "$2" "$1"; }
  read_url() { wget -qO- "$1"; }
else
  die "curl or wget is required"
fi

target() {
  os=$(uname -s)
  arch=$(uname -m)
  case "$os" in
    Darwin)
      case "$arch" in
        arm64|aarch64) echo "aarch64-apple-darwin" ;;
        x86_64)        echo "x86_64-apple-darwin" ;;
        *) die "no macOS build for $arch" ;;
      esac
      ;;
    Linux)
      # A musl build runs anywhere; the gnu one is preferred where glibc is
      # present because it is the smaller surface to get wrong.
      case "$arch" in
        x86_64|amd64)
          if [ -f /etc/alpine-release ] || ! ldd --version >/dev/null 2>&1; then
            echo "x86_64-unknown-linux-musl"
          else
            echo "x86_64-unknown-linux-gnu"
          fi
          ;;
        *) die "no Linux build for $arch - build from source with: cargo install cairns" ;;
      esac
      ;;
    *) die "$os is not supported by this script - try: cargo install cairns" ;;
  esac
}

version() {
  if [ -n "${CAIRNS_VERSION:-}" ]; then
    echo "$CAIRNS_VERSION"
    return
  fi
  tag=$(read_url "https://api.github.com/repos/$REPO/releases/latest" \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
    | head -n1)
  [ -n "$tag" ] || die "could not find the latest release - set CAIRNS_VERSION"
  echo "$tag"
}

TARGET=$(target)
VERSION=$(version)
NAME="cairns-$VERSION-$TARGET"
BASE="https://github.com/$REPO/releases/download/$VERSION"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT INT TERM

printf 'cairns %s for %s\n' "$VERSION" "$TARGET"

fetch "$BASE/$NAME.tar.gz" "$TMP/$NAME.tar.gz" || die "could not download $NAME.tar.gz"
fetch "$BASE/SHA256SUMS" "$TMP/SHA256SUMS" || die "could not download SHA256SUMS"

# Verify before unpacking, not after.
if command -v sha256sum >/dev/null 2>&1; then
  got=$(sha256sum "$TMP/$NAME.tar.gz" | cut -d' ' -f1)
elif command -v shasum >/dev/null 2>&1; then
  got=$(shasum -a 256 "$TMP/$NAME.tar.gz" | cut -d' ' -f1)
else
  die "sha256sum or shasum is required to check the download"
fi
want=$(grep " $NAME.tar.gz\$" "$TMP/SHA256SUMS" | cut -d' ' -f1 | head -n1)
[ -n "$want" ] || die "$NAME.tar.gz is not listed in SHA256SUMS"
[ "$got" = "$want" ] || die "checksum mismatch - expected $want, got $got"

tar -xzf "$TMP/$NAME.tar.gz" -C "$TMP"
[ -f "$TMP/$NAME/cairns" ] || die "the archive did not contain cairns"

mkdir -p "$BIN_DIR"
install -m 755 "$TMP/$NAME/cairns" "$BIN_DIR/cairns" 2>/dev/null \
  || { cp "$TMP/$NAME/cairns" "$BIN_DIR/cairns" && chmod 755 "$BIN_DIR/cairns"; }

printf 'installed %s/cairns\n' "$BIN_DIR"
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) printf '\n%s is not on your PATH. Add it:\n    export PATH="%s:$PATH"\n' "$BIN_DIR" "$BIN_DIR" ;;
esac
