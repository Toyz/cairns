---
number: 32
title: Nothing, with a note after it, is closed
date: 2026-10-08
area: core, spec, skill
files: crates/cairns-core/src/entry.rs, crates/cairns-site/src/html.rs, crates/cairns/src/main.rs, docs/spec/entry.md, crates/cairns/templates/SKILL.md
started: 2026-10-08T20:39:35-07:00
took: 1m
---

# 32. Nothing, with a note after it, is closed

piney_apples writes `**Still unknown:** nothing.` twenty-eight times, and the
log took those as closed. But four more entries said nothing and then
explained - `nothing. The session test runs on Mutation only; ...` - and each
of those was listed as an open question beginning "nothing.". A trailer was
closed only when the whole of it was the one word, so there was no way to say
nothing is open and add a note.

## The first sentence decides

A trailer whose first sentence is `nothing` is closed now, and anything after
that sentence is a note. On piney_apples that closes #380, #381, #403 and #407:
eighty-six open, not ninety. The note was also thrown away - the page lifts the
trailer out of the prose, and a closed one went nowhere - so a closed trailer
now stays in the prose as written, and a reader sees "Still unknown: nothing."
and the note after it. `nothing_closes_an_entry_even_with_a_note_after_it`.

## "Nothing about X" is not decided

Fifteen more piney_apples trailers open with a qualified nothing - `nothing
about the offset`, `nothing new from this entry`, `nothing for this issue`.
Some close the entry and point at another's list (#335: "nothing about +0x08.
The rest of" its entry 95's "list is unchanged"); some wrap a real question (#302:
"nothing about these two paths. Whether any dungeon has a reachable cell whose
room byte is 15 was not surveyed"). The text does not say which, and guessing
would close real questions, so they stay open.

What changes is that a new entry cannot be written that way: `cairns new` and
the MCP tool refuse a trailer that starts with a qualified "nothing", and say
to write `nothing.` or name what is open. History is not held to it - fifteen
entries would fail `check` in piney_apples' CI for a rule that did not exist
when they were written. The skill says the same, and `cairns update` carried
it into this repo's skill - its first use. `nothing_with_a_qualifier_is_hedged`,
`a_hedged_nothing_is_refused_for_a_new_entry`.

**Still unknown:** what the fifteen hedged trailers in piney_apples actually mean - some close their entry and point elsewhere, some carry a real question; they stay listed as open until someone who knows says which
