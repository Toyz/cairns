---
number: 23
title: Areas in one line each and files that fold
date: 2026-09-24
area: spec, site
files: crates/cairns-core/src/config.rs, crates/cairns-site/src/html.rs, crates/cairns-site/src/assets/style.css, docs/spec/config.md
---

# 23. Areas in one line each and files that fold

Two complaints from use, both about friction rather than correctness: adding an
area was a chore, and an entry with fifteen files put a wall of paths between
its title and its first sentence.

## `[area]` as one table

An area is a name and a phrase, and `[[area]]` spent three lines and two keys
saying so. `cairns.toml` now also takes a single table:

```toml
[area]
spec = "the format itself - entries, config, log.json, publishing"
core = "cairns-core: parsing, validation, the canonical document"
```

The `[[area]]` list is still read and means the same thing. It is deliberately
documented as the *long* form rather than an old one: each area there is its own
table, so a key added to areas later has somewhere to go, and the short form
only has room for `about`. TOML refuses a file holding both, so there is nothing
to reconcile.

Both forms go through one hand-written visitor, `areas` in `config.rs`, rather
than an untagged enum - an untagged enum reports "data did not match any
variant" and throws away TOML's own message about what was actually wrong.

The order matters, since it is the presentation order in the filters, the skill
and the index. I expected to need the `toml` crate's `preserve_order` feature
for that; I did not. `toml` 0.8 hands a visitor the keys in document order
regardless - the feature only changes the order of its own `Table` type.
`an_area_table_reads_in_the_order_written` holds it, with areas declared
non-alphabetically.

A name declared twice is now an error in either form. The table form gets that
from TOML for free; the list form never checked, and now `validate` does.

The cheaper half of "adding an area is annoying" was the error message. `cairns
new --area nope` now ends with the line to paste:

```
to add it, write this under [area] in cairns.toml:
    nope = "what belongs here"
```

It says `[area]` even to a config written in the long form, because the parsed
config does not remember which form it came from. Not worth a flag.

`cairns init` writes the short form, and this repo's own config uses it.

## Files, grouped and folded

The entry page printed `files` as one wrapped row of full paths. Most of the
length was the same `crates/x/src/` repeated. Now each directory is said once,
muted, with the names after it, and the full path is on hover. Past five files
(`FILES_SHOWN` in `html.rs`) the list is a `<details>` that starts closed,
labelled with its count - "15 in 7 directories" - and opens to one directory per
line. No script.

A path with a trailing slash is a directory and is named by its last component
under its parent, so `docs/spec/` shows as `spec/` under `docs/`, not as an
empty group under itself.

Checked with the render loop from [[13]], against a copy of this log with
entry 22 given fifteen files, open and closed. The first screenshot showed the
filename inside a code chip and the directory outside it, which read as two
unrelated things; the chip is gone inside `.files`. Tests:
`an_entrys_files_say_each_directory_once`,
`a_long_list_of_files_folds_behind_its_count`.

**Still unknown:** whether five is the right point to fold - it is a guess,
and a log with many three-to-seven-file entries would say.
