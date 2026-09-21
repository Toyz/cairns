---
title: cairns.toml
status: draft
spec_version: 1
---

# `cairns.toml`

One file at the repo root. It carries the project's identity, the areas an entry
may be filed under, and where things live. It holds **no secrets** - see
[publish.md](publish.md) - so it is committed next to the entries it describes.

```toml
spec_version = 1

[project]
name        = "Hellbender"
slug        = "hellbender"
description = "Reverse engineering Hellbender (Microsoft / Terminal Reality, 1996) and porting it to Rust."

[paths]
entries = "worklog"
index   = "WORKLOG.md"

[site]
base_url = "https://toyz.github.io/hellbender/"
theme    = "default"

[[area]]
name  = "format"
about = "a container or record layout decoded"

[[area]]
name  = "decomp"
about = "facts pulled out of HELLBEND.EXE itself"

[[area]]
name  = "port"
about = "Rust port architecture and progress"

[[publish]]
name = "site"
type = "dir"
path = "site/"
```

## `[project]`

`slug` is the project's permanent identifier. An entry's canonical id is
`{project.slug}/{number}` - not the bare number - because a hosted service, or
merely an aggregator over several of your own repos, has to hold entries from
many projects at once without collision. Deciding this on day one costs a line
of config; retrofitting it invalidates every URL already handed out.

`name` and `description` are display text and may be changed freely.

## `[paths]`

Defaults are `worklog` and `WORKLOG.md`, matching the layout the format grew up
in, so an existing project adopts cairns without moving a single file.

## `[site]`

`base_url` is what every internal link is rendered against. Links are never
written file-relative, so one build serves correctly from a GitHub Pages
sub-path and from a domain root without re-rendering.

`theme` names a built-in theme, or a path to one.

## `[[area]]`

An ordered list, not a map, because the order is the order they are presented
in - in the generated skill, in the index summary, and in the site's filters.

Areas are entirely project-defined. The original tool hard-coded fifteen of
them, several specific to one 1996 game, which is the single thing that most
stopped the format from being usable anywhere else. `cairns init` offers a
starting set; a project is expected to edit it.

An entry filed under an area not declared here is an error, and that strictness
is the point: it is what stops a log accumulating `render`, `rendering` and
`renderer` as three separate areas over six months.

`about` is one phrase, lower case, no trailing period. It is copied verbatim
into the generated skill so the model filing an entry knows what each area is
for.
