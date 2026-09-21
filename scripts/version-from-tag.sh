#!/bin/sh
# Rewrite the workspace version from a tag, in the working tree only.
#
#   scripts/version-from-tag.sh v0.2.0
#
# The repo's Cargo.toml says 0.0.0-dev and never says anything else. A release
# is a tag; this turns the tag into the version CI builds and publishes with,
# and nothing is committed - so there is no file to bump by hand and no way for
# a tag and a manifest to disagree about what a release is.
set -eu

tag="${1:-}"
case "$tag" in
  v[0-9]*.[0-9]*.[0-9]*) ;;
  *) printf 'expected a vX.Y.Z tag, got %s\n' "${tag:-<nothing>}" >&2; exit 1 ;;
esac
version="${tag#v}"

# Replace the exact current version string, which appears as the workspace
# version and as the pin on each path dependency - and nowhere else, so a
# dependency on "1" or "0.8" is left alone.
current=$(grep -m1 '^version = ' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
[ -n "$current" ] || { echo "no version in Cargo.toml" >&2; exit 1; }

sed -i.bak "s/version = \"$current\"/version = \"$version\"/g" Cargo.toml
rm -f Cargo.toml.bak

# Only the workspace members move; every other pin in the lockfile stays put,
# so `--locked` still means what it meant.
cargo update --workspace --quiet

printf '%s -> %s\n' "$current" "$version"
