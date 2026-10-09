---
number: 31
title: The clock without a model to start it, and the rest of the list
date: 2026-10-08
area: cli, site, spec, skill
files: crates/cairns/src/main.rs, crates/cairns/src/mcp.rs, crates/cairns/src/clock.rs, crates/cairns/src/doc.rs, crates/cairns-core/src/doc.rs, crates/cairns-site/src/html.rs, docs/spec/config.md
started: 2026-10-08T14:32:15-07:00
took: 5m
---

# 31. The clock without a model to start it, and the rest of the list

Five things that followed from [[30]], and one that came in while doing them.

## A hook, so the clock does not depend on a model

The `clock` skill asks a model to start the clock; a hook does not ask.
`cairns init --hooks` adds a Claude Code `UserPromptSubmit` hook to
`.claude/settings.json` running `cairns start --keep --quiet`: on every prompt,
a clock starts if none is running, and prints nothing - a hook's output can
land in the model's context. `cairns new` stops it as before, so `took:`
becomes the time from the first prompt after the last entry. It keeps the rest
of the settings file, and in its order: `serde_json` sorts object keys unless
`preserve_order` is on, and the first run moved the project's `permissions`
below the new `hooks`. Turning it on changes nothing else - `log.json` came out
byte for byte the same with and without it. Run twice, it adds the hook once.

## The clock over MCP, and a field it never wrote

`cairns mcp` gains `worklog_clock`, and with `--write` `worklog_start`;
`worklog_new` records the clock as `cairns new` does, or takes `took`. Every
clock call there is the quiet one, since stdout is the protocol and a stray
line is a broken message. Doing this found that `worklog_new` had always
offered `resolves` in its schema and never written it: a question closed
through MCP stayed open. It is written now.

## The rest

- `cairns doc list --strict` fails when a page needs looking at, for CI, the
  way `check` guards the log.
- The Areas list shows the time recorded under each area beside its count,
  once any entry records time; an entry in two areas counts in both. On a phone
  the time moves to the hover title.
- A reference page's `covers:` - groups split by `;`, items by `,` - is shown
  under its title the way an entry shows its files, folded past five. On
  piney_apples' dungeon page that is 86 items in 9 groups. The first render
  put each comma before its item, so a wrapped line began with one.
  `covers_is_groups_of_items`.

## `cairns update`

Asked for while the above was being built: a way to refresh what cairns
generates without `init`, which also writes config, relaxes `check` for an old
log and freezes slugs. `update` rewrites the skills - keeping each one's hand
written half - brings an existing clock hook up to the current command without
adding one to a project that never asked, and regenerates the index. Each file
is reported as written, updated or unchanged, because after an upgrade the
question is what changed.

This entry is the first clocked from the start: `cairns start` ran before the
first command of the work, and `took:` is what the clock says.

**Still unknown:** whether a prompt hook is the right edge for the clock - it starts on the first prompt after an entry, so a conversation that wanders before the work begins is counted; only real logs will show how far that skews took:
