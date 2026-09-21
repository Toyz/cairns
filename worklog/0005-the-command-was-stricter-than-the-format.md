---
number: 5
title: The command was stricter than the format
date: 2026-09-20
area: cli, spec
files: crates/cairns/src/main.rs, crates/cairns/templates/SKILL.md
---

# 5. The command was stricter than the format

Asked whether an entry can sit in more than one area, the honest answer was yes
- and then `--area "decomp, engine"` turned out to be rejected while
`--area decomp,engine` worked.

The spec is explicit that both spellings are read: hellbender's log carries 31
entries written `format, tooling` and 19 written `decomp,format`, and
[entry.md](../docs/spec/entry.md) says a reader accepts either. The front matter
parser does. The command that *writes* front matter did not, because clap's
`value_delimiter` splits on the comma without trimming, so the second area
arrived as `" engine"` and failed the area check against a name that has no
space in it.

Both halves were behaving as written. The bug is that they were written to
different rules, and only one of them was the spec. A command that writes a file
has no business being stricter than the one that reads it - the asymmetry is
invisible until someone types the more natural spelling, and then it reads as
the format being fussy rather than the tool being inconsistent.

Values are now split and trimmed on the way in, for `--area` and `--files`
alike, and all three spellings produce the same line:

```
--area decomp,engine        -> area: decomp, engine
--area "decomp, engine"     -> area: decomp, engine
--area " decomp , engine "  -> area: decomp, engine
```

## The question was really about the documentation

The prose in the skill said several areas may be given. Every example showed
one - the `cairns new` line, and the front matter block in the entry format.
Prose under a single-area example does not answer the question the example
raises, which is why the question was asked at all.

Both examples now carry two areas, generated from the project's own first two,
and the sentence saying so sits directly under the command rather than below the
table. The typed form and the written form differ by a space, which the example
now shows rather than explains.

**Still unknown:** nothing.
