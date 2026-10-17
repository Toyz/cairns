---
number: 41
title: check holds an entry's images to their files; the copies and duplicates that came out
date: 2026-10-16
area: cli, site, core, spec
files: crates/cairns/src/doc.rs#missing_images, crates/cairns-site/src/html/markdown.rs#image_targets, crates/cairns-site/src/html/markdown.rs#from_repo_root, crates/cairns-core/src/lib.rs#dir_of, crates/cairns/src/serve.rs#image_type, crates/cairns/src/doc.rs#list, crates/cairns/src/git.rs#last_changed, docs/spec/entry.md
started: 2026-10-16T11:11:27-06:00
took: 8m
---

# 41. check holds an entry's images to their files; the copies and duplicates that came out

An image in an entry that names no file rendered as a broken `<img>` and
passed `check`. The question that found it was whether a51's screenshots
survive the build. They do: all 21 are copied beside their pages, and their
links are rewritten to `../49-slug/scan.png` and so on. But
`![c](missing.png)` in a fresh log built with `check` saying `ok`.

## Images are checked; links still are not

`check` already left an entry's links alone on purpose: an entry is never
edited, so a link that went stale later stays as written. An image is not a
pointer to something that moves. It sits in the entry's own folder, so one
that shows nothing was mistyped or deleted, and the fix is the file. The
entry stays as it is.

[[crates/cairns/src/doc.rs#missing_images]] takes the relative image targets
of a body ([[crates/cairns-site/src/html/markdown.rs#image_targets]], which
is `link_targets` without the plain links) and reports any that name no file:
`worklog/0001-one.md: shows 0001-one/shto.png, which does not exist`. `check`
runs it over every entry. `cairns new` runs it over the new entry and warns,
next to its warning for a code reference that does not resolve, while the
entry can still be fixed. A URL, an anchor, a site-root path and code are
skipped. Two tests hold it: the unit test
`an_image_that_shows_nothing_is_a_problem_and_a_link_is_not`, and the
binary-level
`an_attached_image_is_built_beside_its_page_and_a_missing_one_is_reported`.
The second attaches a file, checks it is copied beside the page and linked
there, checks that `new` warns about the typo and `check` fails on it, and
checks that `check` passes once the file is there.

Run against the twelve cairns logs on this machine (a51, bedlam,
cipher-manu, forbidden, hellbender, hwtr, piney_apples, rugrats, 3lizards,
cairns, gpug, retro_rt), it reports nothing. None of them has a broken image
today.

## What came out

This work was rebased onto [[40]], which had split `main.rs` and `html.rs`
and made some of the same cleanups independently. [[40]] had already dropped
the per-page `BTreeMap<u32, &LogEntry>` in favour of `Log::entry`, the dead
`unused_markdown_linked`, and the `docs.clone()` in `doc list`. It had also
turned `local_target` into `repo_path`. This entry adds what was still
there:

- Two path folds. `repo_path` and `from_repo_root` each walked `..` and `.`
  on their own, and differed at the root: `from_repo_root` dropped an extra
  `..` and kept going, so `Links::resolve` linked a different file on the
  forge. There is now one fold,
  [[crates/cairns-site/src/html/markdown.rs#from_repo_root]], which returns
  nothing for a path above the repository. `Links::resolve` leaves such a
  link as written.
- Eight copies of "the directory a path is in" became
  [[crates/cairns-core/src/lib.rs#dir_of]].
- `cairns serve` had its own image MIME table. Its match was case-sensitive
  and had no `.ico`, so an attachment named `Shot.PNG` was served as
  `application/octet-stream`. [[crates/cairns/src/serve.rs#image_type]] is
  now the only table, shared with `resolve_icon`.
- `doc list` still built a `Log` with every page, and cloned
  `superseded_by` on every lookup, for an answer that depends on entries
  alone. It now builds with no pages, borrows the slice, and computes each
  slug once instead of three times.
- Smaller ones: `git::show` and `last_changed` cloned blob data only to
  compare it, `init` read cairns.toml twice, `refs::scan` and `pages_under`
  ran a `format!` for every file, and four summary clones existed only to be
  formatted.

## How it was verified

Two release binaries were run on each of the twelve logs, one built from
origin/main and one from this work. For every log the built sites are
byte-identical except for the `generated` timestamp in log.json, and the
`doc list` and `check` output is identical. All tests pass, and clippy is
clean.

**Still unknown:** whether a build should parse each markdown body only
once. Backlinks, rendering, `check` and the search index each parse every
body again. Each pass through the two reference rewrites also copies the
whole body, even when it has no `[[`.
