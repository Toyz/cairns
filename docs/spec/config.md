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

[area]
format = "a container or record layout decoded"
decomp = "facts pulled out of HELLBEND.EXE itself"
port   = "Rust port architecture and progress"

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

When `base_url` is not set, it is worked out where the address follows from
the repository - GitHub Pages or GitLab Pages - from, in order: GitLab CI's
`CI_PAGES_URL`, which is the address itself; GitHub Actions'
`GITHUB_REPOSITORY`; GitLab CI's `CI_PROJECT_PATH`; `project.repository`; and
the checkout's `origin` remote. `https://github.com/owner/repo` is published at
`https://owner.github.io/repo/`, a repository named `owner.github.io` at the
root, and GitLab the same with `gitlab.io` and subgroups in the path.
`cairns build` says what it used and where from. A custom domain cannot be
worked out; a site on one sets `base_url`. Without either, a build says so:
the feed, `ids.json` and every page's canonical link have no address.

`theme` names a built-in theme, or a path to one.

`stylesheet` names a CSS file of the project's own. It is appended to the
site's stylesheet, after everything built in and after `[colors]`, so any rule
in it wins. A stylesheet named and not found is an error rather than a warning:
the site would build, look wrong, and give no reason.

`readme` names a markdown file rendered as the site's About page. A worklog
without one tells a visitor what was found out but never what the project *is*.
Its relative links are rewritten to point into `project.repository`, since a
link to `docs/spec/entry.md` means a file in the repo and there is no such page
on the site. Without a `repository` they are left alone, and will not resolve.

## `[colors]`

The site's palette, overridden a token at a time.

```toml
[colors]
accent = "#c2410c"

[colors.dark]
accent = "#f08c5a"
paper  = "#101014"
```

A token in `[colors]` applies in both light and dark. `[colors.light]` and
`[colors.dark]` apply in one scheme only, and win over `[colors]` there. The
common wish is one accent colour, set once; a colour that only reads well on a
dark page goes under `dark`.

The tokens are the ones the stylesheet is written in:

| token | what it colours |
| --- | --- |
| `paper`, `paper-sunk` | the page, and the panels and hover fills set into it |
| `ink`, `ink-soft`, `ink-faint` | text, from body to the quietest labels |
| `rule`, `rule-faint` | dividers and borders |
| `link`, `accent` | links, and the marks of the current place |
| `code-bg`, `mark` | code blocks, and highlighted or selected text |
| `code-keyword`, `code-string`, `code-comment`, `code-number`, `code-function`, `code-type` | highlighted code |

A token not in this list is an error, so a misspelt one is reported rather
than silently never appearing. A value is any CSS colour, and one containing
`;`, `{`, `}`, `<` or `>` is refused, because it lands inside a stylesheet.

For anything a token cannot reach, use `site.stylesheet`.

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

`icon` is optional, and is one of three things:

- a **name** from the small built-in set below, drawn inline;
- a **URL** - `https://...` - shown as an image;
- a **path** to an image in the repository - `assets/logo.svg` - read at build
  time. An SVG is inlined, so one drawn in `currentColor` follows the page's
  colours like the built-in icons do. A `.png`, `.jpg`, `.gif`, `.webp` or
  `.ico` is embedded as a `data:` URL, up to 64 KB, so the page still makes no
  request for it. An SVG carrying a script, an event handler or a
  `javascript:` link is refused: it is copied into every page.

A value with a `/` or a `.` in it is a path; one without is a name.

The built-in names:

`github`, `globe`, `book`, `code`, `download`, `rss`, `chat`, `mail`, `star`,
`link`

with `site` and `web` for `globe`, `docs` for `book`, `feed` for `rss`, and
`forum` and `discord` for `chat`. They are inline SVG rather than a font or a
sprite for the same reason the page uses no webfonts: an icon that needs a
request is missing for the first second, or forever behind a proxy.

A name that is not built in, or a path that cannot be read or is refused,
renders no icon - a link without a glyph still works, and a typo should not
stop a site building - but `check` reports it, so it does not stay quiet
either.

In `log.json` a path is already resolved: `icon` holds the SVG's markup or the
`data:` URL, because whatever renders the document may have no repository to
read it from.

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
`status` of `solid`, `partial` or `guess`, `worklog`, a comma-separated list
of the entry numbers that established it, and `covers`, what the page accounts
for in the thing it documents - files, addresses, functions - as groups
separated by `;` of items separated by `,`:

```
covers: INF gcmn.prg:0x005c1440 DUNGEON::SetClutList, 0x005c17b0 DUNGEON::ChangeClut; INF SLUS_202.67:0x0013aad0 ccModel::ChangeClut
```

A page shows what it covers under its title, the way an entry shows its files.

A page's own open questions go in a section headed `## Unknown` (or
`Unknowns`, `Still unknown`, `Open questions`). The open questions page
collects them beside the log's, under each page's name.

A section can give its own evidence, in a comment under its heading - a forge
shows nothing, the site shows "from" and the entries under the heading, and an
entry it names links to that section rather than the page:

```markdown
## The index table
<!-- worklog: 40, 52, piney:361 -->
```

`cairns doc cite <page> 40 --section "The index table"` writes it. Entries in
other worklogs - `piney:361` - may be cited here and in `worklog:` alike.

`worklog` is the edge that makes keeping both worthwhile. The page states what
is true; the entries say how that was found out. It is rendered in both
directions - a page links to its evidence, and those entries say which pages
rest on them - and `check` rejects a page citing an entry that does not exist.

A `README.md` or `index.md` **is its directory**, not a page inside it.
`docs/spec/README.md` is served at `/docs/spec/` and heads the section, with the
rest of that directory listed beneath it. Listing it as a peer of the pages it
introduces would be backwards.

## `[workspace]`

Other worklogs this one refers to, by a short name - a monorepo with a log per
crate, or sibling repositories.

```toml
[workspace]
piney      = { path = "../piney_apples", url = "https://toyz.github.io/piney_apples/" }
hellbender = "https://toyz.github.io/hellbender/"
```

`[[piney:361]]` then names entry 361 there, in prose or as evidence. A value is
a path to another cairns project, the URL of a published one, or both. The path
is read when it is there - titles, slugs, and `check` refusing an entry the
project does not have. When it is not - in CI, with only this repository
checked out - the URL is used: the published site's `ids.json` is fetched,
kept for an hour in `.cairns/`, and gives the same titles and the same check.
With no network, or a site that publishes no `ids.json`, an entry is linked by
its number - `<url>/361/`, which every cairns site redirects - and not checked.
A path with no URL, not there, is an error.

A name is lower case letters, digits, `-` and `_`.

## `[area]`

The short form is one line per area: its name, and one phrase saying what
belongs under it.

```toml
[area]
format = "a container or record layout decoded"
decomp = "facts pulled out of HELLBEND.EXE itself"
```

The order they are written in is the order they are presented in - in the
generated skill, in the index summary, and in the site's filters. A reader of
this file must keep it, which a TOML library that hands back a sorted map does
not do by default.

Areas are entirely project-defined. The original tool hard-coded fifteen of
them, several specific to one 1996 game, which is the single thing that most
stopped the format from being usable anywhere else. `cairns init` offers a
starting set; a project is expected to edit it.

### The long form

The same areas can be written as an `[[area]]` list, one table per area:

```toml
[[area]]
name  = "format"
about = "a container or record layout decoded"
```

It is the form to reach for when an area needs to say more than one phrase:
each area is a table, so a key added to areas later has somewhere to go, where
the short form only has room for `about`. Until then the two mean exactly the
same thing, and the short form is what `cairns init` writes, because an area
that is a chore to add is one that does not get added - entries get filed under
the nearest wrong one instead.

TOML does not allow both forms in one file. In the long form `about` may be
omitted, and a name declared twice is an error.

An entry filed under an area not declared here is an error, and that strictness
is the point: it is what stops a log accumulating `render`, `rendering` and
`renderer` as three separate areas over six months.

`about` is one phrase, lower case, no trailing period. It is copied verbatim
into the generated skill so the model filing an entry knows what each area is
for.
