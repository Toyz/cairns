---
number: 27
title: The reference index lists what it holds, and the page is centred
date: 2026-10-03
area: site
files: crates/cairns-site/src/html.rs, crates/cairns-site/src/lib.rs, crates/cairns-site/src/assets/search.js, crates/cairns-site/src/assets/style.css
---

# 27. The reference index lists what it holds, and the page is centred

Follow-up to [[26]], most of which shipped in v0.8.0 before it was meant to:
the release command ran although the call was marked refused.

## Every folder is a section

The reference index listed only folders with a README of their own. piney_apples
has fifty-two pages in four folders and no README in any of them, so its index
listed none, and a search over it would have had nothing to filter. A folder
is now a section either way - named and introduced by its README when it has
one, by its directory when it does not - and holds everything under it.

## A README that is the index stays the index

The first version then appended those fifty-two rows under piney_apples' own
README, which already lists every page in tables of its own: the same pages
twice, the page some 4,000 pixels tall. That was worse than before and was
called worse. When the docs root has a README, it is the index again, as it
was; the rows are there only while a search is running, and the README hides
while it is.

## Search

The reference index has the search the entry list and the open questions have,
over every page's full text, from `docs/search.json`. It reports pages, not
entries.

## One head, one width

A reference page now has an entry's head - the label above the title, the
pills below it - and the rail tree reads like the Areas list. Contents stays on
the right from 62rem up: three columns whose side columns are always equal, so
the reading column is centred on the screen at every desktop width, with or
without contents. Between 62rem and 86rem it had been two columns, rail and
page centred as a pair, the page off to the right and contents above it.
Checked at 1100, 1280, 1440 and 1920 pixels.

## A link to WORKLOG.md went nowhere

piney_apples' reference README says "the [worklog](../WORKLOG.md) is the
other half". On the site there is no `WORKLOG.md` - the index is the entry
list - and with no `repository` configured the link was left as written, a
relative path to a file that is not there. A link to the index, the entries
directory or the README now goes to the page the site has for it: the entry
list, or About. `log.json` carries those three paths as `paths`.
`a_link_to_a_file_the_site_replaces_goes_to_its_page`.

**Still unknown:** whether the reference index without a README of its own reads well past a few dozen pages; cairns has 4, and piney_apples, the large one, has its own README
