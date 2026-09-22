---
title: cairns.toml
status: solid
worklog: 1
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
repository  = "https://github.com/Toyz/hellbender"

[paths]
entries = "worklog"
index   = "WORKLOG.md"

[site]
base_url = "https://toyz.github.io/hellbender/"
theme    = "default"
readme   = "README.md"

[docs]
dir   = "docs"
label = "Reference"

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

`readme` names a markdown file rendered as the site's About page. A worklog
without one tells a visitor what was found out but never what the project *is*.
Its relative links are rewritten to point into `project.repository`, since a
link to `docs/spec/entry.md` means a file in the repo and there is no such page
on the site. Without a `repository` they are left alone, and will not resolve.

## `[index]`

One optional key, `header`, replacing the generated opening prose of
`WORKLOG.md`. Everything else in that file is derived, which is why it is never
hand-edited.

## `[[link]]`

The project's own links, shown in the rail below the generated navigation.
That navigation can only ever know about pages cairns makes, and a project site
usually needs to point somewhere else as well.

```toml
[[link]]
label = "Repository"
url   = "https://github.com/Toyz/cairns"
icon  = "github"

[[link]]
label = "Releases"
url   = "https://github.com/Toyz/cairns/releases"
icon  = "download"
```

`icon` is optional and names one of a small built-in set, drawn inline:

`github`, `globe`, `book`, `code`, `download`, `rss`, `chat`, `mail`, `star`,
`link`

with `site` and `web` for `globe`, `docs` for `book`, `feed` for `rss`, and
`forum` and `discord` for `chat`. They are inline SVG rather than a font or a
sprite for the same reason the page uses no webfonts: an icon that needs a
request is missing for the first second, or forever behind a proxy.

A name that is not built in renders no icon - a link without a glyph still
works, and a typo should not stop a site building - but `check` reports it, so
it does not stay quiet either.

## `[check]`

```toml
[check]
open_questions = "required"   # or "optional"
```

`required` is the default: every entry must end with a `**Still unknown:**`
line, saying what it left open or the word `nothing`. The log's most useful
derived output is the list of what the project does not yet know, and an entry
that omits the line leaves that list silently - which is how hellbender's
`cairns open` came to report the state of the project as of entry 33 while
twenty-six later entries said nothing at all.

`optional` is for a log that predates the convention. `cairns init` writes it,
with a comment, when it finds entries without the line, so adopting cairns
never fails on history.

## `[docs]`

Reference pages beside the log, and entirely optional - a worklog is useful on
its own, and most projects have no docs tree to pair with one.

```toml
[docs]
dir   = "docs"
label = "Reference"
```

The tree is walked recursively. A page's front matter is the same strict subset
an entry uses, with different keys: `title`, an optional
`status` of `solid`, `partial` or `guess`, and `worklog`, a comma-separated list
of the entry numbers that established it.

`worklog` is the edge that makes keeping both worthwhile. The page states what
is true; the entries say how that was found out. It is rendered in both
directions - a page links to its evidence, and those entries say which pages
rest on them - and `check` rejects a page citing an entry that does not exist.

A `README.md` or `index.md` **is its directory**, not a page inside it.
`docs/spec/README.md` is served at `/docs/spec/` and heads the section, with the
rest of that directory listed beneath it. Listing it as a peer of the pages it
introduces would be backwards.

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
