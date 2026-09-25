---
number: 24
title: An entry written in one command, prose on stdin
date: 2026-09-24
area: cli, skill
files: crates/cairns/src/main.rs, crates/cairns/src/mcp.rs, crates/cairns/templates/SKILL.md
---

# 24. An entry written in one command, prose on stdin

Writing an entry was two steps: `cairns new` made a stub, then the prose went
into it by editing the file. A model doing this got the second step wrong often
enough to matter - looking for the file, rewriting the front matter, or
repeating the heading. And the `**Still unknown:**` line was the part it got
wrong most: left blank, written twice, or put somewhere other than the end.

`cairns new` now takes the prose directly:

```sh
cairns new "Title" --area cli --unknown "what is open" --body - <<'EOF2'
The prose.
EOF2
```

`--body -` reads stdin, and `--body "text"` takes it inline. The command writes
the front matter, the `# N. Title` heading and the trailer, so nothing has to
be opened afterwards. Without `--body` it writes the stub exactly as before -
`without_a_body_the_stub_is_unchanged`.

## One way for the trailer to arrive

The trailer comes from `--unknown`, or from a `**Still unknown:**` line already
at the end of the body. Never both, and never neither:

| body has a trailer | `--unknown` | result |
| --- | --- | --- |
| yes | absent | the body as written |
| yes | given | refused - two trailers |
| no | given | appended |
| no | absent | refused, naming `--unknown nothing` |

Refusing the last case rather than defaulting to `nothing` is deliberate. A
default of `nothing` would close out every entry whose writer forgot, which is
the silent hole [[17]] made `check` close. The refusal happens before a number
is taken, so a refused body leaves no stub behind.

"Has a trailer" means the marker begins a line, as the spec requires. The first
version of the MCP check used `contains`, which counts a sentence that merely
mentions the marker - this entry is one. Both paths now share `has_trailer`.

A body that starts with a `# heading` loses it, because the command writes the
entry's own and a repeated heading is the most common thing handed over with
the prose. Tests: `a_body_takes_its_trailer_from_unknown`,
`a_body_that_ends_with_a_trailer_keeps_it`,
`a_body_with_no_trailer_at_all_is_refused_with_the_fix`,
`a_body_that_repeats_the_heading_loses_it`.

## The MCP tool

`worklog_new` already took a body, but always appended its own trailer, so a
body that ended with one got two. It now goes through the same `compose_body`.
Its default is unchanged - an absent `still_unknown` still means `nothing`,
because its schema has always said so - except that a trailer in the body now
takes precedence over that default.

The skill now leads with the one-command form, and says to quote the heredoc
marker so backticks and `$` in the prose arrive as written.

**Still unknown:** whether a model that is told about --body stops reaching for the stub - the skill says so, but only use will show it
