---
number: 33
title: Questions carried, questions answered one at a time, and code named by what it defines
date: 2026-10-09
area: spec, core, site, cli, skill
files: crates/cairns-core/src/entry.rs#QuestionRef, crates/cairns-core/src/entry.rs#CodeRef, crates/cairns-core/src/log.rs, crates/cairns-core/src/doc.rs#unknown_section, crates/cairns/src/code.rs#find_definition, crates/cairns-site/src/html.rs#code_reference_html, docs/spec/entry.md
started: 2026-10-09T15:32:04-07:00
took: 11m
---

# 33. Questions carried, questions answered one at a time, and code named by what it defines

Three gaps found by asking piney_apples' 407 entries and 54 pages what the
open-questions model got wrong, and one asked for.

## Moved questions read as answered

piney_apples' entries 282 to 291 are triage passes. Each `resolves`
around thirty earlier entries - about 280 in all - then lists most of those
same questions again in its own trailer: #282 resolves 29 and re-lists
questions from 22 of them. Nothing was answered; they were gathered. But
`resolves` was the only way to say so, so every one of those 280 entries
showed its question struck through as answered. The log was saying something
false.

`carries:` says it truthfully. A carried question leaves the open list where it
was asked and is open where it went; the asking entry keeps it, not struck,
"Carried to No. 282, and open there". `resolves` now means answered and only
that. piney_apples' triage entries still say `resolves` - correcting them is an
edit to the past, which is theirs to decide; a later entry can say it instead.
`a_carried_question_leaves_the_open_list_without_being_answered`.

## One answer closed a whole list

137 of piney_apples' `resolves` point at trailers that list several questions,
and each closed all of them. A question is now named as `54.2` - the second of
entry 54's list, numbered on its page so a writer can see the number - and
`resolves: 54.2` closes that one; the rest stay open, keeping their numbers,
with gaps where others closed. `log.json` keeps `resolves` as numbers for
whole entries, so nothing reading it breaks; single questions are
`resolves_questions`. `check` rejects a number past the end of a list, and one
on a trailer that is not a list. `one_question_answered_leaves_the_rest_open`,
`a_question_number_past_the_list_is_rejected`.

## The reference's unknowns were nowhere

49 of piney_apples' 54 pages keep a `## Unknown` section, and the open questions
page knew only entries. It now collects them, under each page's name: 37 pages
on piney_apples. The first pass listed 42 - five say "None: every file is
accounted for above.", which is the trailer's "nothing" rule again, and the same
first-sentence test now closes them. `a_page_says_what_it_does_not_know_under_its_own_heading`.

## Code, by what it defines

`[[src/a.rs#name]]` links to a definition, found by name; `:12-40` to lines;
`@rev` pins it to a commit through `git show`; `![[...]]` embeds the code,
highlighted, under a caption linking to it. Names are the form to prefer, since
lines move. A name is found where it is defined - after `fn`, `struct`, `class`,
`def` and the like, a definition preferred over an earlier mention - and its
block taken by braces or by indentation.
`a_rust_function_is_found_by_name_and_its_block_by_braces`,
`a_python_function_ends_where_its_indentation_does`,
`a_definition_is_preferred_over_a_mention`.

The binary resolves every reference once and carries the result in `log.json`,
like the README: the renderer reads no files, and a renderer elsewhere has no
checkout. The first version resolved every plain path in every `files:` list
too, and called directories missing; now only references naming lines or a
definition are resolved. One that no longer resolves renders struck through
with the reason, not as a broken link. An entry is not wrong for having pointed
where code used to be, so `check` does not fail on it - `cairns new` warns while
the entry can be fixed, and `doc list` flags it on a page, which is meant to be
current. This entry's own `files:` name definitions.

**Still unknown:**
- whether piney_apples' triage entries 282 to 291 will be restated with `carries` - until they are, its site still shows about 280 questions as answered that were only moved
- how well finding a definition by name holds up outside Rust and Python; it was tested on those two and on nothing else
