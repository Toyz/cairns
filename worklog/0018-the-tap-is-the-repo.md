---
number: 18
title: The tap is the repo
date: 2026-09-20
area: build
files: Formula/cairns.rb, scripts/update-formula.sh, .github/workflows/release.yml
---

# 18. The tap is the repo

A Homebrew tap does not need a repository of its own. `brew tap` takes an
explicit URL, and any repo with a `Formula/` directory in it is a tap:

```sh
brew tap Toyz/cairns https://github.com/Toyz/cairns
brew install cairns
```

The cost is that one command instead of `brew install Toyz/tap/cairns` - the
short form resolves `Toyz/cairns` to `Toyz/homebrew-cairns`, which is why a
dedicated tap is named that way. The gain is worth more than the keystrokes:
the release workflow updates the formula in the repo it is already checked out
in, with the token it already has. A separate tap needs a personal access token
with write access to another repository, kept in a secret, which is a thing to
create, store and eventually rotate.

Verified rather than assumed, because Homebrew 7 made two of my assumptions
wrong on the way. `brew audit <path>` is disabled, and so is installing a
formula from a path - "Homebrew requires formulae to be in a tap". So the test
was a real tap pointed at this working copy, a real `brew install`, and running
the binary it put on `PATH`:

```
$ brew tap Toyz/cairns "$(pwd)"
$ brew install Toyz/cairns/cairns
$ cairns --version
cairns 0.3.0
```

It also settled a detail I would otherwise have guessed at: each archive holds
one top-level directory, and Homebrew enters it before running `install`, so
the binary is `cairns` and not `cairns-v0.3.0-aarch64-apple-darwin/cairns`.

## The formula is generated, not edited

`scripts/update-formula.sh` writes the whole file from a tag and that release's
`SHA256SUMS`. Four values change per release - a version and three digests -
and a formula patched line by line drifts the moment a line moves. Today has
supplied five separate demonstrations of that, so this one regenerates.

The release workflow runs it after the binaries are built and commits the
result to `main`. A tag now updates the binaries, the checksums and the formula
together, which is the only way the three stay in agreement.

**Still unknown:** whether the generated formula should be checked by CI rather
than only by me running `brew install` once. `brew audit` wants a tap, and a
tap wants the formula pushed, so checking it before it is published is
awkward - which is exactly when a broken formula is cheapest to catch.
