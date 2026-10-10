---
number: 35
title: Evidence per section, other worklogs by name, and a build that grows with the log
date: 2026-10-09
area: spec, core, site, cli
files: crates/cairns-core/src/doc.rs#section_evidence, crates/cairns/src/workspace.rs#published, crates/cairns-core/src/log.rs#Lookup, crates/cairns-core/src/config.rs#WorkspaceTarget, crates/cairns-site/src/lib.rs, docs/spec/config.md
started: 2026-10-09T23:13:58-07:00
took: 12m
---

# 35. Evidence per section, other worklogs by name, and a build that grows with the log

## A section rests on its own entries

A reference page's `worklog:` was one list for the whole page, so which entry
backed which section was lost. A section now gives its own, in a comment under
its heading - nothing on a forge, "from" and the entries under the heading on
the site:

```markdown
## The index table
<!-- worklog: 40, 52, piney:361 -->
```

An entry it names says "Documented in The archive › The index table", linking
to the section. `cairns doc cite <page> 40 --section "The index table"` writes
or extends the comment; it refuses a heading the page does not have, listing
the ones it does. `a_section_cites_its_own_evidence`,
`a_section_is_cited_under_its_heading`.

## Other worklogs, by name

`[workspace]` names other worklogs, and `[[piney:361]]` - in prose, in
`worklog:`, in a section's comment - names an entry in one. A local path is
read for titles and checked. That works only where the other repository is
checked out beside this one; in CI it is not, and the path alone failed. So a
workspace may name a path and a URL, the URL used when the path is not there.

What a URL can tell is the site's `ids.json`, which every site now publishes:
each entry's number, slug and title, 65 KB for piney_apples' 407 entries
against 5 MB for `log.json`. It is fetched with `ureq` - pure Rust, TLS
included, which took the release binary from 6.8 MB to 8.5 MB - kept an hour
in `.cairns/workspaces/`, and gives a URL workspace the same titles and the
same `check` a local one has. Tested against piney_apples served locally: a
reference to its entry 99999 was refused with the repository not checked out.
With no network the stale copy is used, and with none at all an entry is
linked by number: every site now serves `/<number>/`, which sends a reader on to
the entry, `#section` kept. Offline, `check` passes in 0.02s rather than
waiting on a dead host. A known entry with no address - a workspace whose
`cairns.toml` has no `base_url` - is shown as its name with its title on hover;
the first version left it as the brackets it was written in.

## A build that grew with the square of the log

Asked whether hundreds of entries would make it slow: piney_apples' 407 built
in 0.5s, and five copies of it - 2,035 entries - in 3.0s, six times as long for
five times as much. Every link on every page was resolved by walking the
entries - by number, by path, by attachment folder, and the pages by path -
so the work went as links times entries. `Log` now indexes them once, on
first use: 2,035 entries build in 0.8s, 407 in 0.4s.

What a reader downloads still grows with the log: at 2,035 entries the index
is 1.26 MB (250 KB as GitHub Pages serves it, compressed) and `search.json`
7.3 MB (2.6 MB). The search index is the one to watch.

In the index's search, an entry's number and Enter goes to the entry.

**Still unknown:** whether a meta refresh fallback and a script preserving #section both land where they should in a real browser - headless Edge renders nothing after a page navigates away, so the redirect was checked as HTML, not seen
