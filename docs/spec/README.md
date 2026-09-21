---
title: The cairns spec
status: solid
worklog: 1
spec_version: 1
---

# The cairns spec

A worklog is a numbered, append-only sequence of markdown entries recording how
a thing was worked out - the evidence, the measurements, and above all the dead
ends, so the next person does not re-walk them.

This directory defines the format. The tool is one implementation of it. The
format is the part that has to survive, so it is written down separately and
versioned separately:

| document | defines |
| --- | --- |
| [entry.md](entry.md) | an entry file: filename, front matter, body conventions |
| [config.md](config.md) | `cairns.toml` - project identity, areas, paths |
| [log-json.md](log-json.md) | `log.json` - the canonical machine-readable export |
| [publish.md](publish.md) | publish targets and the delivery payload |

## Principles

**The markdown is the source of truth.** Everything else - the index, the site,
`log.json` - is generated and disposable. A worklog whose tooling is lost is
still a worklog. This is why there is no database in the design and why the
entries are readable without any of this.

**Generated files are never hand-edited.** `WORKLOG.md` is regenerated from the
entries; editing it is a bug that `cairns check` reports.

**Append-only in spirit.** Entries are not renumbered, rewritten or deleted. A
claim that turns out to be wrong is corrected by a later entry that says so and
links back with `supersedes`. The record of having been wrong is the point.

**One canonical payload.** The static site renders from `log.json`. Any future
hosted service ingests the same `log.json`. There is exactly one path from
markdown to structured data, so the two can never drift.

## Versioning

`spec_version` is an integer, currently `1`. It appears in `cairns.toml` and in
every `log.json`. A reader that understands version N must keep reading version
N logs forever; the version increments only for changes that would otherwise
break such a reader.

Additive changes - a new optional front matter field, a new derived field in
`log.json` - do not increment it. Consumers ignore fields they do not know.
