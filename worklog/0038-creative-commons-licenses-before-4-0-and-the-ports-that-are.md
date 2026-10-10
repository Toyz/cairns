---
number: 38
title: Creative Commons licenses before 4.0, and the ports that are not the license
date: 2026-10-10
area: spec, cli, site
files: crates/cairns/src/license.rs#creative_commons, crates/cairns-site/src/html.rs#license_url, docs/spec/config.md
started: 2026-10-09T23:59:58-07:00
took: 2m
---

# 38. Creative Commons licenses before 4.0, and the ports that are not the license

[[37]] knew Creative Commons only at 4.0, and said so. Older versions are
common in exactly the places a log's text license would be - docs written
years ago under CC BY-SA 3.0 - so [[crates/cairns/src/license.rs#creative_commons]]
now reads every attribution license SPDX names, 1.0 to 4.0.

A file names one in one of four ways, and each is read:

| form | example |
| --- | --- |
| notice | Creative Commons Attribution-ShareAlike 3.0 Unported License |
| legal code | Creative Commons Legal Code / Attribution 3.0 Unported |
| address | `creativecommons.org/licenses/by-nc-sa/2.0/uk/` |
| short form | CC BY-SA 4.0 |

The elements are read as a set, so 1.0's "NoDerivs-NonCommercial" comes out in
SPDX's order as `CC-BY-NC-ND-1.0`, and the result must be one of SPDX's ids or
nothing is named.

## Ports

Before 4.0 a license could be ported to a jurisdiction, and a port is a
different license: "Attribution 3.0 Germany" is `CC-BY-3.0-DE`. The trap is
reading it as `CC-BY-3.0` because the version matched. So for an older
version, the word after it must say there is no jurisdiction (unported,
generic, the end of a sentence, "license") or name one SPDX has (Germany,
Austria, the US, IGO, and the rest). "Attribution 3.0 Spain" - a real port,
not in SPDX - names nothing, and so does "CC BY 3.0 for a while", where the
next word could be anything. 4.0 has no ports, so anything may follow it.

The site's link follows: [[crates/cairns-site/src/html.rs#license_url]] built
a Creative Commons address by splitting off the last part as the version,
which a ported id breaks (`CC-BY-SA-3.0-DE` → `by-sa-3.0/de`). It now finds
the version part and puts the jurisdiction after it:
`creativecommons.org/licenses/by-sa/3.0/de/`.

**Still unknown:** nothing.
