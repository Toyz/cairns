---
number: 4
title: The write path, and an init that nearly ate hellbender's skill
date: 2026-09-20
area: cli, skill, adoption
files: crates/cairns/src/main.rs, crates/cairns/templates/SKILL.md, crates/cairns-core/src/date.rs
---

# 4. The write path, and an init that nearly ate hellbender's skill

`new`, `init` and the skill template close the write path. `new` refuses an area
`cairns.toml` does not declare - naming the ones it does, since the useful part
of that error is the list - and regenerates the index on the way out, so the
index is never stale in the window between creating an entry and remembering to
run `index`.

This entry was created with it.

## Dates, in two places for one reason

`cairns-core` computes a civil date from a Unix timestamp with Howard Hinnant's
`civil_from_days`, about ten lines, which keeps the crate dependency-free and
`wasm32`-clean. The binary does not use it for `new`: it asks `jiff` for the
*local* date, because an entry written at eleven at night should not be dated
tomorrow, and getting a local date right means timezone data the core has no
business carrying.

The arithmetic was tested against the epoch, a leap day, and an arbitrary
timestamp. The arbitrary one failed - and the code was right, the expectation
was wrong. Worth recording only because the reflex on a red test is to look at
the code, and the leap day passing first time was the signal that the
conversion was sound.

## init froze hellbender's slugs, exactly as entry 2 asked

Adoption on a clean clone, with its fifteen areas declared:

```
$ cairns check          # before
... 13 problems
$ cairns init
froze 13 slugs - these entries keep the names they were published under
$ cairns check          # after
ok
```

Each of the thirteen gained a `slug:` line pinning the name it was published
under, so the two entries whose titles had drifted and the eleven cut mid-word
by the old 60-character rule all keep their URLs. New entries get the better
derivation. Adoption preserves rather than improves, which is the only way it
can be safe.

## It also destroyed the thing it was adopting

The first working `init` rewrote `.claude/skills/worklog/SKILL.md` - 58 lines
deleted - and hellbender's skill is not a generated file. It carries the rules
that make that log what it is: every claim locatable by a VA against
`HELLBEND.EXE` at `0x00400000`, a format entry unfinished until the matching
`docs/formats/` page exists. All of it gone, in a command whose entire purpose
is to help a project adopt the tool.

Worse, it announced the opposite:

```
wrote .claude/skills/worklog/SKILL.md (project section preserved)
```

The flag behind that message was "a file was already here", not "its project
section survived". Two separate failures - a destructive default, and a message
asserting the thing that had just not happened - and the second is the one that
would have let it go unnoticed, because the output read like success.

`init` now leaves an existing skill alone unless it carries the
`<!-- cairns:project -->` marker, and says why:

```
kept .claude/skills/worklog/SKILL.md - it has no <!-- cairns:project --> marker.
Everything below that marker is what init preserves, so put it above the parts
this project wrote and run init again.
```

With the marker added, regeneration rewrites the generic half from the template
with the project's own areas in it and returns the project section verbatim.
Both paths are now checked against a fresh hellbender clone.

The general rule this is an instance of: a command that writes into a repo it
did not create must default to refusing, not to overwriting, and must never
describe what it wishes it had done.

**Still unknown:** whether the marker is discoverable enough. A project that
never adds it never gets the generic half updated, and nothing will remind
them - `check` does not currently look at the skill at all.
