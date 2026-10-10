---
number: 37
title: The text's license, found where a file says the text has one
date: 2026-10-09
area: spec, cli
files: crates/cairns/src/license.rs#detect, crates/cairns/src/license.rs#creative_commons, docs/spec/config.md
resolves: 36
started: 2026-10-09T23:57:02-07:00
took: 2m
---

# 37. The text's license, found where a file says the text has one

[[36]] left `text_license` as declared only. It can be found, in the same
spirit as the code's: only where a file says it outright.

[[crates/cairns/src/license.rs#detect]] now takes the worklog's and the
reference's directories and reads a license as the text's when:

- its file is named for the text - `LICENSE-docs`, `LICENSE.content.md`,
  `COPYING-TEXT`;
- it sits in the directory it covers - `docs/LICENSE`, `worklog/LICENSE`;
- it is a Creative Commons `CC-BY*` license at the root beside a software
  license. Creative Commons advise against their licenses for code, so
  `LICENSE` (MIT) beside `LICENSE-CC-BY` is code and prose. Before this it
  would have been shown as `MIT OR CC-BY-4.0` - a choice nobody offered. CC0
  is left out of the rule: it is used for code as often as for anything.

A docs license is often not the legal code but the one-line notice -
"licensed under a Creative Commons Attribution-ShareAlike 4.0 International
License" - so [[crates/cairns/src/license.rs#creative_commons]] knows all six
4.0 licenses by the full name either form carries, narrowest first.

The text's files are in different places rather than offered side by side, so
two different licenses among them is a disagreement and nothing is shown -
unlike `LICENSE-MIT` and `LICENSE-APACHE` at the root, which stay a choice.
`cairns build` says the text's license and where it came from, as it does the
code's.

Checked on a copy of this repo with a one-line CC BY-SA notice in
`docs/LICENSE.md`: the footer reads "Text CC-BY-SA-4.0 · Code MIT", each
linking its own file.

**Still unknown:** nothing. CC 3.0 and older are not recognised; no project seen uses one for its notes.
