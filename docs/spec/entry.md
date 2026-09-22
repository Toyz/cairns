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
- values are plain text, trimmed of surrounding whitespace, never quoted, never
  spanning lines, with no comments, anchors, blocks or nesting
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
| `resolves` | no | list of integers | entries whose open question this one answers |

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
after it on that paragraph is parsed out and collected into the log's open
questions, so a project can ask what it does not yet know across every entry at
once. The literal string `nothing` closes the entry out.

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
