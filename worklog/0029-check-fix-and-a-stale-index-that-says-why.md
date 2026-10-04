---
number: 29
title: check --fix, and a stale index that says why
date: 2026-10-03
area: cli
files: crates/cairns/src/main.rs, crates/cairns/templates/SKILL.md
---

# 29. check --fix, and a stale index that says why

Issue #1: editing `[index] header` in `cairns.toml` leaves `WORKLOG.md` stale,
and `check` fails until `cairns index` is run by hand, though no entry changed.
Setting up a project's header and areas, that is a separate `cairns index`
after every edit, and in CI a config tweak becomes a failed build. The issue
asked for `check` to stay strict by default, and it does.

`cairns check --fix` regenerates a stale or missing index and says so; every
other problem is still reported and still fails. Regenerating loses nothing:
the index is derived from the entries and the config and holds nothing of its
own.

Without `--fix`, the message now says why the index is stale, so a config edit
reads as expected rather than as a problem with the entries:

```
WORKLOG.md is stale: its header changed in cairns.toml since it was generated - run `cairns index`, or `cairns check --fix`
WORKLOG.md is stale: 1 entry is not listed yet - run `cairns index`, or `cairns check --fix`
```

The table of entries is compared on its own, row by entry number: an
unchanged table means only the part above it changed, which comes from the
config. The first version compared rows as text, so a renamed entry counted
twice - a row gone and a row added. `a_stale_index_says_whether_the_config_or_the_entries_moved`.

Checked by following the issue's own steps in an empty directory: `init`,
`check` ok, edit the header, `check` fails naming the config, `check --fix`
regenerates, `check` ok.

**Still unknown:** nothing
