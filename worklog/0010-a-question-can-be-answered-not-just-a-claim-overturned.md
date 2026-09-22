---
number: 10
title: A question can be answered, not just a claim overturned
date: 2026-09-20
area: spec, core, site
files: crates/cairns-core/src/log.rs, crates/cairns-core/src/entry.rs, docs/spec/entry.md
---

# 10. A question can be answered, not just a claim overturned

Asked what happens when a later entry answers an earlier one's open question,
and the answer was: nothing. The question stayed on the open list forever. The
log could record that a claim had been overturned and could not record that a
question had been closed, which are different things and only one of them had a
field.

`resolves` is that field.

```
---
number: 7
resolves: 6
---
```

It is deliberately not `supersedes`. An entry that answers a question another
entry left open has not shown that entry to be wrong - it closed something that
entry opened. Folding the two together would lose the distinction that makes
either of them worth recording at all.

A question with a `resolves` pointing at it leaves `open_questions`, so that
list is what the project does *not yet* know rather than everything it has ever
wondered. It stays on the entry that asked it, struck through, naming what
closed it, because the fact that it was once open is part of the record.

`check` rejects a `resolves` aimed at an entry that left no question open. That
is almost always a wrong number, and nothing else in the tool would notice it.

## The one real case, backfilled

[[6]] ended
asking whether client-side search stays sensible as a log grows.
[[7]] measured it at 192 entries
and answered it. The field did not exist when 7 was written, so `resolves: 6`
was added to it afterwards.

Worth saying out loud, because the log is append-only: that is a metadata
addition to front matter, not an edit to anything 7 claims. The prose is
untouched. Had the correction been to a sentence rather than a field, the rule
would have required a new entry instead.

**Still unknown:** whether a question ever wants partially answering. The format
allows one trailer per entry and one `resolves` closes it entirely, so a partial
answer has to be a new entry with its own open question - which may turn out to
be the right shape or may turn out to be a nuisance.
