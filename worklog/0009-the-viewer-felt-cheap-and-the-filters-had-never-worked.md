---
number: 9
title: The viewer felt cheap, and the filters had never worked
date: 2026-09-20
area: site, cli
files: crates/cairns-site/src/assets/style.css, crates/cairns-site/src/html.rs, crates/cairns/src/serve.rs
---

# 9. The viewer felt cheap, and the filters had never worked

Three rounds of "still cramped" before the actual problem was named: it felt
cheap. That is a different complaint from a spacing one, and spacing is all I
had been adjusting. A page with no identity reads as a default HTML list
because that is exactly what it was.

## A direction, rather than another nudge

A worklog is a lab notebook: numbered, dated, written in prose, read slowly. So
the page is set like one. Prose and titles are a serif; the chrome - nav, chips,
meta, code - is a sans; the entry number is large and quiet in the gutter and is
the one piece of furniture on every page. The ground is warm paper rather than
white, and the accent is a rust that marks the things a notebook marks: a
correction, the open question at the foot of an entry, the section you are in.

No webfonts. The stacks resolve to something good without a request, because a
log that only looks right online is a log that does not look right.

The measurements that changed, after two passes that did not:

```
              first     second    now
base type     16px      17px      17px, prose in a serif
entry title   17px      19px      22px
summary       14px      16px      17px
index width   704px     768px     992px
reading width 704px     592px     704px, with the contents list beside it
```

The width was the part I kept getting wrong in both directions. An index and a
reading column want different things, and giving them one number makes one of
them wrong. The index is wide and puts the date and areas in their own column
on the right; an entry page is a reading column with its contents list in the
margin.

## The filters had never worked, and said nothing

```css
.entries li { display: flex; }
```

The filter code sets `hidden` on a row. The browser's own `[hidden]
{ display: none }` is one class less specific than `.entries li`, so every
hidden row kept its `display: flex` and stayed on screen. Clicking an area chip
updated the button, updated the URL, updated the count in the status line, and
changed nothing visible.

I introduced it in the same pass that made rows flex, which is worth saying
plainly: the redesign broke a feature that had worked, and no test, no build and
no console message registered it. It took a person clicking one.

`[hidden] { display: none !important; }` sits above every display rule now, with
a comment saying why, because the next layout change would do it again.

## A contents list, and pages that reload themselves

Entries run to several `##` sections - hellbender's 55 carry 143 between them -
and there was no way to move around inside one. Headings now get anchors, and
an entry with more than one section gets a contents list: sticky in the margin
on a wide screen, above the prose on a narrow one, with subsections nested.

`cairns serve` was already rebuilding when an entry changed, but the browser had
no way to find out, so it meant nothing without a manual refresh. Pages served
by it now poll a counter and reload when it moves. Polling rather than an event
stream, because the server is single-threaded on purpose and one held-open
stream is a server answering nothing else. The script is injected on the way out
and is never in what `build` writes - checked by a test, since a production site
quietly polling a dead endpoint would be a nasty thing to ship.

**Still unknown:** whether the serif stack resolves well away from macOS. Iowan
and Charter are Apple faces; elsewhere it should fall to Palatino or Georgia,
but I have only looked at it on one machine.
