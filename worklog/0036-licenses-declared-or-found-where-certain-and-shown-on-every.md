---
number: 36
title: Licenses: declared, or found where certain, and shown on every page
date: 2026-10-09
area: spec, core, site, cli
files: crates/cairns/src/license.rs#detect, crates/cairns/src/license.rs#identify, crates/cairns-site/src/html.rs#licenses_html, crates/cairns/src/pages.rs#infer_repository, crates/cairns-core/src/config.rs, docs/spec/config.md
started: 2026-10-09T23:51:58-07:00
took: 4m
---

# 36. Licenses: declared, or found where certain, and shown on every page

A published log is text someone may want to quote, and code someone may want
to copy; neither said under what terms. Now the footer of every page does -
"Text and code MIT", or "Text CC-BY-4.0 · Code MIT" when they differ - with
`<link rel="license">` in the head and `<rights>` in the feed.

## Declared

`project.license` is the code's license, an SPDX expression.
`project.text_license` is the log's and the reference's, and is the code's when
not given - the common case for a project's own notes, but a log written to be
read is often licensed apart from the code, so it gets its own field rather
than being assumed.

## Found

When `license` is not given, [[crates/cairns/src/license.rs#detect]] looks
where an answer is certain, in order:

1. a manifest's own declaration - `Cargo.toml` (`[package]`, then
   `[workspace.package]`), `package.json`, `pyproject.toml`;
2. license files at the root, each identified by phrases only that license's
   text contains ([[crates/cairns/src/license.rs#identify]]). `LICENSE-MIT`
   and `LICENSE-APACHE` side by side become `MIT OR Apache-2.0`.

A file whose wording matches nothing is listed but not named. Nothing is
guessed from a partial match: a wrong license on every page is worse than
none. `cairns build` says which license it used and where it came from, or
that it found none and how to declare one.

## Linked

Each license in the expression links to the repository's own file whose text
is that license, when there is one; otherwise to the Creative Commons deed for
CC licenses, or to spdx.org. Linking the repository's file needs
`project.repository`, which piney_apples never set - so it is now worked out
like `base_url`, from CI (`CI_PROJECT_URL`, `GITHUB_SERVER_URL` +
`GITHUB_REPOSITORY`) or a GitHub or GitLab `origin`
([[crates/cairns/src/pages.rs#infer_repository]]). That also fixes its README
and file links, which had been pointing nowhere.

Checked on this repo (MIT, from `Cargo.toml`, linking `LICENSE`) and on
piney_apples (`MIT OR Apache-2.0`, each half linking its own file), and with
`text_license = "CC-BY-4.0"` on a scratch copy.

**Still unknown:** Whether `text_license` should ever be found rather than declared - a `LICENSE-docs` or a CC file beside the code's could say it, but nothing seen yet does.
