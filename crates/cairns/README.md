# cairns

A worklog kept as numbered markdown entries: write them, check them, publish
them.

A cairn is a stack of stones marking a trail so whoever comes next does not have
to work the route out again - which is what a worklog entry is for, and
especially the ones recording what *did not* work.

```sh
cairns init                                   # config, skill, index
cairns new "What you found out" --area design
cairns check
cairns build --out site
```

Entries are markdown with front matter; the index, the site, the feed and
`log.json` are all generated from them. The log is append-only: a claim that
turns out wrong is overturned by a later entry that links back to it, and the
entry it overturns says so on its own page.

Full documentation, and the format spec, at
<https://github.com/Toyz/cairns>.
