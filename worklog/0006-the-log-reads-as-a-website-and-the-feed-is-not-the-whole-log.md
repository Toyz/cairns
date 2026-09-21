---
number: 6
title: The log reads as a website, and the feed is not the whole log
date: 2026-09-20
area: site, publish, cli
files: crates/cairns-site/src/html.rs, crates/cairns-site/src/feed.rs, crates/cairns/src/main.rs
---

# 6. The log reads as a website, and the feed is not the whole log

`cairns build` renders the payload: an entry page each with prev/next and clean
URLs, an index with area chips and a search box, an open-questions page, an
Atom feed, a search index, and the `log.json` all of it was built from. The
renderer consumes that document and nothing else - it never opens an entry
file - which is what keeps a publish target and a local build the same code
path.

Against hellbender's 55 entries: 57 pages, all of which parse with no unclosed
or stray tags, 74 code blocks and 143 sub-headings rendered, 33 open questions
collected, 1.2 MB total.

## The correction notice works, once there is a correction to show

Hellbender has no `supersedes` edges, so the feature had nothing to render.
Backfilling the one entry 2 identified - `supersedes: 6` on entry 50, which
overturns what entry 6 said about `KREASH.MIX` - produced both halves:

```
entry 6   Revisited later. Something claimed here was corrected by
          KREASH.MIX is the end of a table.
entry 50  This entry revisits Raw images have no header, and the filename
          carries the video mode.
```

A reader landing on entry 6 from a search is now told, above the prose, that
part of it is wrong. That is the entire argument for an append-only log being
safe to read, and it costs one line of front matter per correction.

## Two things the real log changed

**The feed was 263 KB.** Carrying all 55 entries at full content makes a feed
that no reader wants and that is slower to fetch than the site. It is capped at
the newest 25 now, with a `rel="alternate"` link to `log.json` for anything that
wants all of it. The distinction is worth stating plainly, because the spec had
been sloppy about it: the feed is for *reading*, `log.json` is the complete
document and the thing to ingest.

**Raw markdown was leaking into link previews.** An entry's summary is derived
from its first sentence, and that sentence contains markup:

```
og:description: "`.RAW` is 8-bit indexed pixels, top to bottom, ..."
```

Backticks in a link preview look like a bug because they are one. Summaries are
reduced to their text now wherever markup would be shown literally rather than
rendered - the meta tags, the feed summaries, the index blurbs.

## Publishing, and what is honestly not built

The `dir` target works, and the incremental comparison does what it was
specified to do. Editing one entry:

```
$ cairns publish --target site --dry-run
changed: 53
54 unchanged
```

The manifest is the `log.json` already sitting at the destination, so there is
no state file in the working tree to go stale.

The reserved `http` target earns its place already:

```
$ cairns publish --target hosted --dry-run
POST https://example.invalid/ingest
content-type: application/json
authorization: Bearer $(CAIRNS_TOKEN)
content-length: 264869
```

That is a real ingest request, from a real log, with no server anywhere. Run
without `--dry-run` it refuses rather than pretending.

`git-branch` is **not** built, and [publish.md](../docs/spec/publish.md) claimed
it was - written when the spec was describing intentions rather than code. It is
marked reserved now. A CI job that runs `build` and hands the directory to the
host's own deploy action does the same work without this tool force-pushing
anything, which is both safer and what most projects already have; the workflow
is in the spec and in this repo.

## A test that was wrong about the code

One renderer test asserted an entry's prose appears once on its page. It appears
three times, and the code is right: the derived summary lands in `description`
and `og:description` as well as the article. The assertion now checks the
rendered paragraph and the meta tag separately. Worth recording because the
first instinct on a red test here was to go looking for a duplication bug that
was never there.

**Still unknown:** whether client-side search stays sensible as a log grows -
hellbender's `search.json` is 190 KB at 55 entries, fetched on first keystroke,
and nothing has been tried at ten times that.
