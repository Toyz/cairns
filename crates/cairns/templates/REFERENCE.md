---
name: reference
description: Write or update a reference page under {{docs}}/ - what is true now about {{project}}, flatly, for someone who wants to use it. Use when an entry establishes something a reader would look up rather than read the story of - a format, a table, an interface, a rule - and when a later entry changes what such a page says. Also use when the user says "document this", "write the reference", "update the docs", "/reference".
---

# Reference

The worklog says how something was found out. A reference page says what is
true, for a reader who only wants to use it: the layout, the table, the rule,
with no story. The two are kept together because each makes the other
trustworthy - a page names the entries that established it, and those entries
say which pages rest on them.

```
{{docs}}/                     the reference, one page per subject
{{docs}}/<section>/README.md  a section's own page, if it has one
```

## The one rule that differs from the log

**Entries are never edited. Reference pages always are.** A page is the current
truth, so when a later entry finds something new, the page is corrected to
match and that entry is added to its evidence. Do not write the history into
the page - "we used to think" belongs in the log, which is where a reader who
wants it will look.

## Writing a page

```sh
cairns doc new "The DATA.BIN archive" --in formats --status partial --from 12,15 --body - <<'EOF'
One sentence saying what this is, first: the summary on the index is taken
from it.

## Layout

| offset | size | field |
| --- | --- | --- |
| 0x00 | 4 | magic |
EOF
```

That writes `{{docs}}/formats/the-data-bin-archive.md` with its front matter and
heading, and puts it in the index. Front matter the project adds goes in with
`--set key=value`, once per key - `--set covers="DATA.BIN, the index table"`. The body is the page only - no `# Title`, which the command writes.
Quote the heredoc marker so backticks and `$` arrive as written.

```markdown
---
title: The DATA.BIN archive
status: partial
worklog: 12, 15
---
```

## Status

Say how far the page can be trusted, honestly:

| status | means |
| --- | --- |
| `solid` | every field accounted for, round-trips or checked across every sample |
| `partial` | works, but named fields remain unknown or unverified - name them |
| `guess` | a hypothesis written down so the next session can attack it |

A `solid` page that turns out wrong costs more than no page. When in doubt,
`partial`, with the unknowns in a section of their own.

## Evidence

`worklog:` lists the entries that established the page. It is what lets a
reader get from a claim to how it was found out, and it is shown on both
sides - the page lists its entries, each entry says which pages rest on it.

When an entry changes what a page says, edit the page and cite the entry:

```sh
cairns doc cite formats/the-data-bin-archive 31
```

Cite it **under the section it changed** when a page has several, so a reader
of that section sees what it rests on, and the entry links to the place:

```sh
cairns doc cite formats/the-data-bin-archive 31 --section "The index table"
```

which writes `<!-- worklog: 31 -->` under that heading. Evidence from another
worklog the project names under `[workspace]` is `name:12` - `piney:361` - in
either place, and `[[piney:361]]` in prose.

## Keeping it honest

```sh
cairns doc list     # every page: status, evidence, and what needs looking at
cairns check        # fails on a page citing an entry that does not exist
```

`doc list` flags a page resting on an entry that a later one corrected - the
page may still say what the old entry claimed - and a page that cites nothing.
Both are worth fixing before they are trusted.

## Rules

- What is true, not how it was found. Lead with the fact.
- Exact numbers, offsets, names and paths. A wrong constant here is worse than
  none, because this is the page people copy from.
- Tables for layouts and field lists; prose for behaviour.
- An unknown is named, in its own section, not left out. A page that omits what
  it does not know reads as more certain than it is.
- Link entries with `[[12]]`, other pages with a relative link to their file.
- Name code by definition, `[[src/thing.rs#parse_header]]`, rather than by
  line; embed it with `![[...]]` where the page is about that code. `doc list`
  flags a reference that no longer resolves - a page is meant to be current.
- Put what the page does not know under `## Unknown`. The open questions page
  collects it beside the log's.

<!-- cairns:project -->

## This project

<Conventions for this repo's reference: what sections exist, what a page in
each must carry. Written by hand - `cairns init` preserves everything below the
marker above.>
