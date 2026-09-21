---
number: 12
title: Three CSS edits, three wrong anchors, and half a stylesheet gone
date: 2026-09-20
area: site
files: crates/cairns-site/src/assets/style.css
---

# 12. Three CSS edits, three wrong anchors, and half a stylesheet gone

The layout work was a run of small fixes - one page width everywhere, a simpler
entry list, one hover effect instead of three, a one-line footer, the contents
list moved out of the flow. Each was correct. The way I was making them was not.

I was editing by locating a marker string and splicing around it. That failed
three times in a row, in two different ways.

**In Rust, `cargo fmt` moved the target.** A block I had read as one line came
back as six, so the marker no longer matched and the edit silently did nothing -
`log.rs`, `entry.rs` and `serve.rs` all went through a compile error that only
said a later name was missing.

**In CSS, I assumed document order and was wrong.** The edit took everything
between `.toc {` and a marker far below it. `.toc` had been inserted near the
top of the file, so the splice removed 194 lines in between: the masthead, the
nav, the chips, the sort control, the status line and the whole entry list. The
page still rendered. It rendered with nav links run together, a default blue
link where the title should be and bare text where the pills should be, which is
what a stylesheet missing its middle looks like.

Nothing caught it. It compiled, the tests passed, `check` passed, every page
returned 200 and every page validated - because a stylesheet that is missing
half its rules is still a valid stylesheet. It took a person looking at the
screen, which is the third time in this session that has been the thing that
found the bug.

It happened a fourth time before the entry was finished. The `resolves`
validation in `problems()` was written, missed its anchor, and never landed -
and in the meantime I said in conversation that `check` rejected an invalid
`resolves`, which it did not. The code compiled, the tests passed, and the
claim was simply wrong. The last edit was made by line position after reading
the file, and the validation now has tests of its own.

The stylesheet was rewritten whole rather than patched back, and each section
asserted present afterwards.

The lesson is about method, not CSS. An edit that deletes a region has to verify
the region first, and a marker-based splice does not - it silently does the
wrong thing when the file is not shaped the way it was last read. Replacing an
exact known string is safe because a miss is a no-op; splicing between two
indices is not, because a miss is a deletion.

**Still unknown:** nothing.
