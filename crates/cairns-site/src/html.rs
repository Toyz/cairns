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

/// How a link written inside an entry is turned into one that resolves.
///
/// The log links to other entries by filename - `[4](0004-the-write-path.md)` -
/// which is right in the repo and on GitHub and dead on the site, where that
/// entry is a directory named after its number and slug. Anything else
/// relative is a file in the repository, so it points there.
pub struct Links<'a> {
    pub entries: &'a BTreeMap<u32, &'a LogEntry>,
    /// The directory the entries live in. A relative link inside an entry is
    /// relative to *that*, so `../docs/spec/entry.md` is `docs/spec/entry.md`
    /// at the repository root - and saying so needs to know where it started.
    pub entry_dir: &'a str,
    pub repository: Option<&'a str>,
    /// `None` renders entry links relative, for a page that sits beside them;
    /// `Some(base)` renders them absolute, for the feed.
    pub base: Option<&'a str>,
}

impl Links<'_> {
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

        // `0004-the-write-path.md`, however it was reached.
        let name = path.rsplit('/').next().unwrap_or(path);
        if let Some(stem) = name.strip_suffix(".md")
            && let Some((digits, _)) = stem.split_once('-')
            && let Ok(number) = digits.parse::<u32>()
            && let Some(entry) = self.entries.get(&number)
        {
            let target = format!("{}-{}", entry.number, entry.slug);
            return match self.base {
                Some(base) => format!("{}/{target}{fragment}", base.trim_end_matches('/')),
                None => format!("../{target}/{fragment}"),
            };
        }

        match self.repository {
            Some(repo) => format!(
                "{}/blob/HEAD/{}{fragment}",
                repo.trim_end_matches('/'),
                from_repo_root(self.entry_dir, path)
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

/// Where the contents list goes: after the dateline, before the prose.
const CONTENTS_SLOT: &str = "<!--cairns:contents-->";

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

/// An entry's prose as HTML, for the feed, with every link absolute - a feed
/// reader has no page to resolve a relative one against.
pub fn body_html(log: &Log, entry: &LogEntry) -> String {
    let by_number: BTreeMap<u32, &LogEntry> = log
        .entries
        .iter()
        .map(|entry| (entry.number, entry))
        .collect();
    let links = Links {
        entries: &by_number,
        entry_dir: entry_dir(log),
        repository: log.project.repository.as_deref(),
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

/// The page shell. `rel` is the path back to the site root from this page, so
/// one renderer serves the root pages and the entry pages alike.
fn shell(log: &Log, title: &str, description: &str, url: &str, rel: &str, body: &str) -> String {
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
<div class="wrap">
<header class="masthead">
<h1><a href="{rel}">{site}</a></h1>
{tagline}<nav><a href="{rel}">Entries</a>{about}<a href="{rel}open/">Open questions</a><a href="{rel}feed.xml">Feed</a></nav>
</header>
{body}
<footer>
<p>Generated by <a href="https://github.com/Toyz/cairns">cairns</a>.</p>
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
            "<li data-n=\"{n}\" data-areas=\"{slugs}\">\n\
             <p class=\"meta\"><span class=\"no\">{n}</span>\
             <time datetime=\"{date}\">{date}</time>\
             <span class=\"areas\">{areas}</span></p>\n\
             <h2><a href=\"{path}/\">{title}</a></h2>",
            n = entry.number,
            slugs = escape(&entry.areas.join(" ")),
            date = entry.date,
            areas = escape(&entry.areas.join(", ")),
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
        body.push_str("</li>\n");
    }

    body.push_str("</ul>\n<script src=\"search.js\" defer></script>\n");

    let description = if log.project.description.is_empty() {
        format!("{} entries.", log.entries.len())
    } else {
        plain_md(&log.project.description)
    };
    shell(log, &log.project.name, &description, base, "./", &body)
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
    if !this.superseded_by.is_empty() || !this.supersedes.is_empty() || !this.resolves.is_empty() {
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
        entries: by_number,
        entry_dir: entry_dir(log),
        repository: log.project.repository.as_deref(),
        base: None,
    };
    let (headings, prose_html) = markdown_with_headings(prose(this), &links);
    body.push_str(CONTENTS_SLOT);
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
    let body = body.replace(CONTENTS_SLOT, &contents);

    let description = plain_md(this.summary.as_deref().unwrap_or(&this.title));
    let title = format!("{}. {}", this.number, this.title);
    shell(log, &title, &description, &this.url, "../", &body)
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
