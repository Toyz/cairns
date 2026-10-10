---
title: The entry
status: solid
worklog: 1, 10
spec_version: 1
---

# The entry

One entry is one file under the entries directory (`worklog/` by default),
named `NNNN-slug.md`.

```
worklog/0050-kreash-mix-is-the-end-of-a-table.md
```

`NNNN` is the entry number, zero-padded to at least four digits and widening
past four when a log gets that far. Numbers start at 1, are unique, and are
assigned once. Gaps are legal - an entry may be abandoned before it is
committed - and a reader must not assume `N-1` exists.

Putting the number in the filename means the next number is a directory
listing, never a read of the entries themselves, so `cairns new` costs the same
on entry 5 and entry 5,000.

## Two entries with one number

Numbers are assigned by counting, so two branches - or two agents in separate
worktrees - that each write the next entry write the same number, and `check`
says so after they merge. `cairns renumber <path>` moves one to the next free
number: its `number:`, its heading, its file name and the folder of its
attachments. Its slug, which is its URL, stays. Other entries that name the old
number are listed rather than changed - after a merge, `[[34]]` elsewhere may
mean either entry, and only someone who knows which can say.

## Attachments

Files kept with an entry - a screenshot, a capture - go in a folder named like
it, beside it:

```
worklog/0050-the-table.md
worklog/0050-the-table/screenshot.png
```

The entry links to them relative to itself, `![the hold](0050-the-table/screenshot.png)`,
which is also what renders on a forge. The site serves them beside the entry's
page. `cairns new --attach screenshot.png` copies a file in and points the
body's `](screenshot.png)` at it, since the number is not known until the
entry is written.

## The slug

Derived from the title: Unicode NFKD, non-alphanumerics collapsed to `-`,
lower-cased, then truncated **at a word boundary** to at most 60 characters.

Truncating mid-word is the one behaviour deliberately changed from the original
Python tool, which cut at exactly 60 characters and left 13 of hellbender's 53
filenames ending mid-word. The filename is tolerable either way; the URL is not.

A `slug:` field in front matter overrides the derived value. Once an entry is
published the slug is frozen - it is the URL - so a title may be tidied and the
slug left alone. The filename is *not* the identity: the slug is, and the number
above it.

## Front matter

Front matter is delimited by `---` on its own line at the very start of the file
and `---` on its own line to close.

**It is not YAML.** It is a strict subset, defined here so that it parses
identically everywhere and can never grow ambiguity:

- one `key: value` per line, key first, first colon splits
- keys are lower-case ASCII with no spaces
- values are plain text, trimmed of surrounding whitespace, never spanning
  lines, with no comments, anchors, blocks or nesting
- values are not quoted, but a value wrapped in one matching pair of `"` or `'`,
  with no quote of the same kind inside, is read without them - writers quote a
  title with a colon in it on reflex, and a YAML reader would strip them too
- list-valued fields are comma-separated on the single line
- a blank line inside front matter is ignored; anything else is an error

A real YAML parser will read this correctly. That is a convenience, not a
promise - the subset is the spec.

### Fields

| field | required | type | meaning |
| --- | --- | --- | --- |
| `number` | yes | integer | the entry's number; must match the filename |
| `title` | yes | text | plain words, sentence case, no trailing period |
| `date` | yes | `YYYY-MM-DD` | the day the work concluded |
| `area` | yes | list | one or more areas declared in `cairns.toml` |
| `files` | no | list | paths the entry is about, repo-relative |
| `summary` | no | text | one sentence; the index blurb and link preview |
| `slug` | no | text | overrides the derived slug; frozen once published |
| `supersedes` | no | list of integers | entries this one corrects or revisits |
| `resolves` | no | list of questions | questions this one answers: `54`, or `54.2` for one question of a list |
| `carries` | no | list of questions | questions this one takes over, unanswered, in the same form |
| `started` | no | timestamp | when the work began, local with its offset |
| `took` | no | duration | how long it took: `1h 23m`, `45m`, `2h` |

Unknown fields are preserved and passed through to `log.json` untouched. A
future field must never be a breaking change.

`area` is written normalised as `", "`-separated on write, because hellbender's
log accumulated both `format, tooling` and `decomp,format` and the index printed
whichever the entry happened to use. Readers accept either.

`summary`, when absent, is derived as the first sentence of the body. Deriving
it is good enough for the index; writing it explicitly is better for a link
someone posts somewhere, because that is the sentence that has to earn the
click.

`resolves` is the other half of the same idea, and is deliberately *not*
`supersedes`. An entry that answers a question another entry left open has not
shown that entry to be wrong - it has closed something it opened. Conflating the
two would lose the distinction that makes either worth recording.

A question with a `resolves` pointing at it leaves the log's open questions. It
stays on the entry that asked it, struck through, naming what closed it.
`check` rejects a `resolves` aimed at an entry that left no question open,
because that is almost always the wrong number and nothing else would catch it.

## How long it took

`started` and `took` are written by the tool, not by hand: `cairns start`
stamps the time, and `cairns new` records it and the minutes since. The writer
is usually a model, and a model asked how long something took gives a
confident number that is wrong; a clock does not.

```
started: 2026-10-08T14:02:11-06:00
took: 1h 23m
```

What `took` measures is **wall-clock**, start to entry - every break included.
It is documented as that and claims nothing more; a session that spanned a
night is corrected by hand, or recorded with `cairns new --took` instead of the
clock. A duration is hours then minutes, each at most once - `90` alone is
refused, since it reads as either.

An entry without `took` did not record it. Totals count only entries that did;
an entry that says nothing is not counted as zero.

## Answering, and carrying, one question

A trailer written as a list is a list of questions, numbered from 1 in the
order written - the entry's page shows the numbers. `resolves: 54.2` answers
the second question of entry 54 and leaves the others open; `resolves: 54`
answers all of it.

`carries` is for an entry that gathers open questions into its own list
without answering them - a triage pass:

```
carries: 2, 4, 15.1, 15.3
```

The questions it carries are no longer open *there*; they are open here, in
the carrying entry's own trailer. The entries that asked them keep them, not
struck through - nothing was answered - pointing at where they went. Using
`resolves` for this, which was the only way to say it before `carries`
existed, tells every reader of the older entries that their questions were
answered.

`check` rejects either aimed at an entry that left nothing open, at a
question past the end of a list, or at a question number on a trailer that is
not a list.

## Referring to code

```markdown
The trailer is split by [[src/entry.rs#question_items]].
Its numbers come from [[src/entry.rs:300-310|these lines]].
As it was then: [[src/entry.rs#question_items@3fbdc65]].

![[src/entry.rs#question_items]]
```

A `[[...]]` whose target has a `/` or a `.` in it is code in the repository,
not an entry:

| form | names |
| --- | --- |
| `path` | the file |
| `path:12` | a line |
| `path:12-40` | a block of lines |
| `path#name` | a definition, found by name |
| `...@rev` | as it was at a commit |
| `...\|words` | shown as the words |

`#name` is the form to prefer: lines move as code changes, and an entry that
pointed at lines 300-310 will point at something else a month later. A name
is found where it is defined - after `fn`, `struct`, `class`, `def` and the
like - and the reference spans its block, by braces or by indentation. `@rev`
pins it to a commit, read with `git show`.

With a `!` in front the code is embedded rather than linked: shown in the
entry, highlighted, under a caption linking to where it lives - at most 200
lines of it. `files:` takes the same forms, and links to the lines.

References are resolved when the site is built, against the checkout, and
carried in `log.json` so a renderer needs no repository. One that no longer
resolves is shown struck through with the reason, not as a broken link; an
entry is not wrong for having pointed at where code used to be, so `check`
does not fail on it. `cairns new` warns while the entry can still be fixed,
and `cairns doc list` flags it on a reference page, which is meant to be
current.

## Code that refers back

Code refers to the log too - "see worklog 50" in a comment, "(worklog 361)" in
a message. The site finds those in the repository's tracked files - `worklog`
followed by a number, or a path into the entries directory - and an entry's
page lists them as "Mentioned in", each linking to its line; `cairns refs 50`
prints them, with every other place that names the entry. An entry's page also
lists the later entries that link to it with `[[N]]`.

## Referring to another entry

```markdown
As [[12]] showed, the anchors kept moving.
The stylesheet loss is written up in [[12|entry twelve]].
```

`[[12]]` becomes a link to entry 12, labelled with the number and carrying the
entry's title, and `check` rejects one that points at an entry which does not
exist. Nothing to look up, nothing to mistype, and no breakage when a slug
changes - which the older form, a markdown link to the entry's filename, could
not promise.

An entry in another worklog the project names under `[workspace]` is
`[[name:12]]` - see [config.md](config.md).

The older form still works and still renders on GitHub, where `[[12]]` is
literal text. That is the trade: a reference reads better and cannot rot; a
file link survives outside this tool.

References are ignored inside code, fenced or inline, so a page can show the
syntax, or a TOML snippet full of `[[area]]`, without either becoming a link.

`supersedes` is what makes the append-only rule pay off. Entry 50 of hellbender
demonstrates the case exactly - it overturns a claim made in entry 6 - and with
the field set, a reader landing on entry 6 is told so, rather than believing a
thing the author already knows is wrong.

## The body

After the closing `---`:

```markdown
# 50. KREASH.MIX is the end of a table

<prose>

**Still unknown:** <what remains open, or "nothing" if closed out.>
```

The first heading is an `h1` repeating the number and title. Sub-headings inside
the entry are `h2` or deeper.

`**Still unknown:**` is a structured trailer wearing prose clothes. Everything
after it to the end of the entry is parsed out, as markdown, and collected into
the log's open questions, so a project can ask what it does not yet know across
every entry at once. It may be a sentence on the same line or a list of
questions beneath it:

```markdown
**Still unknown:**
- whether the second table is ever read
- what [[12]] left about the header's last word
```

A trailer whose first sentence is `nothing` closes the entry out. Anything
after that sentence is a note - kept on the page, not collected as a question:

```markdown
**Still unknown:** nothing. The test runs on Mutation only; the other volumes
share the same path.
```

`nothing about the offset. Whether ...` is not closed: "nothing about" is a
qualifier, and what follows it is still open.

A trailer used to be read only to the end of its paragraph and folded onto one
line, which was right for a sentence and ruined a list - every item run
together, dashes and all. A reader of an older export may still hold trailers
in that shape.

Every entry must carry the line, and `check` says so. An entry that simply
omits it drops out of the log's open questions silently - hellbender lost
twenty-six entries that way without a single complaint, because a missing
convention is not a broken one. Writing `nothing` is a deliberate act; leaving
it out is not, and the two should not look the same. A blank trailer is the
same fault and is reported the same way.

A log adopted from before the convention can relax it - see `[check]` in
[config.md](config.md) - and `cairns init` does that automatically rather than
failing on history.

The marker must **begin a line**, and where several qualify the **last** one is
the trailer. An entry is entitled to mention the marker in its prose - a log
about this format will do it constantly - and an unanchored search reads that
mention instead of the trailer.

## Conventions the tool does not enforce

These are rules for the writer, carried in the skill rather than in `check`,
because a tool that enforced them would mostly be wrong:

- prose, not bullet soup; bullets for genuine lists only
- lead with the finding, not the process
- label a guess a guess, and say what evidence would settle it
- record the negative results - the format that was not what it looked like,
  the encoding that failed, the function that was dead code
- keep every offset, size, address and path exact; a wrong constant in the log
  is worse than no log
- never rewrite an earlier entry to match what you now know
