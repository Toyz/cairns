---
number: 17
title: An entry that says nothing about what it does not know
date: 2026-09-20
area: spec, core, cli
files: crates/cairns-core/src/entry.rs, crates/cairns-core/src/log.rs, crates/cairns/src/main.rs
---

# 17. An entry that says nothing about what it does not know

Hellbender's entry 60 asks whether `check` should refuse an entry with no
`**Still unknown:**` line at all, rather than accepting a log that quietly
stops collecting. It should, and the evidence in that entry settles it:
twenty-six consecutive entries had dropped the line, `cairns open` had been
reporting the project's state as of entry 33, and nothing complained for
months, because a missing convention is not a broken one.

`check` now reports two faults it used to pass over:

```
worklog/0034-x.md: has no `**Still unknown:**` line - write `nothing` to close it out
worklog/0035-y.md: `**Still unknown:**` is empty - write `nothing` to close it out
```

The blank one matters as much as the missing one, and it was the easier of the
two to get: `cairns new` writes the trailer into every entry it creates with
nothing after it. An entry nobody filled in and an entry that deliberately
closed out read identically to the collector, and only one of them meant it.

So the trailer has four states rather than two - missing, blank, closed,
open - and only the last two are things a person decided.

## Adoption does not fail on history

Turning this on would have failed hellbender before it was fixed, and fails
amber now: 192 migrated entries, none with a trailer, because the format they
came from had no such convention.

`init` writes the relaxation itself, the same way it freezes slugs:

```
$ cairns init
192 entries have no `**Still unknown:**` line - check relaxed to optional in cairns.toml
$ cairns check
ok
```

with a comment in `cairns.toml` saying to set it back once they carry one. The
principle is the one adoption has followed throughout: preserve what is there,
hold what is written from here on to the better standard. A tool that makes a
project fix its history before it can be used is a tool nobody adopts.

**Still unknown:** whether the same argument applies to `files`, which is
optional and which
[[16]] has only
just made visible. An entry about code with no `files` is as silently
incomplete as one with no open question - but unlike the trailer there is no
word for "this entry is not about any file".
