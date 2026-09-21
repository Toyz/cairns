# cairns

A worklog kept as numbered markdown entries: write them, check them, publish
them.

A cairn is a stack of stones marking a trail so whoever comes next does not
have to work the route out again. That is what a worklog entry is for, and
especially the entries recording what *did not* work - the format that turned
out not to be what it looked like, the encoding that failed, the function that
was dead code. Code says what; a worklog says how it was found out, and why it
is that way.

```
worklog/0050-kreash-mix-is-the-end-of-a-table.md    the entries
WORKLOG.md                                          generated index
cairns.toml                                         project, areas, targets
```

Each entry carries front matter - number, title, date, area, and optionally the
files it is about, a summary, and the earlier entries it corrects. The log is
append-only: a claim that turns out to be wrong is overturned by a later entry
that links back to it, never by editing the original. Being wrong on the record
is the point.

## Status

Usable. Every command works except the `git-branch` publish target, which is
reserved in favour of a CI job. This repo keeps its own worklog with it.

Tested against two real logs rather than fixtures. Hellbender's 55 entries: the
generated index matches its committed one byte for byte apart from one
documented change, `init` adopts the repo cleanly, and `build` produces 57
pages that all parse. Amber's 192-entry single-file log migrates with its 7,329
prose lines identical on both sides, and renders in 95 ms.

| | |
| --- | --- |
| `cairns next` | the number the next entry takes |
| `cairns init` | set up a repo: config, skill, and freeze existing slugs |
| `cairns new` | start an entry, numbered and dated, index refreshed |
| `cairns index` | regenerate `WORKLOG.md` from the entries |
| `cairns check` | numbering sound, front matter complete, filenames honest, index current |
| `cairns open` | what the log still does not know, across every entry |
| `cairns export` | the whole log as `log.json` |
| `cairns build` | render the site, feed, search index and `log.json` |
| `cairns serve` | the site on localhost, rebuilt as entries change |
| `cairns publish` | deliver the payload to a configured target |
| `cairns migrate` | convert a single-file worklog into numbered entries |

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/Toyz/cairns/main/install.sh | sh
cargo install cairns                    # from crates.io
brew install Toyz/tap/cairns            # macOS and Linux
```

The script takes the right binary for your platform from the latest
[release](https://github.com/Toyz/cairns/releases) and checks it against that
release's `SHA256SUMS` before installing anything. `CAIRNS_VERSION` pins a tag,
`CAIRNS_BIN_DIR` chooses where it lands (default `~/.local/bin`). Binaries are
built for macOS on both architectures, Linux gnu and musl, and Windows.

Nothing needs installing to *read* a published worklog; the binary is for
keeping one.

## Getting started

```sh
cairns init                                   # config, skill, index
$EDITOR cairns.toml                           # the areas are yours to choose
cairns new "What you found out" --area design
cairns check
cairns serve --open                           # look at it
```

`init` in a repo that already keeps a worklog is safe: it freezes the slug of
every entry whose filename no longer matches its title, so nothing is renamed
and no published link breaks, and it will not touch a hand-written skill.

## Using it from a model

```sh
cairns mcp            # read-only, over stdin and stdout
cairns mcp --write    # also allows writing entries
```

A stdio MCP server, for a host with no shell. Tools cover listing, reading,
searching, the open questions and `check`; `--write` adds one for writing an
entry, and its schema offers only the areas this project declares. Resources -
`worklog://entry/50`, `worklog://open`, `worklog://index`, `worklog://log.json` -
let a model pull one entry into context instead of reading the whole log.

```json
{ "mcpServers": {
    "worklog": { "command": "cairns", "args": ["mcp"], "cwd": "/path/to/repo" }
} }
```

In a host that already has a shell this adds little - `cairns init` writes a
skill that teaches the CLI, which is enough. It earns its place where there is
no terminal.

## Releasing

The tag is the version. `Cargo.toml` says `0.0.0-dev` and never says anything
else - pushing `v0.2.0` makes CI rewrite the workspace version from the tag,
build the binaries and attach them with their checksums. Nothing to bump by
hand, and no way for a tag and a manifest to disagree.

```sh
git tag v0.2.0 && git push origin v0.2.0
```

Publishing to crates.io is off unless the repository variable
`PUBLISH_TO_CRATES` is `true` and a `CARGO_REGISTRY_TOKEN` secret exists.

## The format

The spec is in [docs/spec/](docs/spec/) and is versioned separately from this
tool, because the tool is one implementation and the format is the part that has
to survive. A worklog whose tooling is lost is still a worklog - which is why
the markdown is the source of truth, everything else is generated and
disposable, and nothing here needs a database.

- [entry.md](docs/spec/entry.md) - the entry file
- [config.md](docs/spec/config.md) - `cairns.toml`
- [log-json.md](docs/spec/log-json.md) - the canonical export
- [publish.md](docs/spec/publish.md) - targets and the payload

## What the site is

Entry pages with prev/next and clean URLs, an index with area filters and
search, an Atom feed, and an **open questions** page collecting every
unresolved `**Still unknown:**` in the log - 33 of them across hellbender's 55
entries, which is the page that log never had.

An entry that corrects an earlier one says `supersedes: 6` in its front matter,
and the entry it overturns then says so at the top of its own page. A reader
arriving from a search is told the claim was revisited before they read it.

Every page carries its own metadata, so a link to one entry unfurls with its
title and summary rather than the repo's name.

Point `[site] readme` at a markdown file and it becomes an About page, so a
reader arriving at a worklog can find out what the project is. Its relative
links are rewritten into `[project] repository`, because `docs/spec/entry.md`
means a file in the repo, not a page on the site.

```toml
[project]
repository = "https://github.com/Toyz/cairns"
[site]
readme = "README.md"
```

## Publishing

`build` is pure and `publish` has the side effects. The `dir` target exists and
publishes incrementally, comparing per-entry hashes against the `log.json`
already at the destination - no state file to go stale. For GitHub Pages, a CI
job runs `build` and hands the directory to `deploy-pages`; the workflow is in
[docs/spec/publish.md](docs/spec/publish.md).

An HTTP ingest target is specified and reserved, so that today, with no server
anywhere:

```sh
cairns publish --target hosted --dry-run
```

prints the exact request body such a server would receive. The contract can be
designed against a real log before it is built, and if it never is built,
nothing has been spent - `log.json` is already what the local site renders from.

## Origin

Extracted from the worklog kept while reverse engineering
[hellbender](https://github.com/Toyz/hellbender), where the format earned its
keep over 55 entries. See [worklog/](worklog/) - this repo keeps its own.

MIT.
