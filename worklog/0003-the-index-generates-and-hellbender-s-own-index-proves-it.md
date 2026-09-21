---
number: 3
title: The index generates, and hellbender's own index proves it
date: 2026-09-20
area: cli, site, adoption
files: crates/cairns-site/src/lib.rs, crates/cairns/src/main.rs
summary: cairns index regenerates WORKLOG.md, and rendering hellbender's 55 entries reproduces its committed index byte for byte apart from the documented area-spacing change.
---

# 3. The index generates, and hellbender's own index proves it

`index` was parked with the rest of the write path, which left a hole nobody
had looked at: `check` passed on this repo while it had no `WORKLOG.md` at all.
The Python tool would have failed that - comparing the index against a fresh
render is one of the four things its `check` did. A check that is quiet about
the artifact most likely to be wrong is worse than no check, because it is
believed.

So the index renders now, and `check` compares:

```
$ cairns check
WORKLOG.md is stale - run `cairns index`
```

Missing and stale are reported separately. They have different causes - one is
a repo that never ran the command, the other is entries edited since it last
did - and a reader who sees the right one does not have to work out which.

## The header is the only part a project writes

Everything in the index is derived except its opening prose, which is
`[index] header` in `cairns.toml`. The generated default names the project and
says the file is generated; a project that wants to explain what its log is
*for* says it better than any generated sentence.

The area tally is sorted alphabetically rather than in declared order. The
areas themselves are presented in declared order everywhere else, because that
ordering is a taxonomy the project chose - but a tally is a lookup, and a
lookup reads better sorted by the name you are looking for.

## The parity test

The acceptance test for the port was always going to be hellbender's own index:
render its 55 entries and diff against the file in the repo. With its header
copied into `[index] header`:

```
$ diff committed.md WORKLOG.md | grep -c '^<'
27
$ sed 's/,\([a-z]\)/, \1/g' committed.md | diff - WORKLOG.md | grep -c '^[<>]'
0
```

27 of 55 rows differ, and every one of them differs only in the area column,
where `decomp,engine` becomes `decomp, engine`. Normalise that one spelling and
the two files are byte-identical: same header, same tally, same 55 rows, same
titles, dates, paths and pipe escaping.

That is the whole intended delta. The log had accumulated both spellings
because the old index printed whichever string the entry happened to carry, and
normalising on write is the documented fix. Worth knowing precisely, though,
because it means adopting cairns costs hellbender exactly one reflow commit on
one column - not a migration.

**Still unknown:** whether any other project's index has drifted in ways this
test would not catch, since hellbender is the only real log to test against
until amber is migrated.
