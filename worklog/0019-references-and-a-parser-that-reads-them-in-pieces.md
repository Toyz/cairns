---
number: 19
title: References, and a parser that reads them in pieces
date: 2026-09-21
area: spec, core, site
files: crates/cairns-core/src/entry.rs, crates/cairns-site/src/html.rs
---

# 19. References, and a parser that reads them in pieces

An entry could point at another only by writing out its filename:

```markdown
[12](0012-three-css-edits-three-wrong-anchors-and-half-a-stylesheet-gone.md)
```

which you have to look up, can mistype, and which breaks if the slug ever
changes. `[[12]]` now does it: a link to entry 12 labelled with the number and
carrying its title, with `[[12|in other words]]` for your own wording. `check`
rejects a reference to an entry that does not exist, which is the first
link-checking this tool has done and answers part of what [[11]] left open.

The trade is worth stating plainly: `[[12]]` is literal text on GitHub, where
the filename form renders. Both work, so the choice is the author's - a
reference reads better and cannot rot, a file link survives outside this tool.
This log's six existing cross-links were converted.

## The parser hands it over in pieces

The first attempt did the expansion on parsed events, which is where the
existing link rewriting happens, and produced nothing at all. A markdown parser
reads `[[12]]` as nested bracket tokens - an unresolved shortcut reference
inside another pair of brackets - so the text arrives as several runs and `[[`
is never present in one of them to match on. The compiler said only that the
function was never called.

So it happens on the source instead, before parsing, and emits
`[12](cairns:12 "title")`. The scheme is resolved by the same function that
resolves every other link, so a reference and a written-out link reach the same
place by the same code rather than by two implementations that agree today.

## Code has to be left alone

The spec pages show `[[area]]` and `[[publish]]` in fenced TOML, and the page
documenting this syntax has to be able to print `[[12]]` without linking it.
The scanner tracks fences and inline backticks, and the same scanner serves
both the rewriter and `check`, so what gets linked and what gets validated
cannot disagree.

Requiring digits would have protected `[[area]]` by accident. The code rules
make it a guarantee.

## Two things that were wrong about themselves

A test asserting that inline `` `[[1]]` `` produces no link counted two and
failed. The second was the pager's "Previous" link, which points at entry 1
because entry 1 *is* the previous entry. The code was right and the assertion
was too broad; it counts the reference form specifically now.

And the skill did not pick up the new section on the first `init`, because the
template is embedded with `include_str!` and I ran the old binary. The template
had changed, the file on disk was right, and the generated output was stale -
which looks exactly like an edit that failed, and was not.

**Still unknown:** whether `check` should also follow the filename form of a
cross-entry link, which remains unvalidated. [[11]] raised that and only half
of it is answered.
