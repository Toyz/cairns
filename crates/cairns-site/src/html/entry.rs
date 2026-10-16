//! One entry's page.

use super::*;

/// One entry, with its corrections, its open question, and its neighbours.
pub fn entry(log: &Log, at: usize) -> String {
    let this = &log.entries[at];
    let links = Links {
        log,
        from_dir: entry_dir(log),
        rel: "../",
        base: None,
    };
    let mut body = String::from("<article>\n");
    body.push_str(&dateline(this));
    body.push_str(&notice(log, this));
    // `files` has been parsed, validated and exported since the first version
    // and shown nowhere. It is the entry's link to the code it is about.
    body.push_str(&files_html(&this.files, &links));
    body.push_str(&mentions_html(log, this.number));
    let (headings, prose_html) = markdown_with_headings(prose(this), &links);
    body.push_str(&prose_html);
    body.push_str(&trailer(this, &links));
    body.push_str("</article>\n");
    body.push_str(&pager(log, at));

    let contents = contents_nav(&headings);
    let description = plain_md(this.summary.as_deref().unwrap_or(&this.title));
    let title = format!("{}. {}", this.number, this.title);
    shell(
        log,
        Page {
            title: &title,
            description: &description,
            url: &this.url,
            rel: "../",
            nav: Nav::Entries,
            aside: "",
            contents: &contents,
            body: &body,
        },
    )
}

/// Another entry, by its title, linked from an entry's page.
fn titled(log: &Log, number: u32) -> String {
    match log.entry(number) {
        Some(other) => format!(
            "<a href=\"../{}/\">{}</a>",
            escape(&path_of(other)),
            escape(&other.title)
        ),
        None => format!("entry {number}"),
    }
}

/// The number, the title, and the line under it: the date, how long it took,
/// its areas.
fn dateline(this: &LogEntry) -> String {
    let mut body = String::new();
    let _ = writeln!(body, "<p class=\"entry-number\">Entry {}</p>", this.number);
    let _ = writeln!(body, "<h1>{}</h1>", escape(&this.title));
    let _ = write!(
        body,
        "<p class=\"dateline\"><time datetime=\"{date}\">{date}</time>",
        date = this.date
    );
    // How long it took, beside when: quiet, because it is a fact about the
    // work and not what the entry is about.
    if let Some(minutes) = this.took_minutes {
        let _ = write!(
            body,
            "<span class=\"took\"{title}>{}</span>",
            cairns_core::entry::format_duration(minutes),
            title = this
                .started
                .as_deref()
                .map(|started| format!(" title=\"started {}\"", escape(started)))
                .unwrap_or_default()
        );
    }
    // On an entry page these were decoration. They are the same filter the
    // index has, so they link to it with the filter already applied.
    for area in &this.areas {
        let _ = write!(
            body,
            "<a class=\"chip\" href=\"../#area={area}\">{area}</a>",
            area = escape(area)
        );
    }
    body.push_str("</p>\n");

    body
}

/// What a reader should know before the prose: the notice is the whole reason
/// an append-only log is safe to read - a reader standing on an overturned
/// claim is told so first.
fn notice(log: &Log, this: &LogEntry) -> String {
    let mut body = String::new();
    let link = |number: u32| titled(log, number);
    if !this.superseded_by.is_empty()
        || !this.supersedes.is_empty()
        || !this.resolves.is_empty()
        || !this.resolves_questions.is_empty()
        || !this.carries.is_empty()
        || !this.carries_questions.is_empty()
        || !this.documented_by.is_empty()
        || !this.referenced_by.is_empty()
    {
        body.push_str("<div class=\"notice\">\n");
        if !this.superseded_by.is_empty() {
            let who: Vec<String> = this.superseded_by.iter().map(|n| link(*n)).collect();
            let _ = writeln!(
                body,
                "<p><strong>Revisited later.</strong> Something claimed here was \
                 corrected by {}.</p>",
                who.join(", ")
            );
        }
        if !this.supersedes.is_empty() {
            let who: Vec<String> = this.supersedes.iter().map(|n| link(*n)).collect();
            let _ = writeln!(body, "<p>This entry revisits {}.</p>", who.join(", "));
        }
        // Each page that rests on this entry, and the section of it when the
        // evidence was given section by section - one click to the place.
        if !this.documented_in.is_empty() {
            let places: Vec<String> = this
                .documented_in
                .iter()
                .map(|citation| {
                    let page = if citation.slug.is_empty() {
                        "../docs/".to_string()
                    } else {
                        format!("../docs/{}/", citation.slug)
                    };
                    match (&citation.section, &citation.anchor) {
                        (Some(section), Some(anchor)) => format!(
                            "<a href=\"{page}#{}\">{} \u{203a} {}</a>",
                            escape(anchor),
                            escape(&citation.title),
                            escape(section)
                        ),
                        _ => format!("<a href=\"{page}\">{}</a>", escape(&citation.title)),
                    }
                })
                .collect();
            let _ = writeln!(body, "<p>Documented in {}.</p>", places.join(", "));
        }
        // A single question is named the way it is written - `54.2` - and
        // links to the entry that asked it.
        let one = |question: &str| -> String {
            let (entry, item) = question.split_once('.').unwrap_or((question, ""));
            match entry.parse::<u32>().ok().and_then(|n| log.entry(n)) {
                Some(other) => format!(
                    "question {item} of <a href=\"../{}/\">{}</a>",
                    escape(&path_of(other)),
                    escape(&other.title)
                ),
                None => format!("question {}", escape(question)),
            }
        };
        let mut answers: Vec<String> = this.resolves.iter().map(|n| link(*n)).collect();
        answers.extend(this.resolves_questions.iter().map(|q| one(q)));
        if !answers.is_empty() {
            let _ = writeln!(
                body,
                "<p>This entry answers what was left open by {}.</p>",
                answers.join(", ")
            );
        }
        // The way back along `[[N]]` links, which were only ever written
        // forwards: what built on this entry later.
        if !this.referenced_by.is_empty() {
            let _ = writeln!(
                body,
                "<p>Linked from {}.</p>",
                this.referenced_by
                    .iter()
                    .map(|n| link(*n))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        let mut takes: Vec<String> = this.carries.iter().map(|n| link(*n)).collect();
        takes.extend(this.carries_questions.iter().map(|q| one(q)));
        if !takes.is_empty() {
            let _ = writeln!(
                body,
                "<p>This entry takes over the open questions of {}, unanswered.</p>",
                takes.join(", ")
            );
        }
        body.push_str("</div>\n");
    }

    body
}

/// The `**Still unknown:**` trailer, with its references linked like the
/// prose's. A question a later entry closed stays where it was asked, struck
/// through, naming what closed it; one carried into a later entry stays too,
/// not struck - it was not answered - pointing where it went. A list is
/// numbered, because those numbers are how a later entry names one of its
/// questions.
fn trailer(this: &LogEntry, links: &Links<'_>) -> String {
    let mut body = String::new();
    let entries_named = |numbers: &[u32], long: bool| numbered(links.log, numbers, long);
    if let Some(unknown) = &this.still_unknown {
        if !this.resolved_by.is_empty() {
            let (_, text) = markdown_with_headings(unknown, links);
            let _ = writeln!(
                body,
                "<div class=\"unknown unknown--answered\"><strong>Was unknown</strong>\
                 <div class=\"unknown-text\">{text}</div>\
                 <p class=\"answered\">Closed by {}.</p></div>",
                entries_named(&this.resolved_by, true)
            );
        } else if !this.carried_to.is_empty() {
            let (_, text) = markdown_with_headings(unknown, links);
            let _ = writeln!(
                body,
                "<div class=\"unknown unknown--carried\"><strong>Still unknown</strong>\
                 <div class=\"unknown-text\">{text}</div>\
                 <p class=\"answered\">Carried to {}, and open there.</p></div>",
                entries_named(&this.carried_to, true)
            );
        } else if this.questions.is_empty() {
            let (_, text) = markdown_with_headings(unknown, links);
            let _ = writeln!(
                body,
                "<div class=\"unknown\"><strong>Still unknown</strong>\
                 <div class=\"unknown-text\">{text}</div></div>"
            );
        } else {
            let (preamble, _) = cairns_core::entry::question_items(unknown);
            let mut list = String::new();
            for question in &this.questions {
                let text = question_inline(&question.text, links);
                let (class, note) = if !question.resolved_by.is_empty() {
                    (
                        " class=\"closed\"",
                        format!(
                            "<span class=\"q-note\">closed by {}</span>",
                            entries_named(&question.resolved_by, false)
                        ),
                    )
                } else if !question.carried_to.is_empty() {
                    (
                        " class=\"carried\"",
                        format!(
                            "<span class=\"q-note\">carried to {}</span>",
                            entries_named(&question.carried_to, false)
                        ),
                    )
                } else {
                    ("", String::new())
                };
                let _ = write!(
                    list,
                    "<li value=\"{}\"{class}><div class=\"q-text\">{text}</div>{note}</li>",
                    question.number
                );
            }
            let open = this.questions.iter().filter(|q| q.is_open()).count();
            let _ = writeln!(
                body,
                "<div class=\"unknown{}\"><strong>{}</strong>\
                 <div class=\"unknown-text\">{}<ol class=\"numbered\">{list}</ol></div></div>",
                if open == 0 { " unknown--answered" } else { "" },
                if open == 0 {
                    "Was unknown"
                } else {
                    "Still unknown"
                },
                if preamble.is_empty() {
                    String::new()
                } else {
                    markdown_with_headings(&preamble, links).1
                }
            );
        }
    }
    body
}

/// The entries either side.
fn pager(log: &Log, at: usize) -> String {
    let mut body = String::new();
    body.push_str("<nav class=\"pager\">\n");
    if at > 0 {
        let previous = &log.entries[at - 1];
        let _ = writeln!(
            body,
            "<a class=\"prev\" href=\"../{}/\"><span class=\"label\">Previous</span>{}</a>",
            escape(&path_of(previous)),
            escape(&previous.title)
        );
    }
    if at + 1 < log.entries.len() {
        let next = &log.entries[at + 1];
        let _ = writeln!(
            body,
            "<a class=\"next\" href=\"../{}/\"><span class=\"label\">Next</span>{}</a>",
            escape(&path_of(next)),
            escape(&next.title)
        );
    }
    body.push_str("</nav>\n");
    body.push_str(ARROW_KEYS);

    body
}
