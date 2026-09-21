---
number: 14
title: Hellbender's own docs tree crashed the renderer
date: 2026-09-20
area: site, adoption, core
files: crates/cairns-site/src/html.rs, crates/cairns-core/src/doc.rs
---

# 14. Hellbender's own docs tree crashed the renderer

Enabling cairns on hellbender - the repo the format came from - took a
`cairns.toml` and one `init`, which froze thirteen slugs and left the
hand-written skill alone because it carries no marker. Then `build` died:

```
thread 'main' has overflowed its stack
fatal runtime error: stack overflow, aborting
```

Three faults, none of which this repo's own docs tree could have shown,
because this repo's docs tree is one directory with a README in it.

## The docs root is its own parent

`docs/README.md` is the index *of* `docs/`, so its slug is the directory it
names - which at the root is the empty string. Its section is the directory
above it, which at the root is also the empty string. The tree renderer asked
for the folders inside a section, got a folder whose slug *was* that section,
recursed into it, and asked the same question again.

Two lines guard it now, and the regression test renders a tree with a root
README: a regression there is a crash, not a failed assertion.

The test earned itself immediately. It caught a second fault I had already
convinced myself was fixed - `docs//index.html`, the root README being given a
page of its own as well as being the index. The build "worked" and wrote 88
files; I counted the files and did not read them.

## The rail was empty on the one repo that needed it

Folders were derived from index pages, so a directory without a README was not
a folder and its pages belonged to a section that did not exist. Hellbender has
four such directories - `formats`, `engine`, `content`, `port` - and every page
in them. Its reference rail rendered a heading and nothing else.

Folders come from the directories themselves now. One with its own page is a
link to it; one without is a label. Deriving structure from what a project
happens to have written is how you get a tree that is empty for the projects
with the most in them.

## `worklog: 7 to 20`

`docs/port/plan.md` cites a range. The spec said comma-separated numbers, so
`check` rejected it, correctly and uselessly - a page established over a run of
entries is naturally written that way, and hellbender's was, years before this
tool existed. `7 to 20` and `7-20` both expand now, inclusive.

The alternative was editing their page to match the parser. The parser was
wrong about what people write.

**Still unknown:** whether the same is true of `covers`, which hellbender fills
with glob-ish strings like `ART*.ACT, FOG*.MAP` and which cairns currently
passes through untouched. It may want to be structured, or it may be prose that
is better left alone.
