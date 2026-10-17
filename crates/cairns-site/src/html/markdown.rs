//! Markdown, and the links in it: where a written link goes on the site.

use super::*;

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

pub(crate) fn options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_FOOTNOTES
}

/// Where a page's links go, as written: for `check` to find the ones that go
/// nowhere. Links inside code are not links, which a parser knows and a
/// pattern does not.
pub fn link_targets(text: &str) -> Vec<String> {
    Parser::new_ext(text, options())
        .filter_map(|event| match event {
            Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) => {
                Some(dest_url.to_string())
            }
            _ => None,
        })
        .collect()
}

/// Where a page's images come from, as written - its links' targets without
/// the links that only go somewhere.
pub fn image_targets(text: &str) -> Vec<String> {
    Parser::new_ext(text, options())
        .filter_map(|event| match event {
            Event::Start(Tag::Image { dest_url, .. }) => Some(dest_url.to_string()),
            _ => None,
        })
        .collect()
}

/// The prose alone: the entry's own `# N. Title` and its open-question trailer
/// are rendered by the page, in their own places, so they are cut from here
/// rather than appearing twice.
pub(crate) fn prose(entry: &LogEntry) -> &str {
    let text = cairns_core::entry::without_title(&entry.body);
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
    pub(crate) fn site(&self, path: &str) -> String {
        match self.base {
            Some(base) => format!("{}/{path}", base.trim_end_matches('/')),
            None => format!("{}{path}", self.rel),
        }
    }

    pub(crate) fn resolve(&self, url: &str) -> String {
        if leaves_repository(url) {
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
        let Some(target) = from_repo_root(self.from_dir, path) else {
            return url.to_string();
        };

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

/// A link that is not to a file in the repository: a URL, an address, a link
/// to a place on the same page.
pub(crate) fn leaves_repository(url: &str) -> bool {
    url.is_empty()
        || url.starts_with('#')
        || url.contains("://")
        || url.starts_with("mailto:")
        || url.starts_with("//")
}

/// The file in the repository a link written in `from_dir` leads to, when it
/// is a relative link to one - not a URL, an anchor, a `[[12]]` reference, or
/// a path from a site's root - with its `#section` and `?query` dropped. `None`
/// too for a path that climbs out of the repository.
pub fn repo_path(from_dir: &str, url: &str) -> Option<String> {
    if leaves_repository(url) || url.starts_with('/') || url.starts_with("cairns:") {
        return None;
    }
    let path = url
        .split(['#', '?'])
        .next()
        .unwrap_or("")
        .replace("%20", " ");
    if path.is_empty() {
        return None;
    }
    from_repo_root(from_dir, &path)
}

/// A path written inside a page in `dir`, as a path from the repository root -
/// none when it climbs above the root, where nothing of the repository is.
pub(crate) fn from_repo_root(dir: &str, path: &str) -> Option<String> {
    let mut parts: Vec<&str> = dir.split('/').filter(|part| !part.is_empty()).collect();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            name => parts.push(name),
        }
    }
    Some(parts.join("/"))
}

/// Where the entries live, taken from the paths in the document itself rather
/// than from config the renderer does not read.
pub(crate) fn entry_dir(log: &Log) -> &str {
    log.entries
        .first()
        .map_or("", |entry| cairns_core::dir_of(&entry.path))
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
pub(crate) fn markdown_with_headings(text: &str, links: &Links<'_>) -> (Vec<Heading>, String) {
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
pub(crate) fn markdown_linked(text: &str, links: &Links<'_>) -> String {
    markdown_with_headings(text, links).1
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

/// The first sentence of a page, for the index blurb.
pub(crate) fn first_sentence_of(body: &str) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::from_repo_root;

    #[test]
    fn a_link_out_of_the_entries_directory_lands_at_the_repository_root() {
        let at = |dir, path| from_repo_root(dir, path);
        assert_eq!(
            at("worklog", "../docs/spec/entry.md").as_deref(),
            Some("docs/spec/entry.md")
        );
        assert_eq!(at("worklog", "../README.md").as_deref(), Some("README.md"));
        // No `../` means a path inside the entries directory, which is what a
        // relative link in that file actually means.
        assert_eq!(
            at("worklog", "notes.md").as_deref(),
            Some("worklog/notes.md")
        );
        assert_eq!(
            at("worklog", "./notes.md").as_deref(),
            Some("worklog/notes.md")
        );
        assert_eq!(
            at("log/entries", "../../docs/x.md").as_deref(),
            Some("docs/x.md")
        );
        assert_eq!(at("worklog", "../../outside.md"), None);
    }
}
