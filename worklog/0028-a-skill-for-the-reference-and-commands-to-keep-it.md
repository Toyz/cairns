---
number: 28
title: A skill for the reference, and commands to keep it
date: 2026-10-03
area: skill, cli
files: crates/cairns/templates/REFERENCE.md, crates/cairns/templates/SKILL.md, crates/cairns/src/doc.rs, crates/cairns/src/main.rs, crates/cairns-site/src/html.rs
---

# 28. A skill for the reference, and commands to keep it

`cairns init` wrote one skill, for the log, and it had fallen behind the tool:
nothing on trailers written as lists, and nothing at all on the reference,
though [[16]] onward made reference pages half of what a site shows. A model
keeping piney_apples' fifty-two pages had only the worklog skill to go on.

## Two skills

`init` now writes `.claude/skills/reference/SKILL.md` as well, for a project
with `[docs]` and only then - a project without a reference gets no
instructions for one. It says what a page is for, the three statuses and what
each promises, and the one rule that differs from the log: **entries are never
edited, reference pages always are.** A page is the current truth; when a later
entry changes it, the page is corrected and the entry cited, and the history
stays in the log.

The worklog skill gained a short section pointing at the reference, filled in
only when there is one, and says to write an open question as a list when
there is more than one. Both skills keep everything below the project marker,
through one `write_skill` that both go through.

## `cairns doc`

The same reasoning as [[24]]: whoever writes a page is usually a model, and the
step that goes wrong is editing front matter by hand.

```sh
cairns doc new "The archive" --in formats --status partial --from 12,15 --body -
cairns doc cite formats/the-archive 31
cairns doc list
```

`cite` adds to a page's `worklog:` without repeating what is there, and gives a
page with no front matter some. A page is named by slug, by its path under the
docs root, or by its path from the repository.

`list` shows every page with its status and evidence and flags three things: no
evidence, no status, and a page resting on an entry a later one corrected. That
last is the one nothing else catches - both files are valid. A page that also
cites the correcting entry is not flagged, since it was most likely brought up
to date then; the first version flagged those too, and on piney_apples it was
noise. Run against piney_apples it finds four pages citing nothing, and
`formats/data-bin` resting on #3 and #5 without #7 and #14 that corrected them.

Tests in `doc.rs`: `citing_adds_to_the_list_without_repeating`,
`citing_a_page_with_no_list_gives_it_one`,
`citing_a_page_with_no_front_matter_gives_it_some`, `a_cited_page_still_parses`.

## The tree remembers

The reference tree's folders were open only for the page being read, so a
reload - or every live rebuild under `serve` - folded the rest back. The folders
a reader opens are now kept in the browser, per site, and reopened while the
page parses.

**Still unknown:** whether a model given the reference skill keeps pages current as entries land, or only writes new ones; the skill says to cite and correct, and only use will show whether it does
