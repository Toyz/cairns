---
number: 2
title: What hellbender's own log said about the spec
date: 2026-09-20
area: spec, cli, adoption
files: crates/cairns-core/src/entry.rs, crates/cairns-core/src/log.rs
summary: All 55 of hellbender's entries parse against the new strict front matter with no errors, and check found two real title-filename drifts the old tool could not see.
---

# 2. What hellbender's own log said about the spec

The spec was written against hellbender's log, so the first thing worth knowing
is whether it actually reads it. Pointed at a fresh clone with a `cairns.toml`
declaring the fifteen areas the Python tool hard-coded, all 55 entries parsed:
no front matter errors, no unknown areas, no repeated numbers, and every
`area` field read despite the log using both `format, tooling` and
`decomp,format` spellings across it.

`content_hash` verified from outside the tool, which is the property it was
specified for:

```
$ cairns export --reproducible -o log.json          # entry 50
  content_hash: sha256:40db750a77e9ed60d1b0e242a409751a63f93d0e91e8b9570d0c2e8a72ff6f20
$ shasum -a 256 worklog/0050-kreash-mix-is-the-end-of-a-table.md
  40db750a77e9ed60d1b0e242a409751a63f93d0e91e8b9570d0c2e8a72ff6f20
```

33 of the 55 entries carry an unresolved `**Still unknown:**` trailer, so the
open-questions page has something real to show on day one rather than being a
feature waiting for a habit to form.

## Two entries had drifted, and nothing had noticed

`check` compares an entry's filename against the slug its title derives, which
the Python tool never did - it only checked the four-digit prefix. Two entries
fail it for reasons that are not about slug rules at all:

```
0035-powerups-and-the-sprite-models.md
  title: Powerups, and the sprite models they are drawn with

0037-the-players-guns-and-the-energy-that-feeds-them.md
  title: The player's guns, and the energy that feeds them
```

Both had their titles edited after the file was created. Harmless while the
filename is only a filename. Not harmless once it is a URL, which is the
argument for the `slug:` field: identity is the slug, the filename follows it,
and a title can be tidied afterwards without breaking a link.

## Adoption is not free, and that is the useful finding

Eleven more entries fail the same check for a different reason: the slug is now
cut back to a word boundary rather than at exactly 60 characters, so
`...-the-picture-foun` becomes `...-the-picture`. Better names, but every
existing project adopting cairns would see its files want renaming, and any
already-published link would break.

So adoption has to freeze what exists rather than improve it. `init` against a
log that already has entries should write an explicit `slug:` into every entry
whose filename disagrees with its title, pinning the URLs that were already
handed out, and leave the better derivation for entries written from then on.
That is a real change to what `init` does, and it was not in the plan before
running this.

One more gap: no entry uses `supersedes`, because the field is new - yet entry
50 exists specifically to overturn a claim entry 6 made about `KREASH.MIX`, and
says so in prose. The link is in the log already, just not in a form anything
can read. Backfilling those during migration is worth doing by hand; there are
not many, and they are the most valuable edges in the graph.

## The log found a bug in the log

Writing this entry broke the parser that reads it. The paragraph above
mentions the open-question marker in prose, and the trailer was found with an
unanchored search for the literal, so entry 2's open question came out as the
tail of a sentence about entry counts. The marker now has to begin a line, and
where several qualify the last one is the trailer - which is what the spec
should have said in the first place, because any log written *about* this
format will mention the marker constantly.

Two entries in, dogfooding has paid for itself.

**Still unknown:** whether `migrate` can find prose corrections like entry 50's
reliably enough to suggest `supersedes` edges, or whether that stays a manual
pass.
