---
number: 7
title: Migrate, and the backup that never fired
date: 2026-09-20
area: cli, adoption
files: crates/cairns/src/main.rs
---

# 7. Migrate, and the backup that never fired

`migrate` splits a single-file worklog on its `## ` headings, takes the number
and title from each, demotes the sub-headings one level - the old format made
the entry title an `h2` and its sections `h3`, the new one makes the title the
`h1` - and writes one file per entry.

Amber's log is the real test: 9,727 lines, 192 entries, all numbered.

```
$ cairns migrate WORKLOG.md --date 2026-08-01
kept the original as .../WORKLOG.md.bak
192 entries -> worklog
192 entries -> WORKLOG.md
every entry is dated 2026-08-01 and filed under "port" - the old format
carried neither, so both want correcting by hand
4 entries have a "not done" section that should become a `**Still unknown:**`
trailer
```

Fidelity checks exactly: 7,329 non-blank prose lines in the original outside its
headings, 7,329 across the 192 entries, and the two lists are identical once the
`###` demotion is applied. Nothing was reflowed, reordered or dropped.

What cannot be recovered is what the old format never held. Every entry gets one
date and one area, which is a lie of uniformity rather than a loss - but it is
visible, it is reported, and it is correctable by hand.

## The backup never fired, and the original was destroyed

The first run printed no backup line and left this:

```
WORKLOG.md   9,727 lines  ->  202 lines
```

The migration wrote 192 entries correctly and then `index` regenerated
`WORKLOG.md` over the top of the file it had just read. The guard meant to
prevent that was:

```rust
if from == root.join(&config.paths.index) {
```

`from` is the path as typed - `WORKLOG.md`, relative. `root.join(...)` is
absolute. The comparison is *always* false, so the backup branch was dead code
and nothing said so. In the test clone the original came back out of git. In a
repo where the log had uncommitted edits, it would not have.

This is the same shape as the `init` bug in [4](0004-the-write-path-and-an-init-that-nearly-ate-hellbender-s.md):
a command whose job is to help a project adopt the tool, destroying the thing it
was adopting, and saying nothing. Twice now, which makes it a pattern rather than
an accident - both times the destructive path was the one no test covered,
because both were about a file the tool did not create.

The fix resolves both paths before anything is written, refuses if a `.bak` is
already there, and treats a failed copy as a reason to stop rather than a
warning to print on the way past.

## What 192 entries cost

```
build       0.095s
site        2.5 MB, 199 files
index.html  63 KB raw / 17 KB gzipped
search.json 468 KB raw / 159 KB gzipped
log.json    582 KB raw / 193 KB gzipped
```

That answers most of what [6](0006-the-log-reads-as-a-website-and-the-feed-is-not-the-whole-log.md)
left open about client-side search. Fetched once, lazily, over a host that
serves gzip, 159 KB is unremarkable. Linear growth puts a 500-entry log around
410 KB gzipped, which is where a real index rather than a string scan starts to
be worth it - so the answer is "fine, and the number to watch is 500".

**Still unknown:** whether per-entry dates can be recovered from git history -
each heading first appears in some commit, and that commit has a date. It would
turn 192 identical dates into something true, and nothing else can.
