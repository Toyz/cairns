//! The pages.
//!
//! Hand-built strings rather than a template engine: the whole site is five
//! page shapes, and a dependency that renders them would be larger than they
//! are. Every page is complete HTML with its own metadata, because the unit
//! that gets shared is one entry, not the site.

use cairns_core::log::{DocPage, Log, LogEntry};
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
    html::push_html(
        &mut out,
        crate::highlight::code_blocks(Parser::new_ext(text, options())).into_iter(),
    );
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
    // A closed trailer stays in the prose as written - "Still unknown:
    // nothing.", and whatever note follows it. Only an open one is lifted out
    // into its own box; cutting a closed one too threw its note away.
    if entry.still_unknown.is_none() {
        return text.trim();
    }
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
///
/// A reference, `[[358]]`, reads as `#358`: the brackets were printed as they
/// were written, in every index blurb and link preview, and a bare number in a
/// sentence does not say it is one.
pub fn plain_md(text: &str) -> String {
    use pulldown_cmark::Event;
    // A code reference reads as what it names - `entry.rs#question_items`.
    let text = cairns_core::entry::rewrite_code_references(text, &|code| {
        Some(format!(
            "`{}`",
            code.label.clone().unwrap_or_else(|| code_label(code))
        ))
    });
    let text = cairns_core::entry::rewrite_references(&text, &|reference| {
        Some(format!("cairns:{}", reference.number))
    });
    let mut out = String::new();
    // The number a reference link is labelled with, while inside one.
    let mut reference: Option<String> = None;
    for event in Parser::new_ext(&text, options()) {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                reference = dest_url.strip_prefix("cairns:").map(str::to_string);
            }
            Event::End(TagEnd::Link) => reference = None,
            Event::Text(text) if reference.as_deref() == Some(&*text) => {
                out.push('#');
                out.push_str(&text);
            }
            Event::Text(text) | Event::Code(text) => out.push_str(&text),
            Event::SoftBreak | Event::HardBreak => out.push(' '),
            _ => {}
        }
    }
    out.trim().to_string()
}

/// How a link written inside an entry is turned into one that resolves.
///
/// The log links to other entries by filename - `[4](0004-the-write-path.md)` -
/// which is right in the repo and on GitHub and dead on the site, where that
/// entry is a directory named after its number and slug. Anything else
/// relative is a file in the repository, so it points there.
pub struct Links<'a> {
    pub log: &'a Log,
    /// The directory of the document being rendered. A relative link is
    /// relative to *that*, which is the only way to know what `entry.md` means.
    pub from_dir: &'a str,
    /// The path back to the site root from the page being rendered.
    pub rel: &'a str,
    /// `Some(base)` renders site links absolute, for the feed, which has no
    /// page to resolve a relative one against.
    pub base: Option<&'a str>,
}

impl Links<'_> {
    fn site(&self, path: &str) -> String {
        match self.base {
            Some(base) => format!("{}/{path}", base.trim_end_matches('/')),
            None => format!("{}{path}", self.rel),
        }
    }

    fn resolve(&self, url: &str) -> String {
        let untouched = url.is_empty()
            || url.starts_with('#')
            || url.contains("://")
            || url.starts_with("mailto:")
            || url.starts_with("//");
        if untouched {
            return url.to_string();
        }

        // The scheme `[[12]]` is rewritten to. Resolved here so a reference and
        // a written-out link reach the same place by the same code.
        if let Some(number) = url.strip_prefix("cairns:")
            && let Ok(number) = number.trim().parse::<u32>()
            && let Some(entry) = self.log.entry(number)
        {
            return self.site(&format!("{}-{}/", entry.number, entry.slug));
        }

        let (path, fragment) = match url.split_once('#') {
            Some((path, fragment)) => (path, format!("#{fragment}")),
            None => (url, String::new()),
        };

        // Everything is decided on the repo-root path the link points at, so
        // `entry.md`, `../spec/entry.md` and `docs/spec/entry.md` all land in
        // the same place from wherever they were written.
        let target = from_repo_root(self.from_dir, path);

        // A file kept with an entry, in the folder named like it, is served
        // beside the entry's page.
        // The folder is the target's leading `worklog/0050-x`, found directly
        // rather than by trying every entry.
        if let Some((folder, name)) = target
            .match_indices('/')
            .map(|(at, _)| (&target[..at], &target[at + 1..]))
            .find(|(folder, _)| self.log.entry_with_folder(folder).is_some())
            && let Some(entry) = self.log.entry_with_folder(folder)
            && entry.attachments.iter().any(|a| a == name)
        {
            return format!(
                "{}{fragment}",
                self.site(&format!("{}-{}/{name}", entry.number, entry.slug))
            );
        }

        if let Some(entry) = self.log.entry_at(&target) {
            return format!(
                "{}{fragment}",
                self.site(&format!("{}-{}/", entry.number, entry.slug))
            );
        }
        if let Some(doc) = self.log.doc_at(&target) {
            return format!("{}{fragment}", self.site(&format!("docs/{}/", doc.slug)));
        }

        // The files the site has a page for in place of the file: the index is
        // the entry list, the entries directory is too, and the README is the
        // About page. A link to them is a link to that page, repository or not.
        let paths = &self.log.paths;
        let trimmed = target.trim_end_matches('/');
        if (!paths.index.is_empty() && trimmed == paths.index)
            || (!paths.entries.is_empty() && trimmed == paths.entries)
        {
            return format!("{}{fragment}", self.site(""));
        }
        if self.log.readme.is_some() && paths.readme.as_deref() == Some(trimmed) {
            return format!("{}{fragment}", self.site("about/"));
        }

        match self.log.project.repository.as_deref() {
            Some(repo) => format!(
                "{}/blob/HEAD/{target}{fragment}",
                repo.trim_end_matches('/')
            ),
            None => url.to_string(),
        }
    }
}

/// A path written inside an entry, as a path from the repository root.
fn from_repo_root(entry_dir: &str, path: &str) -> String {
    let mut parts: Vec<&str> = entry_dir
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            name => parts.push(name),
        }
    }
    parts.join("/")
}

/// Where the entries live, taken from the paths in the document itself rather
/// than from config the renderer does not read.
fn entry_dir(log: &Log) -> &str {
    log.entries
        .first()
        .and_then(|entry| entry.path.rsplit_once('/'))
        .map(|(dir, _)| dir)
        .unwrap_or("")
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
fn markdown_with_headings(text: &str, links: &Links<'_>) -> (Vec<Heading>, String) {
    // Code references first: each becomes a link to its lines, or for `![[`
    // the code itself, from what the binary resolved into the log.
    let text = cairns_core::entry::rewrite_code_references(text, &|code| {
        Some(code_reference_html(links.log, code))
    });
    // `[[12]]` becomes `[12](cairns:12)` before parsing, and `resolve` turns
    // that scheme into the entry's URL - so references and written-out links
    // take exactly the same path through the renderer.
    let text = cairns_core::entry::rewrite_references(&text, &|reference| {
        // An entry in another worklog links there, with its title when that
        // worklog could be read, and by number when it is known only by URL.
        if let Some(name) = &reference.workspace {
            return Some(match elsewhere_link(links.log, name, reference.number) {
                Some((url, title)) => format!("<{url}> \"{}\"", title.replace('"', "'")),
                // Known, but with nowhere to link: shown as text, not as the
                // brackets it was written in.
                None => format!(
                    "cairns-text: \"{}\"",
                    elsewhere_title(links.log, name, reference.number).replace('"', "'")
                ),
            });
        }
        links
            .log
            .entry(reference.number)
            // The title rides along as the link's title attribute, so a bare
            // number in prose still says what it points at on hover.
            .map(|entry| {
                format!(
                    "cairns:{} \"{}\"",
                    entry.number,
                    entry.title.replace('"', "'")
                )
            })
    });

    // A link to `cairns-text:` is a reference with nowhere to go: it becomes a
    // span with its title, and its closing tag follows it.
    let mut spans = Vec::new();
    let mut events: Vec<Event> = Parser::new_ext(&text, options())
        .map(|event| match event {
            Event::Start(Tag::Link {
                dest_url, title, ..
            }) if dest_url.starts_with("cairns-text:") => {
                spans.push(true);
                Event::Html(
                    format!(
                        "<span class=\"ref-elsewhere\" title=\"{}\">",
                        escape(&title)
                    )
                    .into(),
                )
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                spans.push(false);
                Event::Start(Tag::Link {
                    link_type,
                    dest_url: links.resolve(&dest_url).into(),
                    title,
                    id,
                })
            }
            Event::End(TagEnd::Link) if spans.pop() == Some(true) => Event::Html("</span>".into()),
            Event::Start(Tag::Image {
                link_type,
                dest_url,
                title,
                id,
            }) => Event::Start(Tag::Image {
                link_type,
                dest_url: links.resolve(&dest_url).into(),
                title,
                id,
            }),
            other => other,
        })
        .collect();
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
    html::push_html(&mut out, crate::highlight::code_blocks(events).into_iter());
    let out = out
        .replace("<table>", "<div class=\"table-scroll\"><table>")
        .replace("</table>", "</table></div>");
    (headings, out)
}

/// Markdown whose relative links point into a repository rather than at the
/// site, which is where a README's `docs/spec/entry.md` actually lives.
fn markdown_linked(text: &str, links: &Links<'_>) -> String {
    markdown_with_headings(text, links).1
}

#[allow(dead_code)]
fn unused_markdown_linked(text: &str, repository: Option<&str>) -> String {
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

/// An entry's prose as HTML, for the feed, with every link absolute - a feed
/// reader has no page to resolve a relative one against.
pub fn body_html(log: &Log, entry: &LogEntry) -> String {
    let links = Links {
        log,
        from_dir: entry_dir(log),
        rel: "",
        base: Some(log.project.base_url.trim_end_matches('/')),
    };
    markdown_with_headings(prose(entry), &links).1
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

/// The page shell: a sticky rail carrying the project's identity, its
/// navigation and whatever the page wants beside it, and a content column that
/// is the same width on every page.
///
/// `rel` is the path back to the site root, so one renderer serves the root
/// pages and the pages nested under `docs/` alike.
#[allow(clippy::too_many_arguments)]
fn shell(
    log: &Log,
    title: &str,
    description: &str,
    url: &str,
    rel: &str,
    current: &str,
    aside: &str,
    contents: &str,
    body: &str,
) -> String {
    let base = log.project.base_url.trim_end_matches('/');
    let here = |key: &str| {
        if key == current {
            " aria-current=\"page\""
        } else {
            ""
        }
    };

    let mut nav = format!("<a href=\"{rel}\"{}>Entries</a>", here("entries"));
    if !log.docs.is_empty() {
        let _ = write!(
            nav,
            "<a href=\"{rel}docs/\"{}>{}</a>",
            here("docs"),
            escape(log.docs_label.as_deref().unwrap_or("Reference"))
        );
    }
    let _ = write!(
        nav,
        "<a href=\"{rel}open/\"{}>Open questions</a>",
        here("open")
    );
    if log.readme.is_some() {
        let _ = write!(nav, "<a href=\"{rel}about/\"{}>About</a>", here("about"));
    }
    let _ = write!(nav, "<a href=\"{rel}feed.xml\">Feed</a>");

    // The project's own links. Kept in their own group rather than mixed into
    // the generated nav, because one set is pages this tool made and the other
    // is everywhere else.
    let mut mine = String::new();
    if !log.links.is_empty() {
        mine.push_str("<div class=\"rail-links\">\n");
        for link in &log.links {
            let glyph = link
                .icon
                .as_deref()
                .and_then(crate::icon::html)
                .unwrap_or_default();
            let _ = writeln!(
                mine,
                "<a href=\"{url}\">{glyph}<span>{label}</span></a>",
                url = escape(&link.url),
                label = escape(&link.label)
            );
        }
        mine.push_str("</div>\n");
    }

    format!(
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
<div class="shell">
<aside class="rail">
<div class="brand">
<a class="brand-name" href="{rel}">{site}</a>
{tagline}</div>
<nav class="rail-nav">{nav}</nav>
{mine}{aside}</aside>
<main>
{body}
<footer><p>Generated by {generator}.</p></footer>
</main>
<aside class="contents-rail">{contents}</aside>
</div>

</body>
</html>
"#,
        title = escape(title),
        description = escape(description),
        url = escape(url),
        site = escape(&log.project.name),
        base = escape(base),
        mine = mine,
        contents = contents,
        generator = generator_html(&log.generator),
        tagline = if log.project.description.is_empty() {
            String::new()
        } else {
            format!(
                "<p class=\"brand-line\">{}</p>\n",
                escape(&log.project.description)
            )
        },
    )
}

fn path_of(entry: &LogEntry) -> String {
    format!("{}-{}", entry.number, entry.slug)
}

/// The index: every entry, with the chips and the search box over it.
pub fn home(log: &Log) -> String {
    let base = log.project.base_url.trim_end_matches('/');

    // The filters live in the rail rather than above the list: they are
    // navigation, they are the same on every visit, and stacking them over the
    // entries pushed the first one off the fold.
    let mut chips = String::from("<div class=\"filters\">\n<p class=\"rail-label\">Areas</p>\n");
    // Where the time goes, beside how many entries each area has - once any
    // entry records how long it took. An entry in two areas counts in both:
    // the work was in both.
    for area in log.areas.iter().filter(|area| area.count > 0) {
        let time = total_took(
            log.entries
                .iter()
                .filter(|entry| entry.areas.contains(&area.name)),
        );
        let _ = writeln!(
            chips,
            "<button class=\"chip\" data-area=\"{name}\" aria-pressed=\"false\" \
             title=\"{about}{hover}\"><span>{name}</span><span class=\"count\">{shown}{count}</span></button>",
            name = escape(&area.name),
            about = escape(&area.about),
            hover = time
                .as_deref()
                .map(|t| format!(" - {t} recorded"))
                .unwrap_or_default(),
            shown = time
                .as_deref()
                .map(|t| format!("<span class=\"area-time\">{t}</span>"))
                .unwrap_or_default(),
            count = area.count
        );
    }
    chips.push_str("</div>\n");

    let mut body = String::from("<div class=\"toolbar\">\n<div class=\"search\">\n");
    body.push_str(
        "<svg viewBox=\"0 0 16 16\" aria-hidden=\"true\" focusable=\"false\">\
         <circle cx=\"7\" cy=\"7\" r=\"4.5\" fill=\"none\" stroke=\"currentColor\" \
         stroke-width=\"1.5\"/><path d=\"M10.5 10.5 L14 14\" stroke=\"currentColor\" \
         stroke-width=\"1.5\" stroke-linecap=\"round\"/></svg>\n",
    );
    let _ = writeln!(
        body,
        "<input type=\"search\" id=\"q\" placeholder=\"Search {} entries\" \
         autocomplete=\"off\" spellcheck=\"false\">\n</div>",
        log.entries.len()
    );
    // Newest first is what someone checking back wants, and it is the order the
    // page ships in so it holds with scripting off.
    //
    // Icons rather than words: four words beside the search field read as a
    // second toolbar. Each keeps its name for a screen reader and on hover.
    let icon = |paths: &str| {
        format!(
            "<svg viewBox=\"0 0 16 16\" aria-hidden=\"true\" focusable=\"false\" fill=\"none\" \
             stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"round\" \
             stroke-linejoin=\"round\">{paths}</svg>"
        )
    };
    let _ = writeln!(
        body,
        "<div class=\"switch\" role=\"group\" aria-label=\"Order\">\n\
         <button data-sort=\"new\" aria-pressed=\"true\" aria-label=\"Newest first\" \
         title=\"Newest first\">{newest}</button>\n\
         <button data-sort=\"old\" aria-pressed=\"false\" aria-label=\"Oldest first\" \
         title=\"Oldest first\">{oldest}</button>\n</div>\n\
         <div class=\"switch\" role=\"group\" aria-label=\"Density\">\n\
         <button data-density=\"detailed\" aria-pressed=\"true\" aria-label=\"Detailed\" \
         title=\"Detailed\">{detailed}</button>\n\
         <button data-density=\"compact\" aria-pressed=\"false\" aria-label=\"Compact\" \
         title=\"Compact\">{compact}</button>\n</div>\n</div>\n\
         <p id=\"status\"></p>\n<div class=\"days\" id=\"entries\">",
        // An arrow beside lines that shrink the way it points.
        newest =
            icon("<path d=\"M4 2.5v11M1.75 11.25 4 13.5l2.25-2.25M9 3.5h5.5M9 8h4M9 12.5h2.5\"/>"),
        oldest =
            icon("<path d=\"M4 13.5v-11M1.75 4.75 4 2.5l2.25 2.25M9 3.5h2.5M9 8h4M9 12.5h5.5\"/>"),
        // A title over a summary, twice; and four plain lines.
        detailed = icon("<path d=\"M2 3h12M2 6h8\"/><path d=\"M2 10.5h12M2 13.5h8\" />"),
        compact = icon("<path d=\"M2 3h12M2 6.33h12M2 9.67h12M2 13h12\"/>"),
    );

    // Under a heading per day rather than a date on every row. A log that runs
    // to hundreds of entries is written a dozen a day, and the same date
    // printed twelve times over is noise between titles. A day is a run of
    // entries in number order, so a date out of sequence shows up as its own
    // short group instead of being silently folded into another.
    let newest_first: Vec<&LogEntry> = log.entries.iter().rev().collect();
    for day in newest_first.chunk_by(|a, b| a.date == b.date) {
        let date = day[0].date;
        let _ = writeln!(
            body,
            "<section class=\"day\" id=\"d{date}\">\n<h2 class=\"day-label\"><time datetime=\"{date}\">{long}</time>\
             <span class=\"day-count\">{count}</span>{took}</h2>\n<ul class=\"entries\">",
            long = long_date(date),
            count = day.len(),
            took = total_took(day.iter().copied())
                .map(|t| format!("<span class=\"day-took\">{t}</span>"))
                .unwrap_or_default()
        );
        for entry in day {
            let _ = writeln!(
                body,
                "<li data-n=\"{n}\" data-areas=\"{slugs}\">\n\
                 <a class=\"row\" href=\"{path}/\">\n\
                 <span class=\"no\">{n}</span>\n<span class=\"row-main\">\n\
                 <span class=\"row-title\" title=\"{title}\">{title}</span>",
                n = entry.number,
                slugs = escape(&entry.areas.join(" ")),
                path = escape(&path_of(entry)),
                title = escape(&entry.title)
            );
            if let Some(summary) = &entry.summary {
                let _ = writeln!(
                    body,
                    "<span class=\"summary\">{}</span>",
                    escape(&plain_md(summary))
                );
            }
            let _ = writeln!(
                body,
                "<span class=\"meta\"><span class=\"areas\">{areas}</span>{took}</span>\n\
                 </span>\n</a>\n</li>",
                areas = escape(&entry.areas.join(" \u{00b7} ")),
                took = entry
                    .took_minutes
                    .map(|m| format!(
                        "<span class=\"took\">{}</span>",
                        cairns_core::entry::format_duration(m)
                    ))
                    .unwrap_or_default()
            );
        }
        body.push_str("</ul>\n</section>\n");
    }

    body.push_str("</div>\n<script src=\"search.js\" defer></script>\n");

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
        "entries",
        &chips,
        &glance(log),
        &body,
    )
}

/// The log at a glance, beside the list: how much there is, how much is still
/// open, and when it was written.
///
/// A list of three hundred entries says nothing about its own shape. Whether a
/// log is alive, whether it came in one burst or a steady drip, and how much of
/// it is unfinished are the first things a visitor wants to know, and none of
/// them could be read off the page.
fn glance(log: &Log) -> String {
    let (Some(first), Some(last)) = (log.entries.first(), log.entries.last()) else {
        return String::new();
    };
    let open = log
        .entries
        .iter()
        .filter(|entry| entry.still_unknown.is_some() && entry.resolved_by.is_empty())
        .count();

    let mut out = String::from(
        "<div class=\"glance\">\n<p class=\"rail-label\">This log</p>\n<dl class=\"glance-stats\">\n",
    );
    let _ = writeln!(
        out,
        "<div><dt>Entries</dt><dd>{}</dd></div>\n\
         <div><dt>Open</dt><dd><a href=\"open/\">{open}</a></dd></div>",
        log.entries.len()
    );
    if let Some(total) = total_took(log.entries.iter()) {
        let clocked = log
            .entries
            .iter()
            .filter(|e| e.took_minutes.is_some())
            .count();
        let _ = writeln!(
            out,
            "<div class=\"wide\" title=\"{clocked} of {} entries record how long they took\">\
             <dt>Time</dt><dd>{total}</dd></div>",
            log.entries.len()
        );
    }
    out.push_str("</dl>\n");

    // One bar per day across the whole span, empty days included - a gap is
    // part of the shape. A span too long for a bar a day is drawn by week, or
    // by month, so the strip stays readable rather than becoming a smear.
    let start = day_number(first.date.min(last.date));
    let end = day_number(first.date.max(last.date));
    let span = end - start + 1;
    let width = match span {
        ..=120 => 1,
        121..=730 => 7,
        _ => 30,
    };
    let buckets = ((span + width - 1) / width) as usize;
    let mut counts = vec![0usize; buckets];
    let mut firsts: Vec<Option<cairns_core::Date>> = vec![None; buckets];
    for entry in &log.entries {
        let at = ((day_number(entry.date) - start) / width) as usize;
        if let Some(count) = counts.get_mut(at) {
            *count += 1;
            let first_day = &mut firsts[at];
            if first_day.is_none_or(|day| entry.date < day) {
                *first_day = Some(entry.date);
            }
        }
    }
    let busiest = counts.iter().copied().max().unwrap_or(1).max(1);
    let unit = match width {
        1 => "day",
        7 => "week",
        _ => "month",
    };
    let _ = writeln!(
        out,
        "<div class=\"spark\" role=\"img\" aria-label=\"Entries per {unit}, busiest {busiest}\">"
    );
    for (count, day) in counts.iter().zip(&firsts) {
        match day {
            Some(day) => {
                let label = if width == 1 {
                    format!("{}: {count}", long_date(*day))
                } else {
                    format!("{unit} of {}: {count}", long_date(*day))
                };
                let _ = writeln!(
                    out,
                    "<a href=\"#d{day}\" style=\"--h:{pct}%\" title=\"{label}\"></a>",
                    pct = (count * 100).div_ceil(busiest).max(4),
                    label = escape(&label)
                );
            }
            None => out.push_str("<span></span>\n"),
        }
    }
    let _ = writeln!(
        out,
        "</div>\n<p class=\"spark-axis\"><span>{}</span><span>{}</span></p>",
        short_date(first.date.min(last.date)),
        short_date(first.date.max(last.date))
    );
    out.push_str(
        "<p class=\"keys\"><kbd>/</kbd> search <kbd>j</kbd><kbd>k</kbd> move \
         <kbd>\u{21b5}</kbd> open</p>\n</div>\n",
    );
    out
}

/// The time a run of entries took, summed, or `None` if none of them say.
/// Entries that do not say are not counted as zero - they are not counted.
fn total_took<'a>(entries: impl Iterator<Item = &'a LogEntry>) -> Option<String> {
    let minutes: Vec<u32> = entries.filter_map(|entry| entry.took_minutes).collect();
    (!minutes.is_empty()).then(|| cairns_core::entry::format_duration(minutes.iter().sum()))
}

/// Days since an arbitrary epoch, for spacing dates apart. Howard Hinnant's
/// `days_from_civil`, the inverse of what `Date` already carries.
fn day_number(date: cairns_core::Date) -> i64 {
    let year = i64::from(date.year) - i64::from(date.month <= 2);
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let month = i64::from(date.month);
    let doy =
        (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(date.day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe
}

/// `2 Oct 2026`.
fn short_date(date: cairns_core::Date) -> String {
    let long = long_date(date);
    let mut parts = long.splitn(3, ' ');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(day), Some(month), Some(year)) => {
            format!("{day} {} {year}", month.get(..3).unwrap_or(month))
        }
        _ => long,
    }
}

/// Evidence as pills: this log's entries as `#12`, other worklogs' as
/// `piney:361`, each linking to its entry with its title on hover.
fn evidence_chips(
    log: &Log,
    here: &[u32],
    elsewhere: &[cairns_core::entry::EntryRef],
    rel: &str,
    by_number: &BTreeMap<u32, &LogEntry>,
) -> String {
    let mut out = String::new();
    for number in here {
        match by_number.get(number) {
            Some(entry) => {
                let _ = write!(
                    out,
                    "<a class=\"chip\" href=\"{rel}{}-{}/\" title=\"{}\">#{number}</a>",
                    entry.number,
                    escape(&entry.slug),
                    escape(&entry.title)
                );
            }
            None => {
                let _ = write!(out, "<span class=\"chip\">#{number}</span>");
            }
        }
    }
    for other in elsewhere {
        match elsewhere_link(log, &other.workspace, other.number) {
            Some((url, title)) => {
                let _ = write!(
                    out,
                    "<a class=\"chip chip--elsewhere\" href=\"{}\" title=\"{}\">{}</a>",
                    escape(&url),
                    escape(&title),
                    escape(&other.to_string())
                );
            }
            None => {
                let _ = write!(
                    out,
                    "<span class=\"chip chip--elsewhere\">{}</span>",
                    escape(&other.to_string())
                );
            }
        }
    }
    out
}

/// What to call an entry in another worklog when there is nowhere to link it.
fn elsewhere_title(log: &Log, workspace: &str, number: u32) -> String {
    match log
        .workspaces
        .get(workspace)
        .and_then(|w| w.entries.get(&number))
    {
        Some(entry) => format!("{workspace} {number}: {}", entry.title),
        None => format!("entry {number} of {workspace}"),
    }
}

/// "cairns 0.10.1", linking to that release - so a reader, or whoever keeps
/// the site, can tell which version made a page without opening `log.json`.
/// A build from source says so rather than showing a version it is not.
fn generator_html(generator: &str) -> String {
    let version = generator.strip_prefix("cairns ").unwrap_or(generator);
    if version.is_empty() || version.contains("dev") {
        return "<a href=\"https://github.com/Toyz/cairns\">cairns</a> (a development build)"
            .to_string();
    }
    format!(
        "<a href=\"https://github.com/Toyz/cairns/releases/tag/v{v}\">cairns {v}</a>",
        v = escape(version)
    )
}

/// Where an entry in another worklog is, and what to call it on hover: its
/// page when the worklog was read, its number's short URL when it is known
/// only by where it is published. `None` when it is known not at all.
fn elsewhere_link(log: &Log, workspace: &str, number: u32) -> Option<(String, String)> {
    let found = log.workspaces.get(workspace)?;
    if found.url.is_empty() {
        return None;
    }
    Some(match found.entries.get(&number) {
        Some(entry) => (
            format!("{}/{number}-{}/", found.url, entry.slug),
            format!("{workspace} {number}: {}", entry.title),
        ),
        None => (
            format!("{}/{number}/", found.url),
            format!("entry {number} of {workspace}"),
        ),
    })
}

/// How a code reference reads when it gives no words of its own: the file's
/// name and what in it - `entry.rs#question_items`, `entry.rs:120-158` - or
/// the whole path for a whole file. The full path is on hover.
fn code_label(code: &cairns_core::entry::CodeRef) -> String {
    let name = code.path.rsplit('/').next().unwrap_or(&code.path);
    if let Some(symbol) = &code.symbol {
        format!("{name}#{symbol}")
    } else if let Some((start, end)) = code.lines {
        if start == end {
            format!("{name}:{start}")
        } else {
            format!("{name}:{start}-{end}")
        }
    } else {
        code.path.clone()
    }
}

/// The repository URL for resolved code, at its lines, when the project names
/// a repository. GitHub's form - `blob/<rev>/<path>#L12-L40` - which GitLab
/// and Gitea also read.
fn code_url(log: &Log, code: &cairns_core::log::ResolvedCode) -> Option<String> {
    let repo = log.project.repository.as_deref()?.trim_end_matches('/');
    let mut url = format!(
        "{repo}/blob/{}/{}",
        code.rev.as_deref().unwrap_or("HEAD"),
        code.path
    );
    if let Some((start, end)) = code.lines {
        url.push_str(&if start == end {
            format!("#L{start}")
        } else {
            format!("#L{start}-L{end}")
        });
    }
    Some(url)
}

/// A code reference as HTML: a link to its lines, or - `![[...]]` - the code
/// itself, highlighted, under a caption linking to where it lives. One that
/// could not be found says why, rather than linking somewhere that is not.
fn code_reference_html(log: &Log, code: &cairns_core::entry::CodeRef) -> String {
    let label = code.label.clone().unwrap_or_else(|| code_label(code));
    let Some(resolved) = log.code.get(&code.key()) else {
        // Not resolved at all - a log.json from before code references, or a
        // renderer handed only part of a log. Shown as written.
        return format!("<code class=\"code-ref\">{}</code>", escape(&label));
    };
    if let Some(why) = &resolved.missing {
        return format!(
            "<code class=\"code-ref code-ref--missing\" title=\"{} - {}\">{}</code>",
            escape(&code.key()),
            escape(why),
            escape(&label)
        );
    }
    let url = code_url(log, resolved);
    let anchor = |inner: String| match &url {
        Some(url) => format!(
            "<a class=\"code-ref\" href=\"{}\" title=\"{}\">{inner}</a>",
            escape(url),
            escape(&code.key())
        ),
        None => inner,
    };
    if !code.embed {
        return anchor(format!("<code>{}</code>", escape(&label)));
    }
    let Some(text) = &resolved.text else {
        return anchor(format!("<code>{}</code>", escape(&label)));
    };
    let place = match resolved.lines {
        Some((start, end)) if start == end => format!("line {start}"),
        Some((start, end)) => format!("lines {start}\u{2013}{end}"),
        None => String::new(),
    };
    // An HTML block, which markdown ends at a blank line: blank lines in the
    // code are given something to hold them open.
    let highlighted = crate::highlight::file(text, &resolved.path)
        .lines()
        .map(|line| {
            if line.trim().is_empty() {
                "<span></span>"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "\n<figure class=\"code-embed\"><figcaption>{}<span>{}</span></figcaption>\n{highlighted}\n</figure>\n",
        anchor(format!("<code>{}</code>", escape(&code.path))),
        escape(&place)
    )
}

/// More files than this and the list folds away behind its count.
const FILES_SHOWN: usize = 5;

/// An entry's files, grouped under their directories.
///
/// Printed flat, fifteen paths were a wall between the title and the first
/// sentence, and most of it was the same `crates/x/src/` over and over. The
/// directory is said once and the names follow it, in the order written. Past
/// [`FILES_SHOWN`] the list is still the entry's link to the code, but not the
/// thing a reader came for, so it starts closed.
fn files_html(files: &[String], links: &Links<'_>) -> String {
    if files.is_empty() {
        return String::new();
    }
    let mut groups: Vec<(&str, Vec<(&str, &String)>)> = Vec::new();
    for file in files {
        // A directory is written with a trailing slash; it is named by its
        // last component like any file, not grouped under itself.
        let (dir, name) = match file.trim_end_matches('/').rsplit_once('/') {
            Some((dir, _)) => (&file[..=dir.len()], &file[dir.len() + 1..]),
            None => ("", file.as_str()),
        };
        match groups.iter_mut().find(|(seen, _)| *seen == dir) {
            Some((_, names)) => names.push((name, file)),
            None => groups.push((dir, vec![(name, file)])),
        }
    }

    let mut list = String::new();
    for (dir, names) in &groups {
        list.push_str("<span class=\"files-group\">");
        if !dir.is_empty() {
            let _ = write!(list, "<span class=\"files-dir\">{}</span>", escape(dir));
        }
        for (name, file) in names {
            // A file named with its lines or a definition links to them.
            let href = cairns_core::entry::CodeRef::parse(file)
                .filter(|code| code.lines.is_some() || code.symbol.is_some())
                .and_then(|code| links.log.code.get(&code.key()))
                .filter(|resolved| resolved.missing.is_none())
                .and_then(|resolved| code_url(links.log, resolved))
                .unwrap_or_else(|| links.resolve(&format!("../{file}")));
            let _ = write!(
                list,
                "<a href=\"{}\" title=\"{}\"><code>{}</code></a>",
                escape(&href),
                escape(file),
                escape(name)
            );
        }
        list.push_str("</span>");
    }

    if files.len() <= FILES_SHOWN {
        return format!("<p class=\"files\"><span class=\"files-label\">Files</span>{list}</p>\n");
    }
    let dirs = match groups.len() {
        1 => String::new(),
        n => format!(" in {n} directories"),
    };
    format!(
        "<details class=\"files\"><summary><span class=\"files-label\">Files</span>\
         <span class=\"files-count\">{count}{dirs}</span></summary>\
         <div class=\"files-list\">{list}</div></details>\n",
        count = files.len()
    )
}

/// `2 October 2026`. Unambiguous in every locale, which `10/02` is not.
fn long_date(date: cairns_core::Date) -> String {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    match MONTHS.get(usize::from(date.month).wrapping_sub(1)) {
        Some(month) => format!("{} {month} {}", date.day, date.year),
        None => date.to_string(),
    }
}

/// Where the repository's own files mention this entry - "worklog 50" in a
/// comment - shown the way its files are, each linking to the line.
fn mentions_html(log: &Log, number: u32) -> String {
    let Some(found) = log.mentions.get(&number).filter(|found| !found.is_empty()) else {
        return String::new();
    };
    let mut list = String::new();
    for mention in found {
        let resolved = cairns_core::log::ResolvedCode {
            path: mention.path.clone(),
            lines: Some((mention.line, mention.line)),
            ..Default::default()
        };
        let label = format!("{}:{}", mention.path, mention.line);
        let _ = write!(
            list,
            "<span class=\"files-group\">{}</span>",
            match code_url(log, &resolved) {
                Some(url) => format!(
                    "<a href=\"{}\" title=\"{}\"><code>{}</code></a>",
                    escape(&url),
                    escape(&mention.text),
                    escape(&label)
                ),
                None => format!(
                    "<code title=\"{}\">{}</code>",
                    escape(&mention.text),
                    escape(&label)
                ),
            }
        );
    }
    let count = found.len();
    if count <= FILES_SHOWN {
        return format!(
            "<p class=\"files mentions\"><span class=\"files-label\">Mentioned in</span>{list}</p>\n"
        );
    }
    format!(
        "<details class=\"files mentions\"><summary><span class=\"files-label\">Mentioned in</span>\
         <span class=\"files-count\">{count} places</span></summary>\
         <div class=\"files-list\">{list}</div></details>\n"
    )
}

/// What a reference page covers, shown the way an entry shows its files: a
/// line of items when there are a few, folded behind a count when there are
/// many, one group to a line. piney_apples' pages cover up to a hundred
/// addresses, and those wrote a table of their own to say so; this is that
/// table's column, given a place on the page it describes.
fn covers_html(groups: &[Vec<String>]) -> String {
    let count: usize = groups.iter().map(Vec::len).sum();
    if count == 0 {
        return String::new();
    }
    let mut list = String::new();
    for group in groups {
        list.push_str("<span class=\"files-group\">");
        for item in group {
            let _ = write!(list, "<code>{}</code>", escape(item));
        }
        list.push_str("</span>");
    }
    if count <= FILES_SHOWN {
        return format!(
            "<p class=\"files covers\"><span class=\"files-label\">Covers</span>{list}</p>\n"
        );
    }
    let groups_note = match groups.len() {
        1 => String::new(),
        n => format!(" in {n} groups"),
    };
    format!(
        "<details class=\"files covers\"><summary><span class=\"files-label\">Covers</span>\
         <span class=\"files-count\">{count}{groups_note}</span></summary>\
         <div class=\"files-list\">{list}</div></details>\n"
    )
}

/// One entry, with its corrections, its open question, and its neighbours.
pub fn entry(log: &Log, at: usize, by_number: &BTreeMap<u32, &LogEntry>) -> String {
    let this = &log.entries[at];
    let mut body = String::from("<article>\n");

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
            match entry.parse::<u32>().ok().and_then(|n| by_number.get(&n)) {
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

    let links = Links {
        log,
        from_dir: entry_dir(log),
        rel: "../",
        base: None,
    };

    // `files` has been parsed, validated and exported since the first version
    // and shown nowhere. It is the entry's link to the code it is about.
    body.push_str(&files_html(&this.files, &links));
    body.push_str(&mentions_html(log, this.number));

    let (headings, prose_html) = markdown_with_headings(prose(this), &links);
    body.push_str(&prose_html);

    // The trailer, with its references linked like the prose's. A question a
    // later entry closed stays where it was asked, struck through, naming what
    // closed it; one carried into a later entry stays too, not struck - it was
    // not answered - pointing where it went. A list is numbered, because those
    // numbers are how a later entry names one of its questions.
    if let Some(unknown) = &this.still_unknown {
        let entries_named = |numbers: &[u32], long: bool| -> String {
            numbers
                .iter()
                .map(|number| match by_number.get(number) {
                    Some(other) if long => format!(
                        "<a href=\"../{}/\">No. {number} \u{2014} {}</a>",
                        escape(&path_of(other)),
                        escape(&other.title)
                    ),
                    Some(other) => format!(
                        "<a href=\"../{}/\" title=\"{}\">#{number}</a>",
                        escape(&path_of(other)),
                        escape(&other.title)
                    ),
                    None => format!("#{number}"),
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        if !this.resolved_by.is_empty() {
            let (_, text) = markdown_with_headings(unknown, &links);
            let _ = writeln!(
                body,
                "<div class=\"unknown unknown--answered\"><strong>Was unknown</strong>\
                 <div class=\"unknown-text\">{text}</div>\
                 <p class=\"answered\">Closed by {}.</p></div>",
                entries_named(&this.resolved_by, true)
            );
        } else if !this.carried_to.is_empty() {
            let (_, text) = markdown_with_headings(unknown, &links);
            let _ = writeln!(
                body,
                "<div class=\"unknown unknown--carried\"><strong>Still unknown</strong>\
                 <div class=\"unknown-text\">{text}</div>\
                 <p class=\"answered\">Carried to {}, and open there.</p></div>",
                entries_named(&this.carried_to, true)
            );
        } else if this.questions.is_empty() {
            let (_, text) = markdown_with_headings(unknown, &links);
            let _ = writeln!(
                body,
                "<div class=\"unknown\"><strong>Still unknown</strong>\
                 <div class=\"unknown-text\">{text}</div></div>"
            );
        } else {
            let (preamble, _) = cairns_core::entry::question_items(unknown);
            let mut list = String::new();
            for question in &this.questions {
                let text = question_inline(&question.text, &links);
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
                    markdown_with_headings(&preamble, &links).1
                }
            );
        }
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
    body.push_str(ARROW_KEYS);

    let contents = contents_nav(&headings);
    let description = plain_md(this.summary.as_deref().unwrap_or(&this.title));
    let title = format!("{}. {}", this.number, this.title);
    shell(
        log,
        &title,
        &description,
        &this.url,
        "../",
        "entries",
        "",
        &contents,
        &body,
    )
}

/// The sections of a page, for the contents rail.
///
/// Sections and their subsections both, because a page long enough to want a
/// contents list is long enough for its subsections to be where the reader is
/// actually trying to get to. Folded on a screen too narrow for a third column,
/// where it sits above the prose: fifteen sections there were a screen of
/// links before the first sentence.
fn contents_nav(headings: &[Heading]) -> String {
    let sections: Vec<&Heading> = headings
        .iter()
        .filter(|heading| heading.level == 2 || heading.level == 3)
        .collect();
    if sections.len() < 2 {
        return String::new();
    }
    let mut toc = format!(
        "<details class=\"toc\">\n\
         <summary class=\"toc-label\">Contents <span class=\"count\">{}</span></summary>\n\
         <nav aria-label=\"Contents\"><ol>\n",
        sections.iter().filter(|section| section.level == 2).count()
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
    toc.push_str("</ol></nav>\n</details>\n");
    toc.push_str(&open_when_wider("62rem"));
    toc
}

/// The reference tree's open state: open beside the page, folded on a phone,
/// and the folders a reader opened kept open across loads. Kept in the
/// browser, per site, because it is one reader's place in the tree and nothing
/// anyone else needs.
const TREE_STATE: &str = "<script>(function(tree){\
if(!matchMedia(\"(max-width: 62rem)\").matches)tree.open=true;\
var brand=document.querySelector(\".brand-name\");\
var key=\"cairns-tree:\"+(brand?brand.href:location.host);\
var saved=[];try{saved=JSON.parse(localStorage.getItem(key)||\"[]\")||[];}catch(e){}\
var folders=[].slice.call(tree.querySelectorAll(\"details[data-folder]\"));\
folders.forEach(function(d){if(saved.indexOf(d.dataset.folder)!==-1)d.open=true;\
d.addEventListener(\"toggle\",function(){\
var open=folders.filter(function(f){return f.open;}).map(function(f){return f.dataset.folder;});\
try{localStorage.setItem(key,JSON.stringify(open));}catch(e){}});});\
})(document.currentScript.previousElementSibling);</script>\n";

/// Opens the `<details>` just before it when the screen is wider than `width`.
///
/// Served closed and opened here, during parsing, so the browser lays it out
/// once at the right size. Served open and closed by a script at the end of
/// the page, it drew open and then collapsed - the page visibly jumped.
fn open_when_wider(width: &str) -> String {
    format!(
        "<script>if(!matchMedia(\"(max-width: {width})\").matches)\
         document.currentScript.previousElementSibling.open=true;</script>\n"
    )
}

/// The arrow keys walk whatever the pager links, on entries and reference pages
/// alike. Inline because it is four lines and a request for it would cost more
/// than it does.
const ARROW_KEYS: &str = "<script>document.addEventListener(\"keydown\",function(e){\
     if(e.metaKey||e.ctrlKey||e.altKey||/^(INPUT|TEXTAREA|SELECT)$/.test(e.target.tagName))return;\
     var a=document.querySelector(e.key===\"ArrowLeft\"?\".pager .prev\":e.key===\"ArrowRight\"?\".pager .next\":null);\
     if(a)location.href=a.href;});</script>\n";

/// Everything the log has not closed out, in one place.
///
/// One card per entry, newest first, the entry named above its questions
/// rather than under them: a question is a fragment out of context, and the
/// reader needs the context first. A trailer that is a list is one card with
/// its items, folded past five, because a triage entry carrying thirty
/// questions forward otherwise buries everything after it.
pub fn open_questions(log: &Log) -> String {
    let base = log.project.base_url.trim_end_matches('/');
    let by_number: BTreeMap<u32, &LogEntry> = log
        .entries
        .iter()
        .map(|entry| (entry.number, entry))
        .collect();
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
    let closed: Vec<&LogEntry> = log
        .entries
        .iter()
        .filter(|entry| {
            entry.still_unknown.is_some()
                && (!entry.resolved_by.is_empty()
                    || !entry.carried_to.is_empty()
                    || (!entry.questions.is_empty()
                        && entry.questions.iter().all(|q| !q.is_open())))
        })
        .collect();

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
        body.push_str(
            "<div class=\"toolbar\">\n<div class=\"search\">\n\
             <svg viewBox=\"0 0 16 16\" aria-hidden=\"true\" focusable=\"false\">\
             <circle cx=\"7\" cy=\"7\" r=\"4.5\" fill=\"none\" stroke=\"currentColor\" \
             stroke-width=\"1.5\"/><path d=\"M10.5 10.5 L14 14\" stroke=\"currentColor\" \
             stroke-width=\"1.5\" stroke-linecap=\"round\"/></svg>\n\
             <input type=\"search\" id=\"q\" placeholder=\"Search the questions\" \
             autocomplete=\"off\" spellcheck=\"false\">\n</div>\n",
        );
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
        // Areas with something open, counted by entry, in the project's order.
        let mut per_area: Vec<(&str, usize)> = Vec::new();
        for area in &log.areas {
            let count = log
                .open_questions
                .iter()
                .filter(|q| {
                    by_number
                        .get(&q.entry)
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

        body.push_str("<ul class=\"questions\" id=\"entries\" data-local>\n");
        // Every entry that asked something, open or closed, newest first; the
        // switch decides which are shown.
        struct Row<'a> {
            number: u32,
            text: &'a str,
            items: &'a [cairns_core::log::Question],
            closed_by: Vec<u32>,
            carried_to: Vec<u32>,
        }
        let mut rows: Vec<Row> = log
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
        for entry in &closed {
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
        let named = |numbers: &[u32]| -> String {
            numbers
                .iter()
                .map(|n| match by_number.get(n) {
                    Some(other) => format!(
                        "<a href=\"../{}/\" title=\"{}\">#{n}</a>",
                        escape(&path_of(other)),
                        escape(&other.title)
                    ),
                    None => format!("#{n}"),
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        for row in rows {
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
            match by_number.get(&row.number) {
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
                    true => question_html(row.text, &links),
                    false => numbered_questions_html(row.text, row.items, &links),
                }
            );
        }
        // Then the reference's, after the log's, each under its page.
        let label = log.docs_label.as_deref().unwrap_or("Reference");
        for (doc, text) in &doc_unknowns {
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
                    format!(" \u{00b7} {}", escape(&folder_name(&doc.section)))
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
        }
        body.push_str("</ul>\n");
    }

    body.push_str("</article>\n<script src=\"../search.js\" defer></script>\n");

    shell(
        log,
        &format!("Open questions - {}", log.project.name),
        "What this log has not closed out.",
        &format!("{base}/open/"),
        "../",
        "open",
        &chips,
        "",
        &body,
    )
}

/// Whether a line of a trailer starts a top-level list item.
fn starts_item(line: &str) -> bool {
    line.starts_with("- ")
        || line.starts_with("* ")
        || line.starts_with("+ ")
        || line.split_once(['.', ')']).is_some_and(|(n, rest)| {
            !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) && rest.starts_with(' ')
        })
}

/// One question as HTML: markdown, linked, and without the paragraph a lone
/// line would be wrapped in, so a list of them stays a compact list.
fn question_inline(text: &str, links: &Links<'_>) -> String {
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
fn numbered_questions_html(
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
fn question_count(text: &str) -> usize {
    text.lines().filter(|line| starts_item(line)).count().max(1)
}

/// A trailer as HTML, a long list folded past its fifth item.
///
/// The fold is a `<details>` in the page as served, not something a script
/// does after it draws: folding on load moved every card below it, and a page
/// that jumps as it settles reads as broken. The list is split in the
/// markdown, so the folded half is the same list continued - an ordered one
/// keeps its numbering, because its sixth item says `6.`.
fn question_html(text: &str, links: &Links<'_>) -> String {
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
        markdown_linked(
            body_md,
            &Links {
                log,
                from_dir: "",
                rel: "../",
                base: None
            }
        )
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
        "about",
        "",
        "",
        &body,
    )
}

/// The reference tree, for the rail: folders, then the pages inside them.
///
/// Navigation for a docs tree belongs beside the page, not only on an index
/// somebody has to go back to. `here` is the slug of the page being rendered,
/// so it can mark itself.
pub fn docs_tree(log: &Log, rel: &str, here: &str) -> String {
    let label = log.docs_label.as_deref().unwrap_or("Reference");
    // On a phone the rail sits above the page, and sixty titles there were a
    // screen and a half before the first sentence. It folds there; beside the
    // page it is open.
    let pages = log.docs.iter().filter(|doc| !doc.is_index).count();
    let mut out = format!(
        "<details class=\"tree\">\n\
         <summary class=\"rail-label\">{} <span class=\"count\">{pages}</span></summary>\n",
        escape(label)
    );
    out.push_str(&branch(log, rel, here, ""));
    out.push_str("</details>\n");
    // Opened on a screen wide enough for the rail, and the folders the reader
    // opened before opened again - while the page is still parsing, so it is
    // laid out once. Without this every reload, and every live rebuild under
    // `serve`, folded back everything but the current page's folder.
    out.push_str(TREE_STATE);
    out
}

fn branch(log: &Log, rel: &str, here: &str, within: &str) -> String {
    let pages: Vec<&DocPage> = log
        .docs
        .iter()
        .filter(|doc| doc.section == within && !doc.is_index)
        .collect();

    // Folders come from the directories themselves, not only from directories
    // that happen to contain a README. Hellbender's docs tree has four
    // subdirectories and no index page in any of them, and deriving folders
    // from index pages alone left its rail empty.
    let mut folders: Vec<String> = log
        .docs
        .iter()
        .filter_map(|doc| child_section(within, &doc.section))
        .collect();
    folders.sort();
    folders.dedup();

    if pages.is_empty() && folders.is_empty() {
        return String::new();
    }

    let mut out = String::from("<ul>\n");
    let link = |doc: &DocPage| {
        format!(
            "<a href=\"{rel}docs/{slug}/\"{current}>{title}</a>",
            slug = escape(&doc.slug),
            current = if doc.slug == here {
                " aria-current=\"page\""
            } else {
                ""
            },
            title = escape(&doc.title)
        )
    };
    for doc in pages {
        let _ = writeln!(out, "<li>{}</li>", link(doc));
    }
    for folder in folders {
        // A folder with its own page is a link to it; one without is a label.
        let named = log
            .docs
            .iter()
            .find(|doc| doc.is_index && doc.slug == folder);
        let heading = match named {
            Some(doc) => link(doc),
            None => format!("<span>{}</span>", escape(&folder_name(&folder))),
        };
        // Every folder folds, and only the one holding this page starts open.
        // Sixty pages listed in full is not navigation; four folders with
        // their counts is.
        let inside = log
            .docs
            .iter()
            .filter(|doc| {
                !doc.is_index
                    && (doc.section == folder || doc.section.starts_with(&format!("{folder}/")))
            })
            .count();
        // A small reference is shown whole; a large one only down to the page
        // being read.
        let small = log.docs.iter().filter(|doc| !doc.is_index).count() <= 12;
        let open = small || here == folder || here.starts_with(&format!("{folder}/"));
        let _ = writeln!(
            out,
            "<li class=\"folder\"><details data-folder=\"{}\"{}><summary>{heading}<span class=\"count\">{inside}</span></summary>",
            escape(&folder),
            if open { " open" } else { "" }
        );
        out.push_str(&branch(log, rel, here, &folder));
        out.push_str("</details></li>\n");
    }
    out.push_str("</ul>\n");
    out
}

/// A directory's name as a heading: `formats` reads as `Formats`. Only the
/// first letter is touched, so `iop` and `VU1` keep their own case after it.
fn folder_name(folder: &str) -> String {
    let name = folder.rsplit('/').next().unwrap_or(folder);
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// `section` as a direct child of `within`, or `None` if it is not one.
///
/// `formats` is a child of ``; `formats/x` is not - it is a child of `formats`.
fn child_section(within: &str, section: &str) -> Option<String> {
    if section.is_empty() || section == within {
        return None;
    }
    let rest = if within.is_empty() {
        section
    } else {
        section.strip_prefix(within)?.strip_prefix('/')?
    };
    let head = rest.split('/').next()?;
    Some(if within.is_empty() {
        head.to_string()
    } else {
        format!("{within}/{head}")
    })
}

/// The reference index: sections as headings, pages as the same list the
/// entries use. A directory's own page is its heading, not an item beside the
/// things it introduces.
pub fn docs_index(log: &Log) -> String {
    let base = log.project.base_url.trim_end_matches('/');
    let label = log.docs_label.as_deref().unwrap_or("Reference");

    // The docs root may have its own README. When it does, that is the page;
    // the generated lead is only there for a tree that has none.
    let root = log
        .docs
        .iter()
        .find(|doc| doc.is_index && doc.slug.is_empty());
    let pages = log.docs.iter().filter(|doc| !doc.is_index).count();
    let mut body = String::from("<article>\n");
    let mut headings = Vec::new();
    // Search, as the entry list and the open questions have: over every page's
    // full text, from `docs/search.json`. The reference had none, so the only
    // way to a page was knowing which section it was filed under.
    let search = format!(
        "<div class=\"toolbar\">\n<div class=\"search\">\n\
         <svg viewBox=\"0 0 16 16\" aria-hidden=\"true\" focusable=\"false\">\
         <circle cx=\"7\" cy=\"7\" r=\"4.5\" fill=\"none\" stroke=\"currentColor\" \
         stroke-width=\"1.5\"/><path d=\"M10.5 10.5 L14 14\" stroke=\"currentColor\" \
         stroke-width=\"1.5\" stroke-linecap=\"round\"/></svg>\n\
         <input type=\"search\" id=\"q\" placeholder=\"Search {pages} pages\" \
         autocomplete=\"off\" spellcheck=\"false\">\n</div>\n</div>\n<p id=\"status\"></p>\n"
    );
    match root {
        Some(doc) => {
            let _ = writeln!(body, "<h1>{}</h1>", escape(&doc.title));
            body.push_str(&search);
            let links = Links {
                log,
                from_dir: doc_dir(doc),
                rel: "../",
                base: None,
            };
            let stripped = match doc.body.trim_start().strip_prefix("# ") {
                Some(rest) => rest.split_once('\n').map(|(_, rest)| rest).unwrap_or(""),
                None => doc.body.as_str(),
            };
            let (found, html) = markdown_with_headings(stripped, &links);
            headings = found;
            // The project's own index page, hidden while a search is showing
            // the pages that match, so the results are not under it.
            let _ = writeln!(
                body,
                "<div class=\"doc-readme\" data-hide-on-search>{html}</div>"
            );
        }
        None => {
            let _ = writeln!(body, "<h1>{}</h1>", escape(label));
            let pages: Vec<&DocPage> = log.docs.iter().filter(|doc| !doc.is_index).collect();
            let sections = log
                .docs
                .iter()
                .filter(|doc| doc.is_index && !doc.slug.is_empty())
                .count();
            let mut tally = String::new();
            for status in [
                cairns_core::Status::Solid,
                cairns_core::Status::Partial,
                cairns_core::Status::Guess,
            ] {
                let count = pages
                    .iter()
                    .filter(|doc| doc.status == Some(status))
                    .count();
                if count > 0 {
                    if !tally.is_empty() {
                        tally.push_str(" \u{00b7} ");
                    }
                    let _ = write!(tally, "{count} {}", status.name());
                }
            }
            let _ = writeln!(
                body,
                "<p class=\"lead\">What is true now, page by page; the log says how it was \
                 found out. {} {} in {} {}: {tally}.</p>",
                pages.len(),
                if pages.len() == 1 { "page" } else { "pages" },
                sections.max(1),
                if sections > 1 { "sections" } else { "section" },
            );
            body.push_str(&search);
        }
    }
    // When the docs root has its own README, that page is the index - it
    // already lists its pages, in its own words - and listing them again under
    // it doubled the page. The rows are still there for search, and only shown
    // while one is running.
    let _ = writeln!(
        body,
        "<div class=\"days doc-index\" id=\"entries\" data-noun=\"pages\"{}>\n{}</div>",
        if root.is_some() {
            " data-show-on-search hidden"
        } else {
            ""
        },
        sections(log, "", "")
    );
    body.push_str("</article>\n<script src=\"search.js\" defer></script>\n");

    shell(
        log,
        &format!("{label} - {}", log.project.name),
        "What is true, flatly.",
        &format!("{base}/docs/"),
        "../",
        "docs",
        &docs_tree(log, "../", ""),
        &contents_nav(&headings),
        &body,
    )
}

/// The pages of one section, and then each section below it.
///
/// `within` is the section being listed; `rel` is the path from the page doing
/// the listing back to the docs root.
fn sections(log: &Log, within: &str, rel: &str) -> String {
    let mut out = String::new();

    let here: Vec<&DocPage> = log
        .docs
        .iter()
        .filter(|doc| !doc.is_index && doc.section == within)
        .collect();
    out.push_str(&doc_cards(&here, rel));

    // Every folder under this one is a section: named and introduced by its
    // own README when it has one, by its directory when it does not. Only
    // folders with a README used to be listed, so a tree like piney_apples',
    // fifty-two pages in folders without one, had an index that listed none.
    let mut folders: Vec<String> = log
        .docs
        .iter()
        .filter_map(|doc| child_section(within, &doc.section))
        .collect();
    folders.sort();
    folders.dedup();
    for folder in folders {
        let index = log
            .docs
            .iter()
            .find(|doc| doc.is_index && doc.slug == folder);
        // Everything under the folder, however deep, under its one heading.
        let inside: Vec<&DocPage> = log
            .docs
            .iter()
            .filter(|doc| {
                !doc.is_index
                    && (doc.section == folder || doc.section.starts_with(&format!("{folder}/")))
            })
            .collect();
        let heading = match index {
            Some(index) => format!(
                "<a href=\"{rel}{}/\">{}</a>",
                escape(&index.slug),
                escape(&index.title)
            ),
            None => escape(&folder_name(&folder)),
        };
        // Headed the way the entry list heads a day, so the two indexes read as
        // one site rather than two.
        let _ = writeln!(
            out,
            "<section class=\"day doc-section\">\n<h2 class=\"day-label\">{heading}\
             <span class=\"day-count\">{}</span></h2>",
            inside.len()
        );
        if let Some(lead) = index.and_then(|index| first_sentence_of(&index.body)) {
            let _ = writeln!(
                out,
                "<p class=\"section-lead\">{}</p>",
                escape(&plain_md(&lead))
            );
        }
        out.push_str(&doc_cards(&inside, rel));
        out.push_str("</section>\n");
    }
    out
}

/// Pages as rows - the entry list's rows, title, summary and a meta line - so a
/// reader moving between the two indexes is on the same kind of page.
fn doc_cards(pages: &[&DocPage], rel: &str) -> String {
    if pages.is_empty() {
        return String::new();
    }
    let mut out = String::from("<ul class=\"entries doc-list\">\n");
    for doc in pages {
        out.push_str(&doc_row(doc, rel));
    }
    out.push_str("</ul>\n");
    out
}

fn doc_dir(doc: &DocPage) -> &str {
    doc.path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("")
}

fn doc_row(doc: &DocPage, rel: &str) -> String {
    let summary = first_sentence_of(&doc.body)
        .map(|summary| {
            format!(
                "<span class=\"summary\">{}</span>\n",
                escape(&plain_md(&summary))
            )
        })
        .unwrap_or_default();
    let status = doc
        .status
        .map(|status| {
            format!(
                "<span class=\"status status--{0}\">{0}</span>",
                status.name()
            )
        })
        .unwrap_or_default();
    let evidence = match doc.worklog.len() {
        0 => String::new(),
        1 => "<span>from 1 entry</span>".to_string(),
        n => format!("<span>from {n} entries</span>"),
    };
    format!(
        "<li data-n=\"{slug}\">\n<a class=\"row\" href=\"{rel}{slug}/\">\n<span class=\"row-main\">\n\
         <span class=\"row-title\">{title}</span>\n{summary}\
         <span class=\"meta\">{status}{evidence}</span>\n</span>\n</a>\n</li>\n",
        slug = escape(&doc.slug),
        title = escape(&doc.title)
    )
}

/// The first sentence of a page, for the index blurb.
fn first_sentence_of(body: &str) -> Option<String> {
    let prose = body
        .lines()
        .skip_while(|line| line.starts_with('#') || line.trim().is_empty())
        .take_while(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let prose = prose.trim();
    if prose.is_empty() {
        return None;
    }
    // A paragraph that leads into a list ends with a colon, and the colon
    // is not the end of a sentence: "... side effects:." was the result.
    Some(match prose.find(". ") {
        Some(stop) => prose[..=stop].trim().to_string(),
        None => prose.trim_end_matches(['.', ':', ';', ',']).to_string() + ".",
    })
}

/// One reference page. The dateline is the entry page's dateline.
pub fn doc_page(log: &Log, doc: &DocPage) -> String {
    let depth = doc.slug.matches('/').count() + 2;
    let rel = "../".repeat(depth);
    let by_number: BTreeMap<u32, &LogEntry> = log
        .entries
        .iter()
        .map(|entry| (entry.number, entry))
        .collect();

    // The same head an entry has - a label above the title, then a line of
    // pills - so a reference page reads as part of the same site. It had a
    // bare title and a line of plain words, sitting higher than an entry's.
    let label = log.docs_label.as_deref().unwrap_or("Reference");
    let section = log
        .docs
        .iter()
        .find(|other| other.is_index && other.slug == doc.section && !doc.is_index)
        .map(|other| other.title.clone())
        .or_else(|| (!doc.section.is_empty() && !doc.is_index).then(|| folder_name(&doc.section)));
    let mut body = String::from("<article>\n");
    let _ = writeln!(
        body,
        "<p class=\"entry-number\">{}</p>",
        match &section {
            Some(section) => format!("{} \u{00b7} {}", escape(label), escape(section)),
            None => escape(label),
        }
    );
    let _ = writeln!(body, "<h1>{}</h1>", escape(&doc.title));

    let _ = write!(body, "<p class=\"dateline\">");
    if let Some(status) = doc.status {
        let _ = write!(
            body,
            "<span class=\"chip status status--{0}\">{0}</span>",
            status.name()
        );
    }
    // The evidence: the entries this page rests on, as the same pills an
    // entry's areas are, each with its title on hover - this log's first,
    // then other worklogs'.
    if !doc.worklog.is_empty() || !doc.elsewhere.is_empty() {
        body.push_str("<span class=\"from\">from</span>");
        body.push_str(&evidence_chips(
            log,
            &doc.worklog,
            &doc.elsewhere,
            &rel,
            &by_number,
        ));
    }
    body.push_str("</p>\n");
    body.push_str(&covers_html(&doc.covers));

    let from_dir = doc_dir(doc);
    let links = Links {
        log,
        from_dir,
        rel: &rel,
        base: None,
    };
    let stripped = match doc.body.trim_start().strip_prefix("# ") {
        Some(rest) => rest.split_once('\n').map(|(_, rest)| rest).unwrap_or(""),
        None => doc.body.as_str(),
    };
    let (headings, mut html) = markdown_with_headings(stripped, &links);
    // A section's own evidence, under its heading: the place a reader asks
    // "where was this found out?" is the section they are reading.
    for section in &doc.sections {
        let opening = format!(" id=\"{}\">", section.anchor);
        let Some(at) = html.find(&opening) else {
            continue;
        };
        let Some(close) = html[at..].find("</h").map(|c| at + c) else {
            continue;
        };
        let Some(end) = html[close..].find('>').map(|e| close + e + 1) else {
            continue;
        };
        let line = format!(
            "\n<p class=\"dateline section-from\"><span class=\"from\">from</span>{}</p>",
            evidence_chips(log, &section.worklog, &section.elsewhere, &rel, &by_number)
        );
        html.insert_str(end, &line);
    }
    body.push_str(&html);

    if doc.is_index {
        let inside = sections(
            log,
            &doc.slug,
            &"../".repeat(doc.slug.matches('/').count() + 1),
        );
        if !inside.is_empty() {
            body.push_str("<h2>Pages</h2>\n");
            body.push_str(&inside);
        }
    }
    body.push_str("</article>\n");

    // The neighbours in its own section, in the order the rail lists them: a
    // reference is read a section at a time, and the way to the next page was
    // back up to the tree.
    if !doc.is_index {
        let siblings: Vec<&DocPage> = log
            .docs
            .iter()
            .filter(|other| other.section == doc.section && !other.is_index)
            .collect();
        if let Some(at) = siblings.iter().position(|other| other.slug == doc.slug) {
            body.push_str("<nav class=\"pager\">\n");
            for (class, label, other) in [
                (
                    "prev",
                    "Previous",
                    at.checked_sub(1).and_then(|i| siblings.get(i)),
                ),
                ("next", "Next", siblings.get(at + 1)),
            ] {
                if let Some(other) = other {
                    let _ = writeln!(
                        body,
                        "<a class=\"{class}\" href=\"{rel}docs/{}/\"><span class=\"label\">{label}</span>{}</a>",
                        escape(&other.slug),
                        escape(&other.title)
                    );
                }
            }
            body.push_str("</nav>\n");
            body.push_str(ARROW_KEYS);
        }
    }

    shell(
        log,
        &format!("{} - {}", doc.title, log.project.name),
        &doc.title,
        &doc.url,
        &rel,
        "docs",
        &docs_tree(log, &rel, &doc.slug),
        &contents_nav(&headings),
        &body,
    )
}

#[cfg(test)]
mod footer_tests {
    use super::generator_html;

    #[test]
    fn the_footer_names_the_version_that_made_the_page() {
        assert_eq!(
            generator_html("cairns 0.10.1"),
            "<a href=\"https://github.com/Toyz/cairns/releases/tag/v0.10.1\">cairns 0.10.1</a>"
        );
        assert!(generator_html("cairns 0.0.0-dev").contains("development build"));
    }
}

#[cfg(test)]
mod tests {
    use super::from_repo_root;

    #[test]
    fn a_link_out_of_the_entries_directory_lands_at_the_repository_root() {
        assert_eq!(
            from_repo_root("worklog", "../docs/spec/entry.md"),
            "docs/spec/entry.md"
        );
        assert_eq!(from_repo_root("worklog", "../README.md"), "README.md");
        // No `../` means a path inside the entries directory, which is what a
        // relative link in that file actually means.
        assert_eq!(from_repo_root("worklog", "notes.md"), "worklog/notes.md");
        assert_eq!(from_repo_root("worklog", "./notes.md"), "worklog/notes.md");
        assert_eq!(
            from_repo_root("log/entries", "../../docs/x.md"),
            "docs/x.md"
        );
    }
}

#[cfg(test)]
mod tree_tests {
    use super::child_section;

    #[test]
    fn a_section_is_a_child_of_exactly_one_folder() {
        assert_eq!(child_section("", "formats").as_deref(), Some("formats"));
        assert_eq!(
            child_section("", "formats/inner").as_deref(),
            Some("formats")
        );
        assert_eq!(
            child_section("formats", "formats/inner").as_deref(),
            Some("formats/inner")
        );
        // Its own section is not a child of itself - this is the shape that
        // recursed until the stack ran out.
        assert_eq!(child_section("formats", "formats"), None);
        assert_eq!(child_section("", ""), None);
        assert_eq!(child_section("port", "formats"), None);
    }
}
