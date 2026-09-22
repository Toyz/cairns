---
number: 20
title: One plus-minus sign crashed the scanner
date: 2026-09-21
area: core
files: crates/cairns-core/src/entry.rs
---

# 20. One plus-minus sign crashed the scanner

```
thread 'main' panicked at crates/cairns-core/src/entry.rs:490:32:
start byte index 1 is not a char boundary; it is inside '±' (bytes 0..2 of string)
```

The reference scanner from [[19]] walks each line looking for `[[`, and where
a character is not one it advances with `at += 1`. That is a byte, not a
character. `±` is two bytes, so the next `line[at..]` started inside it and
Rust refused - correctly, and fatally.

It steps by `ch.len_utf8()` now. Hellbender's log contains eight `±`, amber's
contains eleven em dashes, and both were enough; hellbender's is the log that
found it.

## What the tests missed and why

There were six tests on this scanner, covering references, labels, fenced
code, inline code and dangling numbers. Every one of them was written in
ASCII, because I wrote them, and I was thinking about the syntax rather than
about the text it sits in. A worklog is prose about measurements - `±`, `—`,
`µs`, `°` - and the one thing the scanner is guaranteed to meet is a character
I did not type into a test.

The new tests run the scanner over `±`, an em dash, an emoji, `±[[3]]±`, and a
fence containing `±`, and assert the surrounding text survives the rewrite
intact rather than merely that nothing panicked.

It is also worth noticing what caught it: not `check`, not the tests, not the
build. Someone ran it on a real log. That is the fourth time in two days that
the thing which found the fault was the tool meeting real content rather than
anything I ran against content I had invented.

## The rest of the codebase, checked

The same habit of mind wrote the front matter splitter, the slug derivation,
the summary extractor and the trailer parser, so all of them were read again.
They are safe, and for a reason worth recording rather than by luck:

- every one of them slices at an index found by searching for an **ASCII**
  delimiter - `\n`, `---`, `. `, `/`, `#`, `**Still unknown:**` - and a byte
  index found that way is always on a character boundary
- `slugify` truncates its own output, which is ASCII by construction: it emits
  only `a-z`, `0-9` and `-`
- the one other `at += 1` in the tree walks a `Vec<Event>`, not a string

So the scanner was the only place advancing a byte at a time through text it
had not built. That is the distinction to watch for: searching for ASCII and
slicing there is fine, and stepping through arbitrary text is not.

**Still unknown:** nothing.
