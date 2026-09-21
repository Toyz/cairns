---
number: 15
title: Two install instructions that were not true
date: 2026-09-20
area: build, adoption
files: README.md
---

# 15. Two install instructions that were not true

The README said:

```sh
cargo install cairns                    # from crates.io
brew install Toyz/tap/cairns            # macOS and Linux
```

Neither works. `cairns` is not on crates.io - the publish job is deliberately
gated behind a repository variable nobody has set - and `Toyz/homebrew-tap`
does not exist. Both return 404. I wrote them while writing the release
plumbing, describing what the plumbing would make possible, and never came back
to check whether it had.

That is the same failure as everything else today: stating a thing is so
because I arranged for it to be possible, rather than because I watched it
happen. A false install line is worse than most, because the person who finds
out is a stranger who wanted to try the tool and now thinks it is broken.

The README now offers the two that were tested:

```sh
curl -fsSL .../install.sh | sh
cargo install --git https://github.com/Toyz/cairns cairns
```

The second was run before it was written down. It works, and it reports its
version as `0.0.0-dev`, because the repo never claims a release number - the
tag does, and CI applies it to the working tree only. The binary is right and
`--version` is uninformative, which is a real cost of that design and is now
written next to the command rather than left to be discovered.

**Still unknown:** whether a source install should be able to know its version
at all. `git describe` in a build script would do it, at the cost of a build
script and of behaving differently inside and outside a git checkout.
