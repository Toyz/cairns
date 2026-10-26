---
number: 42
title: serve answered one connection at a time, and a browser's spare one stopped it
date: 2026-10-25
area: cli
files: crates/cairns/src/serve.rs#serve, crates/cairns/tests/cli.rs
---

# 42. serve answered one connection at a time, and a browser's spare one stopped it

`cairns serve` hung on a Linux machine, on NFBM. Every command run against
NFBM here finished in under half a second, with 0.10.2 and with main; the
hang was in the server.

It answered one connection at a time, blocking on each with no timeout.
Browsers open connections they may never send a request on - Chrome and
Firefox do it to save a round trip later, and readily on Linux - and one such
socket left every request after it waiting, forever. Reproduced here: one
idle connection open, and a plain `GET /` got no answer in five seconds.

[[crates/cairns/src/serve.rs#serve]] answers each connection on a thread of its
own with a ten-second read timeout; the site is shared as an `Arc` and
replaced whole on a rebuild, so a thread serves the site as it was when its
connection arrived. With three idle connections held open, each page answers
in about ten milliseconds, and a touched entry still rebuilds and reloads.

`--open` had a second way to hang: it waited for `xdg-open` to exit before the
server accepted anything, and on Linux `xdg-open` can stay until the browser
it launched does - so that browser's first request went unanswered. It is
started and not waited for now.

`serve_answers_while_a_connection_sits_idle` in [[crates/cairns/tests/cli.rs]]
runs the server on port 0, holds a connection open, and asks for a page; it
fails on the old server and passes on this one.

**Still unknown:** Whether this was the whole of the hang on Linux - it is the one found by looking; the machine that hangs has not run the fix yet.
