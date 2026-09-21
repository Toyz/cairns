---
number: 8
title: Distribution, and a worklog a model can read
date: 2026-09-20
area: cli, build, skill
files: crates/cairns/src/mcp.rs, .github/workflows/release.yml, packaging/homebrew/cairns.rb
---

# 8. Distribution, and a worklog a model can read

Two things that both amount to the same question - who can get at this - and one
correction to work done earlier in the day.

## The binary

CI runs fmt, clippy with `-D warnings`, the tests, and `cairns check` against
this repo's own log, so the tool is held to the standard it holds everyone else
to. A second job builds on the declared MSRV, 1.88, which is what let-chains in
`config.rs` cost. Tagging cuts binaries for five targets - macOS on both
architectures, Linux gnu and musl, Windows - with a `SHA256SUMS` beside them,
and there is a Homebrew formula template in `packaging/`.

`cargo publish --dry-run` passes, and the packaged crates carry what they need:
the binary's `templates/SKILL.md`, the site crate's CSS and JS.

### The actions were years out of date

Every workflow was written against the versions in my head, and every one was
several majors behind:

| | written | current |
| --- | --- | --- |
| `actions/checkout` | v4 | v7 |
| `actions/upload-artifact` | v4 | v7 |
| `actions/download-artifact` | v4 | v8 |
| `actions/upload-pages-artifact` | v3 | v5 |
| `actions/deploy-pages` | v4 | v5 |
| `softprops/action-gh-release` | v2 | v3 |

Bumping a major is not the same as bumping a number, so each one's `action.yml`
was read at the new tag to confirm the inputs still exist - `merge-multiple`,
`path`, `files`, `generate_release_notes`, the `page_url` output. They all do.
The workflows themselves have not been run; they are eyeballed YAML until a
push proves otherwise.

### Installing without a toolchain

`install.sh` picks the target from `uname`, resolves the latest tag from the
API unless `CAIRNS_VERSION` says otherwise, and - the part that matters in
anything piped to a shell - checks the download against the release's
`SHA256SUMS` *before* unpacking it. On Linux it prefers the gnu build and falls
back to musl where there is no glibc.

Tested against a fake release served locally, because a script nobody has run
is a script that does not work:

```
good checksum      -> installed, `cairns --version` runs
tampered archive   -> "checksum mismatch", exit 1, nothing installed
missing from sums  -> "not listed in SHA256SUMS", exit 1
```

## MCP

`cairns mcp` speaks JSON-RPC 2.0 over stdin and stdout. Read-only by default:
tools for listing, reading, searching, open questions and `check`, and
resources at `worklog://entry/50`, `worklog://open`, `worklog://index` and
`worklog://log.json`. `--write` adds one tool for writing an entry, and its
schema enumerates only the areas the project declares, so a model cannot invent
one.

It is hand-rolled rather than taken from `rmcp`. The surface used here is small
and synchronous and wants no async runtime; the cost is that protocol drift is
ours. One hedge against that: `initialize` echoes back whatever
`protocolVersion` the client asked for rather than insisting on a known one,
because nothing in this server's tools or resources has ever differed between
versions and refusing a newer client would break a host for no gain.

Driven with a real client, writing through it produces an entry indistinguishable
from one the CLI wrote, `supersedes` and all - and reading the entry it corrects
immediately says so:

```
worklog_read 47  ->  NOTE: something claimed here was corrected by entry 56.
```

The reason this was half a day rather than a week is a decision from
[1](0001-the-format-is-the-asset-so-the-spec-is-written-down-first.md): the core
has no filesystem in its API and `log.json` is already the canonical
serialisation, so the server is a transport over a document that existed, not
new logic.

Worth being plain about where it does *not* help. In a host with a shell it adds
nothing - the skill `init` writes teaches the CLI, and every entry in this log
was written that way. MCP is for the host that has no terminal.

**Still unknown:** whether echoing the client's protocol version is the right
hedge or a way to fail confusingly later, since it has only been driven by a
client this repo wrote.
