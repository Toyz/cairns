---
number: 26
title: Questions that close, reference pages that read, icons from anywhere
date: 2026-10-03
area: site, core, spec
files: crates/cairns-core/src/entry.rs, crates/cairns-site/src/html.rs, crates/cairns-site/src/icon.rs, crates/cairns-site/src/assets/search.js, crates/cairns-site/src/assets/style.css, crates/cairns/src/main.rs, crates/cairns/src/serve.rs, docs/spec/entry.md, docs/spec/config.md
---

# 26. Questions that close, reference pages that read, icons from anywhere

Four things on piney_apples' site, each reported as "broken" or "bad", and
each a different cause.

## Closing a question never showed

`resolves:` worked in the data - a closed question left `cairns open` and the
open page - but the entry that asked it showed the question exactly as if it
were still open. The stylesheet had `.unknown--answered` and `.answered` from
the start, struck through and naming what closed it, and no page ever emitted
either class. The spec described a feature the renderer did not have. It does
now: "Was unknown", struck, "Closed by No. 282 - ...".
`a_closed_question_says_so_on_the_entry_that_asked_it`.

## A trailer that is a list

151 of piney_apples' 360 trailers are lists. The parser read a trailer to the
end of its paragraph and folded it onto one line, which read a sentence right
and turned a list into one run-on paragraph with its dashes left in - on the
open page, thirty questions as a single bullet. And because the page cuts the
prose at the marker, anything after a blank line in the trailer was in neither
place. The trailer is now everything from the marker to the end of the entry,
kept as markdown. Its `[[N]]` references are linked; they were printed raw.
`a_trailer_that_is_a_list_keeps_its_items`.

## The open questions page

Rebuilt: a card per entry, newest first, entry named above its questions
rather than under them; area filters and a search over the questions
themselves; a list past seven items folds after five. Open and closed are one
list with a switch, not a second list folded underneath - opening a few
hundred rows below the page shifted everything around them, which was the
complaint. The long-list fold is a `<details>` in the served HTML, split in
the markdown so an ordered list keeps its numbering; the first version folded
by script after load and the page jumped as it settled.

The same jump was in the reference tree and contents box, which a script at
the end of the page closed on a narrow screen after they had drawn open. They
are now served closed, and a one-line script straight after each opens it on a
wide screen while the page is still parsing.

## Reference pages

A contents list, which entries had and reference pages never did - the
dungeon page has fifteen sections and an empty right column. The evidence is
one line of `#19 #25 #30` rather than eleven pills wrapping under the title.
The tree folds by folder with counts, only the current page's folder open,
folder names capitalised. Previous and next within a section, and the arrow
keys. In a table the first column keeps 9rem, so a long last column no longer
squeezes "The Infection DVD" to a word a line.

## Also

`[[link]]` icons take a URL, or a path to an image in the repository: an SVG is
inlined, so `currentColor` follows the page; anything else is a `data:` URL up
to 64 KB. An SVG with a script, an event handler or a `javascript:` link is
refused, since it lands in every page. The path is resolved before `log.json`
is written, because a renderer elsewhere has no repository to read.

`serve` re-reads `cairns.toml` when it changes. It watched the file and
rebuilt on a change, with the config it read at start - so a new colour
triggered a rebuild in the old colours.

**Still unknown:** whether the fold-before-paint scripts hold on a slow phone, where the parser might yield before the script after a closed details runs; no device was tried, only headless Edge
