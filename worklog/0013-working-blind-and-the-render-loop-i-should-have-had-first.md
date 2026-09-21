---
number: 13
title: Working blind, and the render loop I should have had first
date: 2026-09-20
area: site
files: crates/cairns-site/src/assets/style.css, crates/cairns-site/src/html.rs
---

# 13. Working blind, and the render loop I should have had first

Four rounds of "still cramped", "so fucking tacky", "the page is too thin",
before the accurate version of the complaint arrived: *you're not even using a
headless browser, you're throwing shit at a wall.*

That was exactly right. Every CSS change in this project had been written,
compiled, verified by grepping the rendered HTML for class names, and handed to
a person to look at. I was using the reader as the render loop. Everything I
called verification - the page returns 200, the tags close, the rule is in the
stylesheet - tests that the bytes are what I wrote, never that the result looks
like anything.

It is also how [12](0012-three-css-edits-three-wrong-anchors-and-half-a-stylesheet-gone.md)
went unnoticed. Half a stylesheet was missing and every check I had still
passed, because a stylesheet with its middle deleted is a valid stylesheet.

## The loop

Headless Edge is already on this machine:

```sh
"$EDGE" --headless --disable-gpu --hide-scrollbars \
  --window-size=1440,1700 --virtual-time-budget=2500 \
  --screenshot=/tmp/shot.png http://127.0.0.1:8899/
```

Three seconds a shot, against `cairns serve`. The first screenshot answered the
question four rounds of guessing had not: the page had no structure at all. A
flat column on a flat ground, a small masthead lost at the top, controls
floating unattached above an undifferentiated list. Not "cramped" - *empty*, in
the way a page with nothing holding it together is empty.

## What it became

A rail and a column. The rail is sticky and carries the identity, the
navigation with the current page marked, and - where they belong - the area
filters, as a facet list with counts rather than a stack of pills that looked
like form fields. The column is the same width on every page.

The reference section gets the same treatment: the docs tree in the rail,
folders naming themselves and indenting what is inside them, the current page
marked. A docs section whose only navigation is an index you have to go back to
is a docs section nobody navigates.

Entry rows became one target rather than a title-sized one, with the number
legible in a left gutter instead of buried in the meta line. Search stopped
being an outlined box with a segmented control bolted beside it and became a
filled field with an icon, and two words you can click.

None of those were ideas I could have had from the source. Each one was
obvious within a second of looking at a picture.

**Still unknown:** whether the light theme holds up. Headless here inherits the
system's dark mode, so every screenshot in this entry is the dark one, and the
light palette has still never been looked at by anything.
