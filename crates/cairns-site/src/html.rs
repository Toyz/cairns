//! The pages.
//!
//! Hand-built strings rather than a template engine: the whole site is five
//! page shapes, and a dependency that renders them would be larger than they
//! are. Every page is complete HTML with its own metadata, because the unit
//! that gets shared is one entry, not the site.

use cairns_core::log::{Log, LogEntry};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd, html};
use std::collections::BTreeMap;
use std::fmt::Write as _;

pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_FOOTNOTES
}

fn markdown(text: &str) -> String {
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(text, options()));
    // A wide table is the one thing allowed to scroll sideways, and only
    // inside its own box - never the page.
    out.replace("<table>", "<div class=\"table-scroll\"><table>")
        .replace("</table>", "</table></div>")
}

/// The prose alone: the entry's own `# N. Title` and its open-question trailer
/// are rendered by the page, in their own places, so they are cut from here
/// rather than appearing twice.
fn prose(entry: &LogEntry) -> &str {
    let text = entry.body.as_str();
    let text = match text.strip_prefix("# ") {
        Some(rest) => rest.split_once('\n').map(|(_, rest)| rest).unwrap_or(""),
        None => text,
    };
    let cut = text
        .match_indices(cairns_core::entry::STILL_UNKNOWN)
        .map(|(at, _)| at)
        .filter(|at| *at == 0 || text[..*at].ends_with('\n'))
        .last()
        .unwrap_or(text.len());
    text[..cut].trim()
}

/// Markdown reduced to its text, for anywhere markup would be shown literally
/// rather than rendered - a link preview, a feed summary, a list blurb.
pub fn plain_md(text: &str) -> String {
    use pulldown_cmark::Event;
    let mut out = String::new();
    for event in Parser::new_ext(text, options()) {
        match event {
            Event::Text(text) | Event::Code(text) => out.push_str(&text),
            Event::SoftBreak | Event::HardBreak => out.push(' '),
            _ => {}
        }
    }
    out.trim().to_string()
}

/// One heading in an entry: its depth, its words, and the anchor it gets.
pub struct Heading {
    pub level: u8,
    pub label: String,
    pub id: String,
}

/// Render markdown, giving every heading an anchor and collecting them.
///
/// The anchors are what a table of contents needs, and an entry that runs to
/// several sections is unreadable without one - `pulldown-cmark` emits no ids
/// of its own, so they are assigned here before the HTML is pushed.
fn markdown_with_headings(text: &str) -> (Vec<Heading>, String) {
    let mut events: Vec<Event> = Parser::new_ext(text, options()).collect();
    let mut headings = Vec::new();
    let mut taken: BTreeMap<String, usize> = BTreeMap::new();

    let mut at = 0;
    while at < events.len() {
        let Event::Start(Tag::Heading { level, .. }) = &events[at] else {
            at += 1;
            continue;
        };
        let depth = match level {
            HeadingLevel::H1 => 1,
            HeadingLevel::H2 => 2,
            HeadingLevel::H3 => 3,
            HeadingLevel::H4 => 4,
            HeadingLevel::H5 => 5,
            HeadingLevel::H6 => 6,
        };

        let mut label = String::new();
        let mut end = at + 1;
        while end < events.len() {
            match &events[end] {
                Event::End(TagEnd::Heading(_)) => break,
                Event::Text(text) | Event::Code(text) => label.push_str(text),
                _ => {}
            }
            end += 1;
        }

        // Two sections may be called the same thing; the anchors may not be.
        let base = cairns_core::entry::slugify(&label);
        let seen = taken.entry(base.clone()).or_insert(0);
        *seen += 1;
        let id = if *seen == 1 {
            base
        } else {
            format!("{base}-{seen}")
        };

        if let Event::Start(Tag::Heading { id: slot, .. }) = &mut events[at] {
            *slot = Some(id.clone().into());
        }
        headings.push(Heading {
            level: depth,
            label,
            id,
        });
        at = end + 1;
    }

    let mut out = String::new();
    html::push_html(&mut out, events.into_iter());
    let out = out
        .replace("<table>", "<div class=\"table-scroll\"><table>")
        .replace("</table>", "</table></div>");
    (headings, out)
}

/// Markdown whose relative links point into a repository rather than at the
/// site, which is where a README's `docs/spec/entry.md` actually lives.
fn markdown_linked(text: &str, repository: Option<&str>) -> String {
    let Some(repo) = repository.map(|repo| repo.trim_end_matches('/').to_string()) else {
        return markdown(text);
    };
    let into_repo = move |url: pulldown_cmark::CowStr<'_>| -> pulldown_cmark::CowStr<'static> {
        let target = url.as_ref();
        let absolute = target.is_empty()
            || target.starts_with('#')
            || target.contains("://")
            || target.starts_with("mailto:")
            || target.starts_with("//");
        if absolute {
            return target.to_string().into();
        }
        let clean = target.trim_start_matches("./");
        format!("{repo}/blob/HEAD/{clean}").into()
    };

    let events = Parser::new_ext(text, options()).map(|event| match event {
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Link {
            link_type,
            dest_url: into_repo(dest_url),
            title,
            id,
        }),
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Image {
            link_type,
            dest_url: into_repo(dest_url),
            title,
            id,
        }),
        other => other,
    });
    let mut out = String::new();
    html::push_html(&mut out, events);
    out.replace("<table>", "<div class=\"table-scroll\"><table>")
        .replace("</table>", "</table></div>")
}

/// An entry's prose as HTML, for the feed.
pub fn body_html(entry: &LogEntry) -> String {
    markdown(prose(entry))
}

/// An entry's body as plain text, for the search index.
pub fn plain(entry: &LogEntry) -> String {
    use pulldown_cmark::Event;
    let mut out = String::new();
    for event in Parser::new_ext(prose(entry), options()) {
        match event {
            Event::Text(text) | Event::Code(text) => {
                out.push_str(&text);
                out.push(' ');
            }
            Event::SoftBreak | Event::HardBreak => out.push(' '),
            _ => {}
        }
    }
    out
}

/// Which shape of page this is. The index wants the width, an entry wants a
/// reading column with its contents in the margin, and everything else wants
/// the reading column on its own.
#[derive(Clone, Copy, PartialEq)]
pub enum Layout {
    Index,
    Entry,
    Read,
}

impl Layout {
    fn class(self) -> &'static str {
        match self {
            Layout::Index => " wrap--index",
            Layout::Entry => " wrap--entry",
            Layout::Read => "",
        }
    }
}

/// The page shell. `rel` is the path back to the site root from this page, so
/// one renderer serves the root pages and the entry pages alike.
fn shell(
    log: &Log,
    title: &str,
    description: &str,
    url: &str,
    rel: &str,
    layout: Layout,
    body: &str,
) -> String {
    let base = log.project.base_url.trim_end_matches('/');
    let mut out = String::new();
    let _ = write!(
        out,
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<meta name="description" content="{description}">
<link rel="canonical" href="{url}">
<meta property="og:type" content="article">
<meta property="og:title" content="{title}">
<meta property="og:description" content="{description}">
<meta property="og:url" content="{url}">
<meta property="og:site_name" content="{site}">
<meta name="twitter:card" content="summary">
<link rel="alternate" type="application/atom+xml" title="{site}" href="{base}/feed.xml">
<link rel="stylesheet" href="{rel}style.css">
</head>
<body>
<div class="wrap{wide}">
<header class="masthead">
<h1><a href="{rel}">{site}</a></h1>
{tagline}<nav><a href="{rel}">Entries</a>{about}<a href="{rel}open/">Open questions</a><a href="{rel}feed.xml">Feed</a></nav>
</header>
{body}
<footer>
<p>An append-only worklog: a claim that turned out wrong is overturned by a
later entry, never by editing the original. Generated by
<a href="https://github.com/Toyz/cairns">cairns</a>.</p>
</footer>
</div>
</body>
</html>
"#,
        title = escape(title),
        description = escape(description),
        url = escape(url),
        site = escape(&log.project.name),
        tagline = if log.project.description.is_empty() {
            String::new()
        } else {
            format!("<p>{}</p>\n", escape(&log.project.description))
        },
        base = escape(base),
        wide = layout.class(),
        about = if log.readme.is_some() {
            format!("<a href=\"{rel}about/\">About</a>")
        } else {
            String::new()
        },
    );
    out
}

fn path_of(entry: &LogEntry) -> String {
    format!("{}-{}", entry.number, entry.slug)
}

/// The index: every entry, with the chips and the search box over it.
pub fn home(log: &Log) -> String {
    let base = log.project.base_url.trim_end_matches('/');
    let mut body = String::new();

    body.push_str("<div class=\"controls\">\n");
    let _ = writeln!(
        body,
        "<input type=\"search\" id=\"q\" placeholder=\"Search {} entries\" \
         autocomplete=\"off\" spellcheck=\"false\">",
        log.entries.len()
    );
    // Newest first is what someone checking back wants, and it is the order the
    // page ships in so it holds with scripting off. Oldest first is for reading
    // the thing through, which is the other half of why anyone opens a worklog.
    body.push_str(
        "<div class=\"sort\" role=\"group\" aria-label=\"Order\">\n\
         <button data-sort=\"new\" aria-pressed=\"true\">Newest</button>\n\
         <button data-sort=\"old\" aria-pressed=\"false\">Oldest</button>\n\
         </div>\n</div>\n<div class=\"chips\">\n",
    );
    for area in log.areas.iter().filter(|area| area.count > 0) {
        let _ = writeln!(
            body,
            "<button class=\"chip\" data-area=\"{name}\" aria-pressed=\"false\" \
             title=\"{about}\">{name}<span class=\"count\">{count}</span></button>",
            name = escape(&area.name),
            about = escape(&area.about),
            count = area.count
        );
    }
    body.push_str("</div>\n<p id=\"status\"></p>\n<ul class=\"entries\" id=\"entries\">\n");

    for entry in log.entries.iter().rev() {
        let _ = writeln!(
            body,
            "<li data-n=\"{n}\" data-areas=\"{areas}\">\n\
             <span class=\"no\">{n}</span>\n<div class=\"entry\">\n\
             <h2><a href=\"{path}/\">{title}</a></h2>",
            n = entry.number,
            areas = escape(&entry.areas.join(" ")),
            path = escape(&path_of(entry)),
            title = escape(&entry.title)
        );
        if let Some(summary) = &entry.summary {
            let _ = writeln!(
                body,
                "<p class=\"summary\">{}</p>",
                escape(&plain_md(summary))
            );
        }
        let _ = writeln!(
            body,
            "<p class=\"meta\"><time datetime=\"{date}\">{date}</time>\
             <span class=\"areas\">{areas}</span></p>\n</div>\n</li>",
            date = entry.date,
            areas = escape(&entry.areas.join(" \u{00b7} "))
        );
    }

    body.push_str("</ul>\n<script src=\"search.js\" defer></script>\n");

    let description = if log.project.description.is_empty() {
        format!("{} entries.", log.entries.len())
    } else {
        plain_md(&log.project.description)
    };
    shell(
        log,
        &log.project.name,
        &description,
        base,
        "./",
        Layout::Index,
        &body,
    )
}

/// One entry, with its corrections, its open question, and its neighbours.
pub fn entry(log: &Log, at: usize, by_number: &BTreeMap<u32, &LogEntry>) -> String {
    let this = &log.entries[at];
    let mut body = String::from("<article>\n");

    let _ = writeln!(body, "<h1>{}</h1>", escape(&this.title));
    let _ = write!(
        body,
        "<p class=\"dateline\"><time datetime=\"{date}\">{date}</time>",
        date = this.date
    );
    for area in &this.areas {
        let _ = write!(body, "<span class=\"chip\">{}</span>", escape(area));
    }
    body.push_str("</p>\n");

    // The notice is the whole reason an append-only log is safe to read: a
    // reader standing on an overturned claim is told so before the prose.
    let link = |number: u32| match by_number.get(&number) {
        Some(other) => format!(
            "<a href=\"../{}/\">{}</a>",
            escape(&path_of(other)),
            escape(&other.title)
        ),
        None => format!("entry {number}"),
    };
    if !this.superseded_by.is_empty() || !this.supersedes.is_empty() {
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
        body.push_str("</div>\n");
    }

    let (headings, prose_html) = markdown_with_headings(prose(this));
    body.push_str(&prose_html);

    if let Some(unknown) = &this.still_unknown {
        let _ = writeln!(
            body,
            "<div class=\"unknown\"><strong>Still unknown</strong>{}</div>",
            markdown(unknown)
        );
    }
    body.push_str("</article>\n<nav class=\"pager\">\n");
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

    // Sections and their subsections both, because an entry long enough to
    // want a contents list is long enough for its subsections to be where the
    // reader is actually trying to get to.
    let sections: Vec<&Heading> = headings
        .iter()
        .filter(|heading| heading.level == 2 || heading.level == 3)
        .collect();
    let contents = if sections.len() < 2 {
        String::new()
    } else {
        let mut toc = String::from(
            "<nav class=\"toc\" aria-label=\"Contents\">\n\
                                    <p class=\"toc-label\">Contents</p>\n<ol>\n",
        );
        for section in &sections {
            let _ = writeln!(
                toc,
                "<li class=\"toc-{}\"><a href=\"#{}\">{}</a></li>",
                section.level,
                escape(&section.id),
                escape(&section.label)
            );
        }
        toc.push_str("</ol>\n</nav>\n");
        toc
    };
    let body = format!("{contents}{body}");

    let description = plain_md(this.summary.as_deref().unwrap_or(&this.title));
    let title = format!("{}. {}", this.number, this.title);
    shell(
        log,
        &title,
        &description,
        &this.url,
        "../",
        Layout::Entry,
        &body,
    )
}

/// Everything the log has not closed out, in one place.
pub fn open_questions(log: &Log) -> String {
    let base = log.project.base_url.trim_end_matches('/');
    let by_number: BTreeMap<u32, &LogEntry> = log
        .entries
        .iter()
        .map(|entry| (entry.number, entry))
        .collect();

    let mut body = String::from("<article>\n<h1>Open questions</h1>\n");
    let _ = writeln!(
        body,
        "<p class=\"dateline\">{} of {} entries have something still unresolved.</p>",
        log.open_questions.len(),
        log.entries.len()
    );

    if log.open_questions.is_empty() {
        body.push_str("<p class=\"empty\">Nothing open.</p>\n");
    } else {
        body.push_str("<ul class=\"questions\">\n");
        for question in &log.open_questions {
            let path = by_number
                .get(&question.entry)
                .map(|entry| path_of(entry))
                .unwrap_or_default();
            let _ = writeln!(
                body,
                "<li><a class=\"no\" href=\"../{path}/\">{}</a><div>{}</div></li>",
                question.entry,
                markdown(&question.text)
            );
        }
        body.push_str("</ul>\n");
    }
    body.push_str("</article>\n");

    shell(
        log,
        &format!("Open questions - {}", log.project.name),
        "What this log has not closed out.",
        &format!("{base}/open/"),
        "../",
        Layout::Read,
        &body,
    )
}

/// The project's README, so a reader who arrives at a worklog can find out
/// what the project is - and how to install it.
pub fn about(log: &Log) -> String {
    let base = log.project.base_url.trim_end_matches('/');
    let readme = log.readme.as_deref().unwrap_or_default();

    // Its own `# Title` is the project name, which the masthead already says.
    let body_md = match readme.trim_start().strip_prefix("# ") {
        Some(rest) => rest.split_once('\n').map(|(_, rest)| rest).unwrap_or(""),
        None => readme,
    };

    let body = format!(
        "<article>\n<h1>About</h1>\n{}</article>\n",
        markdown_linked(body_md, log.project.repository.as_deref())
    );
    shell(
        log,
        &format!("About - {}", log.project.name),
        &if log.project.description.is_empty() {
            format!("What {} is.", log.project.name)
        } else {
            plain_md(&log.project.description)
        },
        &format!("{base}/about/"),
        "../",
        Layout::Read,
        &body,
    )
}
