---
number: 25
title: A log of hundreds read like a log of ten
date: 2026-10-03
area: site, core, spec
files: crates/cairns-site/src/html.rs, crates/cairns-site/src/assets/style.css, crates/cairns-site/src/assets/search.js, crates/cairns-site/src/highlight.rs, crates/cairns-core/src/entry.rs, crates/cairns-core/src/config.rs, docs/spec/config.md, docs/spec/entry.md
---

# 25. A log of hundreds read like a log of ten

The site was built against this log and hellbender's, which is to say against
twenty to fifty entries. piney_apples has 360, eighteen areas and a reference
tree, and on it the site felt wrong in a way no single thing explained. Taken
apart, it was several things, each invisible at twenty entries.

## Two that were bugs

**Quoted titles.** 103 of piney_apples' 360 titles are written `title: "..."`,
because a title with a colon or semicolon looks like it needs protecting. The
subset never quotes, so the quotes were part of the title and printed on every
row of the index. A matching pair of `"` or `'` with no quote of the same kind
inside is now read as delimiters - the reading a YAML parser would give, which
the spec already claimed agreement with. Slugs do not change, since `slugify`
drops quotes anyway; `WORKLOG.md` does, so a log with quoted titles needs one
`cairns index` after upgrading or `check` fails on a stale index.
`a_quoted_value_loses_its_quotes`,
`a_value_that_only_starts_and_ends_with_quotes_keeps_them`.

**References in summaries.** A summary is plain text, and `plain_md` dropped
the link but kept the brackets: `[[358]] and [[359]] left two questions open`.
It now resolves the reference and prints `#358`.

## The index

Eight entries to a screen, each with its full date, in 22px serif. Now:

- a heading per day, sticky while its entries scroll under it, and the date
  gone from every row - piney_apples writes a dozen a day;
- titles a step smaller, summaries clamped to two lines, less padding;
- a Detailed/Compact switch, compact being one line an entry, about 25 to a
  screen, remembered in the browser rather than the URL;
- `/` to search, `j`/`k` to move, Enter to open; `←`/`→` on an entry page;
- beside the list, when there is room for a third column: entries, open
  questions, and entries per day across the log's span, each bar going to its
  day. Below three columns it is dropped rather than pushed above the list.

Sorting reverses both the days and the entries within each, and filtering
hides a day once it is empty, or the page is a column of dates. A `#d2026-10-02`
hash from the strip is a place to scroll to, and `fromHash` now ignores it -
before, clicking a bar would have cleared the filters.

On a phone the eighteen area filters were a wall above the search box. They are
one scrolling row now. The first attempt showed the label and no chips: a
narrow-screen rule gives `.rail-label` 100% width, and in a no-wrap row that
pushed every chip out of view.

## Colours, a stylesheet, highlighting

`[colors]` overrides the palette by token, `[colors.light]` and
`[colors.dark]` per scheme; `site.stylesheet` appends a CSS file after
everything else. Both are appended to the one `style.css`, so an override never
costs a second request. Unknown tokens are errors by name; a value that could
end its declaration is refused. This repo's own config uses it.

Fenced code with a language is highlighted at build time by `syntect` -
classes, not inline colours, mapped onto six `--code-*` tokens so code follows
the scheme and `[colors]` like everything else. Its bundled grammars have no
TOML, the most common tagged fence in either log, so a small grammar ships in
`assets/toml.sublime-syntax`. An untagged or `text` fence, which is most of
them, stays plain.

## The layout bug the screenshots missed

Every screenshot above was taken at 1440px or wider, or at phone width. Between
62rem and 86rem - roughly 1000 to 1376px, which is most laptops - an entry page
with a contents box was broken, and had been since the contents box was moved
above the prose for narrow screens. The shell is a two-column grid there with
three things in it, and auto-placement put the contents box in the second
column of the first row and wrapped the article into the *first* column of the
second: the whole entry in a 14rem strip under the rail. The rail now spans
both rows and the contents box and article are pinned to the second column.

The lesson is the one [[13]] already taught, at a width it did not cover:
screenshot the widths in between, not only the two ends.

The order and density switches started as four words and are now two icon
pairs, each keeping its name as a label and on hover.

Everything above was checked by screenshot against a local build of
piney_apples, and of this log in both schemes, at phone, laptop and desktop
widths.

**Still unknown:** whether the activity strip earns its place on a log that is written in one burst - piney_apples' is eleven days, and a strip that is one tall bar says little
