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

        let (path, fragment) = match url.split_once('#') {
            Some((path, fragment)) => (path, format!("#{fragment}")),
            None => (url, String::new()),
        };

        // Everything is decided on the repo-root path the link points at, so
        // `entry.md`, `../spec/entry.md` and `docs/spec/entry.md` all land in
        // the same place from wherever they were written.
        let target = from_repo_root(self.from_dir, path);

        if let Some(entry) = self.log.entries.iter().find(|entry| entry.path == target) {
            return format!(
                "{}{fragment}",
                self.site(&format!("{}-{}/", entry.number, entry.slug))
            );
        }
        if let Some(doc) = self.log.docs.iter().find(|doc| doc.path == target) {
            return format!("{}{fragment}", self.site(&format!("docs/{}/", doc.slug)));
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
    let mut events: Vec<Event> = Parser::new_ext(text, options())
        .map(|event| match event {
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => Event::Start(Tag::Link {
                link_type,
                dest_url: links.resolve(&dest_url).into(),
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
    html::push_html(&mut out, events.into_iter());
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
{aside}</aside>
<main>
{body}
<footer><p>Generated by <a href="https://github.com/Toyz/cairns">cairns</a>.</p></footer>
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
        contents = contents,
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
    for area in log.areas.iter().filter(|area| area.count > 0) {
        let _ = writeln!(
            chips,
            "<button class=\"chip\" data-area=\"{name}\" aria-pressed=\"false\" \
             title=\"{about}\"><span>{name}</span><span class=\"count\">{count}</span></button>",
            name = escape(&area.name),
            about = escape(&area.about),
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
    body.push_str(
        "<div class=\"sort\" role=\"group\" aria-label=\"Order\">\n\
         <button data-sort=\"new\" aria-pressed=\"true\">Newest</button>\n\
         <button data-sort=\"old\" aria-pressed=\"false\">Oldest</button>\n\
         </div>\n</div>\n<p id=\"status\"></p>\n<ul class=\"entries\" id=\"entries\">\n",
    );

    for entry in log.entries.iter().rev() {
        let _ = writeln!(
            body,
            "<li data-n=\"{n}\" data-areas=\"{slugs}\">\n\
             <a class=\"row\" href=\"{path}/\">\n\
             <span class=\"no\">{n}</span>\n<span class=\"row-main\">\n\
             <span class=\"row-title\">{title}</span>",
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
            "<span class=\"meta\"><time datetime=\"{date}\">{date}</time>\
             <span class=\"areas\">{areas}</span></span>\n</span>\n</a>\n</li>",
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
        "entries",
        &chips,
        "",
        &body,
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
        || !this.documented_by.is_empty()
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
        if !this.documented_by.is_empty() {
            let _ = writeln!(
                body,
                "<p>Documented in {}.</p>",
                this.documented_by
                    .iter()
                    .map(|title| escape(title))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        if !this.resolves.is_empty() {
            let who: Vec<String> = this.resolves.iter().map(|n| link(*n)).collect();
            let _ = writeln!(
                body,
                "<p>This entry answers the question left open by {}.</p>",
                who.join(", ")
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
    if !this.files.is_empty() {
        body.push_str("<p class=\"files\"><span class=\"files-label\">Files</span>");
        for file in &this.files {
            let _ = write!(
                body,
                "<a href=\"{}\"><code>{}</code></a>",
                escape(&links.resolve(&format!("../{file}"))),
                escape(file)
            );
        }
        body.push_str("</p>\n");
    }

    let (headings, prose_html) = markdown_with_headings(prose(this), &links);
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
        "<p class=\"lead\">{} of {} entries end with something unresolved. \
         Each is quoted as its entry left it.</p>",
        log.open_questions.len(),
        log.entries.len()
    );

    if log.open_questions.is_empty() {
        body.push_str("<p class=\"empty\">Nothing open.</p>\n");
    } else {
        body.push_str("<ul class=\"questions\">\n");
        for question in &log.open_questions {
            body.push_str("<li>\n");
            let _ = writeln!(
                body,
                "<div class=\"question\">{}</div>",
                markdown(&question.text)
            );
            // The question is a sentence fragment out of context, so it is
            // always shown with the entry that raised it, by name.
            match by_number.get(&question.entry) {
                Some(entry) => {
                    let _ = writeln!(
                        body,
                        "<p class=\"source\"><a href=\"../{path}/\">No. {n} \u{2014} {title}</a>\
                         <time datetime=\"{date}\">{date}</time></p>",
                        path = escape(&path_of(entry)),
                        n = entry.number,
                        title = escape(&entry.title),
                        date = entry.date
                    );
                }
                None => {
                    let _ = writeln!(body, "<p class=\"source\">No. {}</p>", question.entry);
                }
            }
            body.push_str("</li>\n");
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
        "open",
        "",
        "",
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
    let mut out = format!(
        "<div class=\"tree\">\n<p class=\"rail-label\">{}</p>\n",
        escape(label)
    );
    out.push_str(&branch(log, rel, here, ""));
    out.push_str("</div>\n");
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
            None => format!(
                "<span>{}</span>",
                escape(folder.rsplit('/').next().unwrap_or(&folder))
            ),
        };
        let _ = writeln!(out, "<li class=\"folder\">{heading}");
        out.push_str(&branch(log, rel, here, &folder));
        out.push_str("</li>\n");
    }
    out.push_str("</ul>\n");
    out
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
    let mut body = String::from("<article>\n");
    match root {
        Some(doc) => {
            let _ = writeln!(body, "<h1>{}</h1>", escape(&doc.title));
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
            body.push_str(&markdown_with_headings(stripped, &links).1);
        }
        None => {
            let _ = writeln!(body, "<h1>{}</h1>", escape(label));
            let pages = log.docs.iter().filter(|doc| !doc.is_index).count();
            let _ = writeln!(
                body,
                "<p class=\"lead\">{pages} pages: what is true, flatly. The log says \
                 how it was found out.</p>"
            );
        }
    }
    body.push_str(&sections(log, "", ""));
    body.push_str("</article>\n");

    shell(
        log,
        &format!("{label} - {}", log.project.name),
        "What is true, flatly.",
        &format!("{base}/docs/"),
        "../",
        "docs",
        &docs_tree(log, "../", ""),
        "",
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
    if !here.is_empty() {
        out.push_str("<ul class=\"entries\">\n");
        for doc in here {
            out.push_str(&doc_row(doc, rel));
        }
        out.push_str("</ul>\n");
    }

    for index in log
        .docs
        .iter()
        .filter(|doc| doc.is_index && doc.section == within && doc.slug != within)
    {
        let _ = writeln!(
            out,
            "<h2 class=\"section\"><a href=\"{rel}{}/\">{}</a></h2>",
            escape(&index.slug),
            escape(&index.title)
        );
        let inside: Vec<&DocPage> = log
            .docs
            .iter()
            .filter(|doc| !doc.is_index && doc.section == index.slug)
            .collect();
        if inside.is_empty() {
            continue;
        }
        out.push_str("<ul class=\"entries\">\n");
        for doc in inside {
            out.push_str(&doc_row(doc, rel));
        }
        out.push_str("</ul>\n");
    }
    out
}

fn doc_dir(doc: &DocPage) -> &str {
    doc.path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("")
}

fn doc_row(doc: &DocPage, rel: &str) -> String {
    let mut row = String::new();
    let _ = writeln!(
        row,
        "<li>\n<p class=\"meta\">{status}{cites}</p>\n\
         <h3><a href=\"{rel}{slug}/\">{title}</a></h3>",
        status = doc
            .status
            .map(|status| format!(
                "<span class=\"status status--{0}\">{0}</span>",
                status.name()
            ))
            .unwrap_or_default(),
        cites = if doc.worklog.is_empty() {
            String::new()
        } else {
            format!(
                "<span>from {}</span>",
                doc.worklog
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
        slug = escape(&doc.slug),
        title = escape(&doc.title)
    );
    if let Some(summary) = first_sentence_of(&doc.body) {
        let _ = writeln!(
            row,
            "<p class=\"summary\">{}</p>",
            escape(&plain_md(&summary))
        );
    }
    row.push_str("</li>\n");
    row
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
    Some(match prose.find(". ") {
        Some(stop) => prose[..=stop].trim().to_string(),
        None => prose.trim_end_matches('.').to_string() + ".",
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

    let mut body = String::from("<article>\n");
    let _ = writeln!(body, "<h1>{}</h1>", escape(&doc.title));

    let _ = write!(body, "<p class=\"dateline\">");
    if !doc.section.is_empty() {
        let _ = write!(body, "<span>{}</span>", escape(&doc.section));
    }
    if let Some(status) = doc.status {
        let _ = write!(
            body,
            "<span class=\"status status--{0}\">{0}</span>",
            status.name()
        );
    }
    // The edge back to the evidence, which is the point of keeping both.
    if !doc.worklog.is_empty() {
        // Bare blue numbers read as nothing. They are the same pills the areas
        // use, labelled, so "from entry 6" is what the reader sees.
        body.push_str("<span class=\"from\">from</span>");
        for number in &doc.worklog {
            match by_number.get(number) {
                Some(entry) => {
                    let _ = write!(
                        body,
                        "<a class=\"chip\" href=\"{rel}{}-{}/\" title=\"{}\">entry {number}</a>",
                        entry.number,
                        escape(&entry.slug),
                        escape(&entry.title)
                    );
                }
                None => {
                    let _ = write!(body, "<span class=\"chip\">entry {number}</span>");
                }
            }
        }
    }
    body.push_str("</p>\n");

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
    let (_, html) = markdown_with_headings(stripped, &links);
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

    shell(
        log,
        &format!("{} - {}", doc.title, log.project.name),
        &doc.title,
        &doc.url,
        &rel,
        "docs",
        &docs_tree(log, &rel, &doc.slug),
        "",
        &body,
    )
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
