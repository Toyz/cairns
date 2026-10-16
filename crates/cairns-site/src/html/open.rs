//! The open questions page: what the log and the reference do not know yet.

use super::*;

/// Everything the log has not closed out, in one place.
///
/// One card per entry, newest first, the entry named above its questions
/// rather than under them: a question is a fragment out of context, and the
/// reader needs the context first. A trailer that is a list is one card with
/// its items, folded past five, because a triage entry carrying thirty
/// questions forward otherwise buries everything after it.
pub fn open_questions(log: &Log) -> String {
    let base = log.project.base_url.trim_end_matches('/');
    let links = Links {
        log,
        from_dir: entry_dir(log),
        rel: "../",
        base: None,
    };

    let asked: usize = log
        .open_questions
        .iter()
        .map(|q| match q.items.len() {
            0 => question_count(&q.text),
            n => n,
        })
        .sum();
    // Closed: answered, carried elsewhere, or every question of its list one
    // or the other. A carried question is still open - where it went - but it
    // is not open *here*, and "is this still open?" is asked of this page.
    let closed = closed_entries(log);
    // The reference's own unknowns, from each page's `## Unknown` section: the
    // other half of what the project does not know, which this page used to
    // leave out entirely.
    let doc_unknowns: Vec<(&DocPage, String)> = log
        .docs
        .iter()
        .filter(|doc| !doc.is_index)
        .filter_map(|doc| cairns_core::doc::unknown_section(&doc.body).map(|text| (doc, text)))
        .collect();
    let asked = asked
        + doc_unknowns
            .iter()
            .map(|(_, text)| question_count(text))
            .sum::<usize>();
    let plural =
        |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let mut body = String::from("<article class=\"open-page\">\n<h1>Open questions</h1>\n");
    let _ = writeln!(
        body,
        "<p class=\"lead\">{} across {}{}, quoted as each left it. A later entry closes one \
         with <code>resolves:</code>, or takes it over with <code>carries:</code>.</p>",
        plural(asked, "question", "questions"),
        plural(log.open_questions.len(), "entry", "entries"),
        match doc_unknowns.len() {
            0 => String::new(),
            n => format!(" and {}", plural(n, "reference page", "reference pages")),
        },
    );
    let anything_open = !log.open_questions.is_empty() || !doc_unknowns.is_empty();

    // The same toolbar as the index, minus density: search runs over the
    // questions on this page, not the entries' full text.
    if anything_open {
        body.push_str("<div class=\"toolbar\">\n");
        body.push_str(&search_field("Search the questions"));
        // Open and closed are two views of one list rather than two lists:
        // the closed ones used to unfold underneath, and opening a few hundred
        // rows at the bottom of a page shifted everything around them.
        let _ = writeln!(
            body,
            "<div class=\"switch switch--words\" role=\"group\" aria-label=\"Show\">\n\
             <button data-state=\"open\" aria-pressed=\"true\">Open <span class=\"count\">{}</span></button>\n\
             <button data-state=\"closed\" aria-pressed=\"false\">Closed <span class=\"count\">{}</span></button>\n\
             </div>\n</div>\n<p id=\"status\"></p>",
            log.open_questions.len() + doc_unknowns.len(),
            closed.len()
        );
    }

    let mut chips = String::new();
    if !anything_open {
        body.push_str("<p class=\"empty\">Nothing open.</p>\n");
    } else {
        chips = area_chips(log);
        body.push_str("<ul class=\"questions\" id=\"entries\" data-local>\n");
        // Every entry that asked something, open or closed, newest first; the
        // switch decides which are shown.
        for row in rows(log, &closed) {
            body.push_str(&entry_row(&row, &links));
        }
        // Then the reference's, after the log's, each under its page.
        for (doc, text) in &doc_unknowns {
            body.push_str(&page_row(log, doc, text));
        }
        body.push_str("</ul>\n");
    }

    body.push_str("</article>\n<script src=\"../search.js\" defer></script>\n");

    shell(
        log,
        Page {
            title: &format!("Open questions - {}", log.project.name),
            description: "What this log has not closed out.",
            url: &format!("{base}/open/"),
            rel: "../",
            nav: Nav::Open,
            aside: &chips,
            contents: "",
            body: &body,
        },
    )
}

/// The entries whose questions are all closed here: answered, carried
/// elsewhere, or every question of its list one or the other. A carried
/// question is still open - where it went - but it is not open *here*, and
/// "is this still open?" is asked of this page.
fn closed_entries(log: &Log) -> Vec<&LogEntry> {
    log.entries
        .iter()
        .filter(|entry| {
            entry.still_unknown.is_some()
                && (!entry.resolved_by.is_empty()
                    || !entry.carried_to.is_empty()
                    || (!entry.questions.is_empty()
                        && entry.questions.iter().all(|q| !q.is_open())))
        })
        .collect()
}

/// An entry's questions, as the page lists them.
struct Row<'a> {
    number: u32,
    text: &'a str,
    items: &'a [cairns_core::log::Question],
    closed_by: Vec<u32>,
    carried_to: Vec<u32>,
}

/// Every entry that asked something, open or closed, newest first.
fn rows<'a>(log: &'a Log, closed: &[&'a LogEntry]) -> Vec<Row<'a>> {
    let mut rows: Vec<Row<'a>> = log
        .open_questions
        .iter()
        .map(|q| Row {
            number: q.entry,
            text: &q.text,
            items: &q.items,
            closed_by: Vec::new(),
            carried_to: Vec::new(),
        })
        .collect();
    for entry in closed {
        let Some(text) = entry.still_unknown.as_deref() else {
            continue;
        };
        let mut closed_by = entry.resolved_by.clone();
        let mut carried_to = entry.carried_to.clone();
        for question in &entry.questions {
            closed_by.extend(&question.resolved_by);
            carried_to.extend(&question.carried_to);
        }
        closed_by.sort_unstable();
        closed_by.dedup();
        carried_to.sort_unstable();
        carried_to.dedup();
        rows.push(Row {
            number: entry.number,
            text,
            items: &entry.questions,
            closed_by,
            carried_to,
        });
    }
    rows.sort_by_key(|row| std::cmp::Reverse(row.number));
    rows
}

/// The areas with something open, counted by entry, in the project's order.
fn area_chips(log: &Log) -> String {
    let mut chips = String::new();
    // Areas with something open, counted by entry, in the project's order.
    let mut per_area: Vec<(&str, usize)> = Vec::new();
    for area in &log.areas {
        let count = log
            .open_questions
            .iter()
            .filter(|q| {
                log.entry(q.entry)
                    .is_some_and(|e| e.areas.contains(&area.name))
            })
            .count();
        if count > 0 {
            per_area.push((&area.name, count));
        }
    }
    chips.push_str("<div class=\"filters\">\n<p class=\"rail-label\">Areas</p>\n");
    for (name, count) in per_area {
        let _ = writeln!(
            chips,
            "<button class=\"chip\" data-area=\"{name}\" aria-pressed=\"false\">\
             <span>{name}</span><span class=\"count\">{count}</span></button>",
            name = escape(name)
        );
    }
    chips.push_str("</div>\n");

    chips
}

/// One entry's card: the entry, then what it asked.
fn entry_row(row: &Row<'_>, links: &Links<'_>) -> String {
    let log = links.log;
    let mut body = String::new();
    let named = |numbers: &[u32]| numbered(log, numbers, false);
    let count = match row.items.len() {
        0 => question_count(row.text),
        n => n,
    };
    let closed_row = !row.closed_by.is_empty() || !row.carried_to.is_empty();
    let state = if closed_row { "closed" } else { "open" };
    let mut by = String::new();
    if !row.closed_by.is_empty() {
        let _ = write!(
            by,
            "<span class=\"q-closed\">closed by {}</span>",
            named(&row.closed_by)
        );
    }
    if !row.carried_to.is_empty() {
        let _ = write!(
            by,
            "<span class=\"q-closed\">carried to {}</span>",
            named(&row.carried_to)
        );
    }
    match log.entry(row.number) {
        Some(entry) => {
            let _ = writeln!(
                body,
                "<li data-n=\"{n}\" data-areas=\"{slugs}\" data-state=\"{state}\">\n\
                     <a class=\"q-no\" href=\"../{path}/\">{n}</a>\n<div class=\"q-main\">\n\
                     <a class=\"q-title\" href=\"../{path}/\">{title}</a>\n\
                     <p class=\"q-meta\"><time datetime=\"{date}\">{short}</time>\
                     <span>{areas}</span>{many}{by}</p>",
                n = entry.number,
                slugs = escape(&entry.areas.join(" ")),
                path = escape(&path_of(entry)),
                title = escape(&entry.title),
                date = entry.date,
                short = short_date(entry.date),
                areas = escape(&entry.areas.join(" \u{00b7} ")),
                many = if count > 1 {
                    format!("<span class=\"q-count\">{count} questions</span>")
                } else {
                    String::new()
                }
            );
        }
        None => {
            let _ = writeln!(
                body,
                "<li data-n=\"{n}\" data-state=\"{state}\">\n\
                     <span class=\"q-no\">{n}</span>\n<div class=\"q-main\">",
                n = row.number
            );
        }
    }
    let _ = writeln!(
        body,
        "<div class=\"question\">{}</div>\n</div>\n</li>",
        match row.items.is_empty() {
            true => question_html(row.text, links),
            false => numbered_questions_html(row.text, row.items, links),
        }
    );
    body
}

/// A reference page's card: the page, then its `## Unknown`.
fn page_row(log: &Log, doc: &DocPage, text: &str) -> String {
    let mut body = String::new();
    let label = log.docs_label.as_deref().unwrap_or("Reference");
    let links = Links {
        log,
        from_dir: doc_dir(doc),
        rel: "../",
        base: None,
    };
    let count = question_count(text);
    let _ = writeln!(
        body,
        "<li data-n=\"doc:{slug}\" data-state=\"open\">\n\
         <span class=\"q-no q-doc\" aria-hidden=\"true\"></span>\n<div class=\"q-main\">\n\
         <a class=\"q-title\" href=\"../docs/{slug}/\">{title}</a>\n\
         <p class=\"q-meta\"><span>{label}{section}</span>{status}{many}</p>\n\
         <div class=\"question\">{text}</div>\n</div>\n</li>",
        slug = escape(&doc.slug),
        title = escape(&doc.title),
        label = escape(label),
        section = if doc.section.is_empty() {
            String::new()
        } else {
            format!(
                " \u{00b7} {}",
                escape(&crate::reference::section_title(log, &doc.section))
            )
        },
        status = doc
            .status
            .map(|s| format!("<span class=\"status status--{0}\">{0}</span>", s.name()))
            .unwrap_or_default(),
        many = if count > 1 {
            format!("<span class=\"q-count\">{count} questions</span>")
        } else {
            String::new()
        },
        text = question_html(text, &links)
    );
    body
}

/// Whether a line of a trailer starts a top-level list item.
pub(crate) fn starts_item(line: &str) -> bool {
    line.starts_with("- ")
        || line.starts_with("* ")
        || line.starts_with("+ ")
        || line.split_once(['.', ')']).is_some_and(|(n, rest)| {
            !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) && rest.starts_with(' ')
        })
}

/// One question as HTML: markdown, linked, and without the paragraph a lone
/// line would be wrapped in, so a list of them stays a compact list.
pub(crate) fn question_inline(text: &str, links: &Links<'_>) -> String {
    let html = markdown_with_headings(text, links).1;
    let trimmed = html.trim_end();
    match trimmed
        .strip_prefix("<p>")
        .and_then(|rest| rest.strip_suffix("</p>"))
    {
        Some(inner) if !inner.contains("<p>") => inner.to_string(),
        _ => html,
    }
}

/// A list of questions by their numbers in the trailer - with gaps where
/// others were answered - folded past five when there are more than seven, in
/// the page as served.
pub(crate) fn numbered_questions_html(
    text: &str,
    items: &[cairns_core::log::Question],
    links: &Links<'_>,
) -> String {
    const SHOWN: usize = 5;
    let (preamble, _) = cairns_core::entry::question_items(text);
    let item = |question: &cairns_core::log::Question| {
        let class = if !question.resolved_by.is_empty() {
            " class=\"closed\""
        } else if !question.carried_to.is_empty() {
            " class=\"carried\""
        } else {
            ""
        };
        format!(
            "<li value=\"{}\"{class}>{}</li>",
            question.number,
            question_inline(&question.text, links)
        )
    };
    let mut out = String::new();
    if !preamble.is_empty() {
        out.push_str(&markdown_with_headings(&preamble, links).1);
    }
    let (shown, folded) = if items.len() > SHOWN + 2 {
        items.split_at(SHOWN)
    } else {
        (items, &[][..])
    };
    out.push_str("<ol class=\"numbered\">");
    for question in shown {
        out.push_str(&item(question));
    }
    out.push_str("</ol>");
    if !folded.is_empty() {
        let _ = write!(
            out,
            "<details class=\"more\"><summary>Show {} more</summary><ol class=\"numbered\">",
            folded.len()
        );
        for question in folded {
            out.push_str(&item(question));
        }
        out.push_str("</ol></details>");
    }
    out
}

/// How many questions a trailer asks: the items of a list, or one.
pub(crate) fn question_count(text: &str) -> usize {
    text.lines().filter(|line| starts_item(line)).count().max(1)
}

/// A trailer as HTML, a long list folded past its fifth item.
///
/// The fold is a `<details>` in the page as served, not something a script
/// does after it draws: folding on load moved every card below it, and a page
/// that jumps as it settles reads as broken. The list is split in the
/// markdown, so the folded half is the same list continued - an ordered one
/// keeps its numbering, because its sixth item says `6.`.
pub(crate) fn question_html(text: &str, links: &Links<'_>) -> String {
    const SHOWN: usize = 5;
    let starts: Vec<usize> = text
        .split_inclusive('\n')
        .scan(0, |at, line| {
            let here = *at;
            *at += line.len();
            Some((here, line))
        })
        .filter(|(_, line)| starts_item(line))
        .map(|(at, _)| at)
        .collect();
    if starts.len() <= SHOWN + 2 {
        return markdown_with_headings(text, links).1;
    }
    let cut = starts[SHOWN];
    let (shown, folded) = text.split_at(cut);
    format!(
        "{}<details class=\"more\"><summary>Show {} more</summary>{}</details>",
        markdown_with_headings(shown, links).1,
        starts.len() - SHOWN,
        markdown_with_headings(folded, links).1
    )
}
