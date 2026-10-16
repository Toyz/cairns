---
number: 40
title: One entry writer, a main that only dispatches, pages a file each, and tests that run the binary
date: 2026-10-10
area: cli, site, core, build
files: crates/cairns/src/write.rs#write, crates/cairns/src/mcp.rs#new_entry, crates/cairns/tests/cli.rs, crates/cairns/src/main.rs, crates/cairns-site/src/html/mod.rs, crates/cairns-core/src/log.rs#Edges, Cargo.toml
started: 2026-10-10T14:23:57-07:00
took: 56m
---

# 40. One entry writer, a main that only dispatches, pages a file each, and tests that run the binary

cairns had grown a feature at a time and was never put back together. This
was the pass that did, with one rule: nothing a user sees changes, and that
is checked, not assumed.

## The check

[[crates/cairns/tests/cli.rs]] came first: eight tests that run the built
binary in a directory of its own - `new` with stdin and a refusal that leaves
nothing behind, `check` and `check --fix`, `renumber`, the reference's `doc
new`/`doc index`/`doc cite`/`check`, `build`, `init` and `update`, and
`worklog_new` over MCP beside `cairns new`. Then, for every refactor of the
renderer, the site built by the last commit's binary and by this tree's, for
this log and piney_apples', diffed: 95 and 879 files, byte-identical
throughout, `log.json` the same but for its timestamp. A release build of
piney_apples' site takes the same 0.38s before and after.

The first test found a bug before anything moved: `cairns new` in a project
with no entries directory said "No such file or directory". `next`, `check`
and `index` did too. A missing directory is an empty log now.

## One entry writer

`cairns new` and MCP's `worklog_new` each checked areas, took a number,
assembled front matter, read the clock and composed the trailer - and had
drifted: the server once never wrote `resolves`, could not attach a file, and
refused things in other words. [[crates/cairns/src/write.rs#write]] is the one
path; each surface turns its input into a `NewEntry` and reports a `Written`.
A `Problem` says whether the entry was refused or the write failed, which is
the line MCP draws between an invalid call and an internal error. The test
asserts the two surfaces write the same bytes.

## main.rs

`run()` was 747 lines: every command's work inline in one match, with forty
helpers after it. It is a dispatch now, loading the project once, and the work
lives where it belongs: `project` (finding and building the log), `write`,
`check`, `skills` (`init`, `update`), `publish` (`build`, `export`,
`publish`), `inspect` (`ls`, `refs`, `open`), `renumber`, `migrate`; `doc`
owns its own subcommands. 2,447 lines became 305. On the way: `init` wrote a
`[check]` comment into `cairns.toml` indented by twenty-five spaces (a string
continuation without its backslash), and a doc comment for `build_log` had
drifted onto `text_dirs`.

## html.rs

3,095 lines became [[crates/cairns-site/src/html/mod.rs]] and a file per page,
with `markdown` (text and where links go), `frame` (head, rail, footer) and
`parts` (what pages share). Gone: `unused_markdown_linked`, dead code kept
alive with `#[allow(dead_code)]`, and the `markdown` helper only it used.
`shell()` took nine positional arguments, three of them adjacent `&str`s that
could be swapped silently, behind `#[allow(clippy::too_many_arguments)]`; it
takes a `Page` now, with the current page an enum rather than a string. No
`#[allow]` is left anywhere.

The entry page (266 lines) and the open questions page (280) are composed of
named parts - dateline, notice, trailer, pager; the rows, the chips, a card.
Pages built their own `by_number` map of entries though `Log` keeps an index
for exactly that; they use `log.entry(n)`, as do `refs`, `doc list` and MCP,
which were scanning.

## Said once

- "Is this link a file in the repository?" was written four times; it is
  `repo_path`, which `check`'s dead-link test and the backlinks both use.
- Dropping a body's `# Title` was written five times; it is
  `entry::without_title`.
- The headings that mean "unknown" were listed twice; core's
  `unknown_heading` is the one list.
- The search field's markup was in three pages; it is `search_field`.
- `Log::build_with` (254 lines) inverts its edges in
  [[crates/cairns-core/src/log.rs#Edges]], works out what is still open in
  `still_open`, and moves each entry's fields into the log instead of cloning
  every body, title and hash out of entries it owned and then dropped.

## Lints that stay on

`[workspace.lints.clippy]` turns on what caught work done for nothing:
`format!` appended to a string (32 places, now `write!`), strings collected
from `format!`, arguments taken by value and only read (`build_with` took the
pages by value and `doc list` cloned them to call it), redundant clones and
closures, `map().unwrap_or()`, and numeric casts that can truncate - fixed
with `try_from`, a counter zipped in (`1u32..`) where an index was cast, or an
`#[expect]` with its reason where the range is fixed by construction. `clippy
-D warnings` holds the code to them from here.

**Still unknown:** Whether the pedantic lints left off - Self over the type's name, must_use, map_or_else - are worth a pass of their own; they are style, and turning them on would be a large diff for no reader. And whether cli.rs should also drive serve and publish, which it does not touch.
