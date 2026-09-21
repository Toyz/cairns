---
title: Publishing
status: partial
worklog: 1, 6, 8
spec_version: 1
---

# Publishing

Two commands, split along the line between the deterministic part and the part
with side effects:

```
cairns build      entries -> log.json -> ./site      pure, repeatable, testable
cairns publish    deliver the payload to a target    the side effect
```

`build` never reaches outside the working tree. Everything that writes
somewhere else is a publish target, and every target is configured in
`cairns.toml`.

## The payload

Whatever the target, the payload is the same thing:

```
log.json          the canonical document - see log-json.md
assets/           anything the entries reference
site/             the rendered site, for targets that serve files
```

A target that serves files takes all of it. A target that ingests structured
data takes `log.json` and the assets. There is no third representation and no
target-specific serialisation, which is the whole reason a server can be added
later without touching the parts that make it.

## Targets

```toml
[[publish]]
name = "site"
type = "dir"
path = "site/"

[[publish]]
name = "pages"
type = "git-branch"
branch = "gh-pages"
```

| type | status | does |
| --- | --- | --- |
| `dir` | implemented | writes the payload to a local directory |
| `git-branch` | **reserved** | commits the payload to an orphan branch and pushes |
| `http` | **reserved** | POSTs `log.json` to an ingest endpoint |

`git-branch` is specified but not built. A CI job that runs `cairns build` and
hands the directory to the host's own deploy action does the same work without
this tool force-pushing anything, which is the safer default and the one most
projects already have:

```yaml
- run: cargo run -p cairns -- check
- run: cargo run -p cairns -- build --out site
- uses: actions/upload-pages-artifact@v5
  with: { path: site }
```

It stays in the spec because a project publishing somewhere without CI still
wants one command.

`http` is specified but deliberately not implemented. The shape is written down
now so that the payload, the incremental protocol and the config surface are
designed against it, and so that:

```
cairns publish --target hosted --dry-run
```

prints the exact request body a server would receive - today, from a real log,
with no server in existence. The ingest contract can be designed, reviewed and
argued with before any of it is built, and if it never is built, nothing has
been spent: `log.json` is already what the local site renders from.

## Incremental

A target holds a manifest - for `dir` and `git-branch` that is simply the
`log.json` already there. Publish compares `content_hash` per entry and reports,
or sends, only what differs.

One mechanism serves every target. There is no "last published" state file in
the working tree, so nothing can go stale, nothing needs cleaning up after a
failed run, and a fresh clone publishes identically to a dirty one.

## Secrets

`cairns.toml` is committed. It therefore never contains a credential.

A target's `token` must be an environment variable reference - a value beginning
with `$`:

```toml
token = "$CAIRNS_TOKEN"
```

`cairns check` fails on a `token` that does not, which catches the mistake in
the commit that introduces it rather than after the push.
