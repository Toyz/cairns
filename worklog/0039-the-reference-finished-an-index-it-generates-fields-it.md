---
number: 39
title: The reference, finished: an index it generates, fields it checks, pages that say what doc list knows
date: 2026-10-10
area: spec, cli, site, skill
files: crates/cairns-site/src/reference.rs, crates/cairns/src/doc.rs#problems, crates/cairns/src/git.rs#last_changed, crates/cairns-site/src/html.rs#doc_notice, crates/cairns-site/src/html.rs#backlinks, crates/cairns-site/src/lib.rs#render_index, crates/cairns-core/src/config.rs#DocField, docs/spec/config.md, docs/README.md
started: 2026-10-10T00:04:26-07:00
took: 14m
---

# 39. The reference, finished: an index it generates, fields it checks, pages that say what doc list knows

The reference worked but stopped at rendering pages. The clearest sign was
piney_apples' `tools/docs.py`: 151 lines to generate `docs/README.md` from
front matter, give its four sections titles and an order, check `volumes`
against its five values, fail on a dead link, and fail when the index went
stale. Each of those is something cairns should have been doing.

## An index it generates

[[crates/cairns-site/src/reference.rs]] writes the listing of any
`README.md` under the docs root that carries `<!-- cairns:index -->`. Above
the marker is the project's prose; below it, a table per section - title,
status, the project's own fields, and the evidence linked to the entries. It is
the WORKLOG.md arrangement with one difference: the README has prose worth
keeping, so the marker splits the file rather than the config holding a
header. A README without the marker is the project's, listing and all, and is
never touched.

`cairns doc index` writes it (and a root README if there is none); `cairns
index`, `doc new` and `doc cite` keep it current; `check` fails when it is
stale, with the pages that differ, and `check --fix` rewrites it. On the
site, only the prose above the marker is shown - the site lists the pages
itself, with summaries, which a table cannot.

## Sections and fields it checks

`[[docs.section]]` gives a folder a title, a sentence and a place in the order
- the index, the rail and the generated README follow it. `[[docs.field]]`
declares front matter: `values` it may hold, whether it is `required`, whether
it is a column. `required` applies to `status`, `worklog` and `covers` too.
[[crates/cairns/src/doc.rs#problems]] reports a missing field, a value not
allowed, a `status` that is not one of the three (which used to be dropped
silently - `solidish` read as no status), a page in an undeclared folder, and a
relative link to a file that is not there. Links are read with the markdown
parser, so a link in a code span is not one. Entries are left out of the link
check: an entry is never edited, so a dead link in one cannot be fixed.

`cairns doc new --set volumes=INF` writes a field and refuses a value
`values` does not allow. The reference skill now ends with the project's
sections and fields, generated from `cairns.toml`, so a model writing a page
reads that `volumes` is a list of INF, MUT, OUT, QUA or all.

piney_apples, migrated on a scratch copy: four `[[docs.section]]`, three
`[[docs.field]]`, the marker in place of the old listing. `cairns doc index`
produced its old table plus a Worklog column, in the same order, and `check`
passed; with `volumes: all, PS3`, `status: solidish`, an undeclared folder and
a dead link planted, it reported each. `tools/docs.py` is no longer needed.

## Pages that say what doc list knows

`doc list` knew a page rested on an entry a later one corrected; a reader
never saw it. [[crates/cairns-site/src/html.rs#doc_notice]] puts it at the
top of the page, in the notice an entry has: which entry, corrected by which -
unless the page cites the correction too. It also counts what the page's
`## Unknown` names, linking to it, and says where the page is linked from:
other pages and entries, found by [[crates/cairns-site/src/html.rs#backlinks]]
once for the whole site. An index page links every page under it, so index
pages are left out of that.

Under the page, "How this was found out" lists the evidence with titles and
dates, which sections each backs, and corrections, then the page's source in
the repository.

The dateline says when the page last changed, from the history
([[crates/cairns/src/git.rs#last_changed]]): one walk newest-first for all
pages, skipping commits whose docs tree id did not move, stopping when every
page is placed. A page changed and uncommitted is today. Checked against `git
log -1` for every page here.

## WORKLOG.md

The index marks a corrected entry in its row, says how many entries still
have open questions, and ends with `## Still open`: each, newest first, with
its question's first sentence. It links the site only when `site.base_url` is
written in `cairns.toml` - an address worked out from CI differs between a
fork's CI and a checkout, and the index would be stale in one of them.

**Still unknown:** Whether a project with hundreds of reference pages wants the generated README split - one table per section README rather than every page in the root's. The marker works in a section README already; nothing yet decides for a project that it should use it. And whether `changed` should skip commits that only touched front matter - a `cairns doc cite` changes the date, though the page says nothing new.
