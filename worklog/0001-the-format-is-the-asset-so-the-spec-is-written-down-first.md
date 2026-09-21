---
number: 1
title: The format is the asset, so the spec is written down first
date: 2026-09-20
area: spec, core
files: docs/spec/, crates/cairns-core/src/
summary: The worklog format is extracted from hellbender as its own project, with the spec written before the tool so the format can outlive this implementation.
---

# 1. The format is the asset, so the spec is written down first

The worklog format grew up inside
[hellbender](https://github.com/Toyz/hellbender) as `tools/worklog.py` plus a
skill, and it works - 55 entries, a generated index, numbering that holds. What
it could not do is leave that repo. Three things were tangled in the one skill
file: the format, the tool, and fifteen hard-coded areas, several of them
specific to one 1996 game. Only the first of those is worth sharing, so this
project separates them and writes the format down as a spec under `docs/spec/`
before implementing any of it.

The tool is one implementation. A worklog whose tooling is lost is still a
worklog, which is the whole reason the markdown stays the source of truth and
nothing here needs a database.

## What got decided

**Rust, one binary.** The original is 200 lines of Python and the port buys
nothing on its own; it buys distribution. Anyone handed a binary runs it, and
the core compiles to `wasm32` with `--no-default-features`, which is what keeps
a future hosted version a deployment rather than a rewrite.

**The static site renders from `log.json`, not from the markdown.** One path
from entries to structured data, so a second reader can never disagree with the
first about something subtle in an output nobody is looking at. It also means
an ingest contract can be designed against real payloads before any server
exists - `publish --target hosted --dry-run` prints the request body today.

**Front matter is not YAML.** It is a strict `key: value` subset, defined in
[entry.md](../docs/spec/entry.md), with no quoting, nesting, blocks or comments.
A real YAML parser reads it correctly, but that is a convenience rather than a
promise. The subset has exactly one reading and cannot grow ambiguity, which
matters more for a file format meant to be legible in twenty years than
nesting nobody needs.

**An entry's id is `{project}/{number}`, not the number.** Costs one line of
config now; retrofitting it invalidates every URL already handed out the moment
two projects sit in one place.

**`content_hash` is the SHA-256 of the file's bytes**, so it verifies with
`shasum -a 256` and no knowledge of this format at all. It is what lets a
publish send only what changed, and it works identically for a local directory
and for an ingest endpoint that does not exist yet.

**No secrets in `cairns.toml`.** A `token` must be an environment variable
reference beginning with `$`, and `check` fails on a literal - catching the
leak in the commit that introduces it rather than after the push.

## What is built

`cairns-core` parses and validates entries and builds the canonical document.
`cairns-site` renders a payload from it, which today is `log.json` alone.
The command surface is settled - `new`, `next`, `index`, `check`, `open`,
`build`, `export`, `publish`, `init`, `migrate` - and the read half of it
works. The write half is the parity port, and it is not built yet.

**Still unknown:** whether the derived first-sentence summary is good enough in
practice, or whether entries will need `summary:` written by hand often enough
that the tool should prompt for it.
