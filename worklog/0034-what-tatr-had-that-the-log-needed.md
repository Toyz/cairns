---
number: 34
title: What tatr had that the log needed
date: 2026-10-09
area: cli, site, spec, core
files: crates/cairns/src/main.rs#renumber, crates/cairns/src/refs.rs#mentions, crates/cairns-core/src/query.rs, crates/cairns/src/main.rs#located, crates/cairns/src/main.rs#render_site, docs/spec/entry.md
started: 2026-10-09T16:36:03-07:00
took: 9m
---

# 34. What tatr had that the log needed

tsoding's `tatr` keeps tasks in the repository as folders, `tasks/<timestamp>/TASK.md`.
It tracks what is to be done where cairns records what was found out, but five
of its ideas carried over.

## Two branches, one number

tatr names a task by the second it was made, so parallel branches cannot
collide. cairns counts, so two branches - two agents in two worktrees - both
write the next entry, and `check` only says so after the merge. Counting is
worth keeping: it is how every entry is named in prose. `cairns renumber
<path>` moves one: its `number:`, heading, file name and attachments' folder,
slug kept. Every other mention of the old number is listed, not changed -
`[[34]]` in another entry may mean either one, and the text cannot say which.
The duplicate-number problem now names the command.

## The code mentions the log back

`tatr ref` greps a task's id across the repository. cairns links from an entry
to code ([[33]]), and code links back - piney_apples puts "(worklog 361)" in its
messages. The site now scans the tracked files (git's index, so build output
is never read) for `worklog` and a number, or a path into the entries
directory, and an entry's page shows each as "Mentioned in", linking to the
line; `cairns refs N` prints them with every other place that names the entry.
Under half a second on this repository. The first scan counted
`worklog: 12, 15` in the reference skill's template - front matter in an
example - so a line starting `worklog:` is skipped; real pages are read as
pages. Pages also list the later entries that link to them, the way back along
`[[N]]`. `a_line_mentions_an_entry_by_number_or_by_path`.

## A query language

`cairns ls :battle and open and not superseded`, `took gt 2h`, `date ge
2026-10-01`, and the MCP list tool takes the same. tatr's shape: brackets to
group and `lt`/`gt` to compare, so a shell passes it unquoted. Text search was
first `~word`, which the shell expands to a home directory - it is `has word`
now. zsh still globs a bare `[`, so brackets need quoting there.
`a_query_reads_the_way_tatr_does`, `a_bad_query_says_what_was_wrong`.

## Attachments

A tatr task is a folder, so its screenshots sit with it. An entry's
attachments go in a folder named like it, `worklog/0050-x/`, beside the file:
nothing about how entries are found or numbered changes, and the relative link
renders on a forge too. The site copies them beside the entry's page; `cairns
new --attach` copies a file in and points the body's link at it.

## Lines an editor can jump to

`check` prints `path:line: message`, the line the problem is about - the front
matter key it names, the `[[N]]` that is wrong, the trailer.

Piping `cairns ls` into `head` panicked on the closed pipe: Rust ignores
SIGPIPE and turns it into a failed print. It is restored to the default on
Unix, so the command ends quietly, as Unix tools do.

## Git without `git`

The first version ran the `git` binary for both things it reads from git - the
tracked files, and a file at a revision for `@rev` - which needed one on the
path, cost a process each, and meant scraping its output. They are read
in-process through `gix` now, with only its `revision`, `index` and `sha1`
features: a release binary went from 5.0 MB to 6.8 MB on this machine. A
project whose `cairns.toml` sits in a subdirectory of its repository is
handled - paths are translated between the two, checked with a repository whose
log lives under `proj/`. The only process left to spawn is the browser `serve
--open` starts.

Writing this entry, `--unknown -` was passed by habit a second time, an hour
after the refusal for it was added - and was refused, which is the refusal
doing its job.

**Still unknown:** whether renumbering by hand holds up once several agents write in parallel - it fixes one collision after a merge, and a log written by four worktrees at once may want numbers handed out before the merge instead
