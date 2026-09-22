---
number: 11
title: The log's own links were dead on its own site
date: 2026-09-20
area: site
files: crates/cairns-site/src/html.rs, crates/cairns-site/src/feed.rs
---

# 11. The log's own links were dead on its own site

The skill tells you to correct an earlier entry by linking to it:

```markdown
[[12]]
```

That is right in the repo and right on GitHub. On the generated site it is a
404, because there is no `.md` file there - an entry is a directory named for
its number and its slug. Every cross-entry link in this log, which is the
convention the format exists to encourage, was broken on the thing built to
show the format off:

```
$ curl -o /dev/null -w '%{http_code}' .../7-migrate/0004-the-write-path.md
404
```

The fix is translation rather than a new convention. A link whose filename
parses as `NNNN-` and matches an entry becomes that entry's page; anything else
relative is a file in the repository and points there. Both spellings stay
correct where they already were.

Two things fell out of doing it properly. A relative link inside an entry is
relative to the *entries directory*, so `../docs/spec/entry.md` is
`docs/spec/entry.md` at the repository root - the first attempt pasted the
`../` into the URL and produced
`github.com/Toyz/cairns/blob/HEAD/../docs/spec/entry.md`. And the feed carries
the same prose with no page to resolve a relative link against, so entry links
render absolute there and relative on the site.

## Why this went unnoticed

Every page returned 200 and every page validated. The broken thing was a link
*inside* rendered prose, which nothing checks - not `check`, which reads front
matter, and not the HTML validation, which only asks whether tags close.

A link checker over the rendered site would have caught it in a second, and
there is still no such check. That is the gap, rather than the bug.

**Still unknown:** whether `check` should follow cross-entry links in prose and
fail on one that points at no entry. It would want parsing markdown in the core,
which so far has stayed out of it.
