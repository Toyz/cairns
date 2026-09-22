---
number: 21
title: Links the generated nav could never know about
date: 2026-09-21
area: site, spec
files: crates/cairns-site/src/icon.rs, crates/cairns-core/src/config.rs
---

# 21. Links the generated nav could never know about

The rail's navigation was whatever cairns had made: entries, the reference, the
open questions, the About page, the feed. A project site needs to point
somewhere else too - the repository, the releases, a chat - and there was no way
to say so.

```toml
[[link]]
label = "Repository"
url   = "https://github.com/Toyz/cairns"
icon  = "github"
```

They sit in their own group below the generated nav rather than mixed into it,
because one set is pages this tool made and the other is everywhere else, and a
reader can tell at a glance which is which.

## Ten icons, inline

`github`, `globe`, `book`, `code`, `download`, `rss`, `chat`, `mail`, `star`,
`link`, with the obvious aliases - `docs` for `book`, `feed` for `rss`,
`discord` for `chat`.

They are inline SVG, for the same reason the page has no webfonts: an icon that
needs a request is missing for the first second, and missing forever behind a
proxy. Ten of them cost less than one request. The GitHub mark is the Octicons
path, which is MIT licensed; the rest are drawn as plain geometry.

An unknown name renders no icon at all. That is right at build time - a link
without a glyph still works, and a typo should not stop a site from building -
but it is wrong to stay quiet about, so `check` reports it with the list of
what is available:

```
cairns.toml: link "Releases" wants icon "downlod", which is not one of:
github, globe, book, code, download, rss, chat, mail, star, link
```

That pairing is the shape worth repeating: be forgiving in the renderer, strict
in `check`. The site builds either way, and the person who made the typo finds
out.

**Still unknown:** whether a project will want an icon that is not in the ten -
a Mastodon mark, its own logo. A path to an SVG file in the repository would
cover it, but it would also mean the renderer reading a file, which it has so
far never done.
