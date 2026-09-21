---
title: log.json
status: draft
spec_version: 1
---

# `log.json`

The whole worklog as one structured document. It is generated, never written by
hand, and it is the only path from markdown to structured data:

```
worklog/*.md  ->  log.json  ->  the static site
                          \->  anything else, later
```

The static site is a *consumer* of `log.json`, not a second reader of the
markdown. That is deliberate. The moment a second code path parses entries
directly, the two disagree about something subtle and the bug appears only in
whichever output nobody was looking at.

## Shape

```json
{
  "spec_version": 1,
  "generator": "cairns 0.1.0",
  "generated": "2026-09-20T23:40:00Z",

  "project": {
    "name": "Hellbender",
    "slug": "hellbender",
    "description": "Reverse engineering Hellbender ...",
    "base_url": "https://toyz.github.io/hellbender/"
  },

  "areas": [
    { "name": "decomp", "about": "facts pulled out of HELLBEND.EXE itself", "count": 30 }
  ],

  "entries": [
    {
      "id": "hellbender/50",
      "number": 50,
      "slug": "kreash-mix-is-the-end-of-a-table",
      "url": "https://toyz.github.io/hellbender/50-kreash-mix-is-the-end-of-a-table",
      "path": "worklog/0050-kreash-mix-is-the-end-of-a-table.md",
      "title": "KREASH.MIX is the end of a table",
      "date": "2026-09-20",
      "areas": ["decomp", "format"],
      "files": ["docs/formats/colour-tables.md"],
      "summary": "Every per-level .MIX in GAME.POD is zero bytes except KREASH.MIX ...",
      "supersedes": [6],
      "superseded_by": [],
      "resolves": [],
      "resolved_by": [],
      "still_unknown": "the span loop that would prove it has not been found yet",
      "body": "The colour tables page had three unknowns ...",
      "content_hash": "sha256:9f2a...",
      "extra": {}
    }
  ],

  "open_questions": [
    { "entry": 50, "text": "the span loop that would prove it has not been found yet" }
  ],

  "readme": "# Hellbender\n\nReverse engineering ..."
}
```

`entries` is ordered by `number`, ascending, always.

## The fields that are not just copied through

**`body` is markdown, not HTML.** The payload stays renderer-agnostic, so a
newer renderer can re-render an old log, and a consumer that wants plain text or
a feed excerpt is not unpicking someone else's HTML to get it.

**`superseded_by` is derived** by inverting every `supersedes` in the log. This
is the field the site uses to tell a reader standing on entry 6 that entry 50
later overturned it. Deriving rather than storing it keeps the correction
recorded in exactly one place - the later entry, which is the only one that
could have known.

**`resolved_by` is derived** the same way, by inverting every `resolves`. While
it is non-empty the entry's question is closed and is absent from
`open_questions`, so the list is what the project does *not yet* know rather
than everything it has ever wondered.

**`still_unknown` and `open_questions`.** The former is the entry's own trailer,
parsed out; the latter is every unresolved one collected in entry order, which
is the log answering "what does this project still not know" without anyone
maintaining a separate list. Entries whose trailer is `nothing` are absent from
both.

**`content_hash` is `sha256:` plus the hex digest of the entry file's bytes**,
exactly as on disk, front matter included. Verifiable with `shasum -a 256`
against the file, with no knowledge of this format at all. It is what lets a
publish send only what changed, and what lets any future ingest dedupe without
diffing prose.

**`readme`** is the project's README as markdown, when `site.readme` names one.
It is carried in the document rather than read by the renderer, so the site is
still built from this file alone - and so anything ingesting a log gets the
project's own description of itself along with its entries.

**`extra`** holds any front matter key the spec does not define, passed through
untouched, so a project can carry its own metadata without forking the format
and without the tool needing to know what it means.

## Determinism

Two runs over unchanged entries produce byte-identical output but for
`generated`. `cairns export --reproducible` omits `generated` entirely, so CI
can assert the payload has not changed without a timestamp defeating it.
