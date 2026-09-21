---
number: 16
title: A field carried all the way through and shown nowhere
date: 2026-09-20
area: site, spec
files: crates/cairns-site/src/html.rs
---

# 16. A field carried all the way through and shown nowhere

`files` is in the spec as "paths the entry is about". `cairns new --files`
writes it, the parser reads it, `log.json` exports it, and every one of this
log's entries has one. The site rendered it nowhere at all.

It had passed through the entire pipeline without ever arriving anywhere a
reader could see it, which is why nothing caught it: there is no test for
"appears on the page", and the field was present at every point anyone checked.

It sits under the dateline now, each path linked into the repository through
the same resolution the prose uses - so an entry about a bug in `html.rs` is
one click from `html.rs`.

The general shape is worth naming, because this project has hit it twice today:
a field can be parsed, validated, exported and still be dead, and none of
parsing, validating or exporting will tell you. The other was the traffic-light
status badge, which rendered but looked like a different website. Both needed
somebody to look at the page.

**Still unknown:** whether a reference page's `covers` deserves the same. It is
in `extra` and passes through untouched; hellbender fills it with glob-ish
strings like `ART*.ACT, FOG*.MAP` that say what a page accounts for, which is
the same kind of claim `files` makes and is equally invisible.
