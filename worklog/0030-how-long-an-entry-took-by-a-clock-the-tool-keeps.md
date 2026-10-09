---
number: 30
title: How long an entry took, by a clock the tool keeps
date: 2026-10-08
area: spec, cli, site
files: crates/cairns/src/clock.rs, crates/cairns/src/main.rs, crates/cairns-core/src/entry.rs, crates/cairns-core/src/log.rs, crates/cairns-site/src/html.rs, docs/spec/entry.md, crates/cairns/templates/SKILL.md
took: 1h 10m
---

# 30. How long an entry took, by a clock the tool keeps

An entry can now say how long the work took. The point is the cost of a dead
end - "this took four hours and went nowhere" is the most useful sentence one
can carry - and, across a log, which areas the time goes to.

## The tool keeps the clock

The writer is usually a model, and a model has no sense of elapsed time: asked
how long something took, it gives a confident number that is wrong. So the
model is never asked.

```sh
cairns start "what you are about to find out"   # stamps .cairns/clock
cairns new ... --body -                          # records started: and took:, stops the clock
cairns clock                                     # what is running
cairns start --cancel                            # drop it
```

`.cairns/` holds a `.gitignore` of `*`, so a running clock is never committed
and no project's own `.gitignore` has to know about it. The clock is read
before the entry is written and stopped after, so a refused entry leaves it
running.

```
started: 2026-10-08T14:25:39-07:00
took: 1h 23m
```

## What it measures

Wall-clock, start to entry, every break included. The spec says exactly that
and the field claims nothing more. Past eight hours `new` says so on stderr,
since a number that long almost certainly spans a break; `--took` gives the
time instead of the clock, and is the honest answer for a session that never
was clocked. A duration is hours then minutes, each at most once: `90` alone is
refused, because summed across a log a number that reads two ways is wrong
half the time. `a_duration_reads_one_way_only`.

An entry with no `took:` did not record one. Totals - a day's under its
heading, the log's in the panel beside the list - count only entries that did,
never an unrecorded one as zero.

## A skill of its own

Telling the model to start the clock in the worklog skill would not work, and
the reason is how skills are chosen: by their description. The worklog skill's
says "use whenever a unit of work finishes" - it is matched at the end of the
work, which is too late to start a clock. So `init` writes a third skill,
`clock`, whose description says to use it "when beginning any piece of work...
before investigating or editing anything". It is written in every project,
like the worklog skill; the reference skill is still only for projects with
`[docs]`. `the_clock_has_a_skill_that_triggers_at_the_start`.

This entry was not clocked from the start of the work - the clock was started
after - so its own `took:` is given with `--took`, as an estimate from the
session, which is exactly the case the flag is for.

**Still unknown:** whether models actually run cairns start when the skill tells them to - an entry with no took: is indistinguishable from one where nobody started the clock, and only a few weeks of use will say how many are which
