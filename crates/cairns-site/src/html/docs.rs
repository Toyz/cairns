//! The reference: its index, its pages, and the tree in the rail.

use super::*;

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

pub(crate) fn branch(log: &Log, rel: &str, here: &str, within: &str) -> String {
    let pages: Vec<&DocPage> = log
        .docs
        .iter()
        .filter(|doc| doc.section == within && !doc.is_index)
        .collect();

    // Folders come from the directories themselves, not only from directories
    // that happen to contain a README. Hellbender's docs tree has four
    // subdirectories and no index page in any of them, and deriving folders
    // from index pages alone left its rail empty.
    let folders = crate::reference::child_sections(log, within);

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
        let named = crate::reference::section_page(log, &folder);
        let title = crate::reference::section_title(log, &folder);
        let heading = match named {
            Some(doc) => format!(
                "<a href=\"{rel}docs/{slug}/\"{current}>{}</a>",
                escape(&title),
                slug = escape(&doc.slug),
                current = if doc.slug == here {
                    " aria-current=\"page\""
                } else {
                    ""
                },
            ),
            None => format!("<span>{}</span>", escape(&title)),
        };
        // Every folder folds, and only the one holding this page starts open.
        // Sixty pages listed in full is not navigation; four folders with
        // their counts is.
        let inside = crate::reference::pages_under(log, &folder).len();
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
        "<div class=\"toolbar\">\n{}</div>\n<p id=\"status\"></p>\n",
        search_field(&format!("Search {pages} pages"))
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
            // A generated listing below the marker is for the forge, where the
            // README is all there is; here the site's own rows list the pages.
            let own = crate::reference::above_marker(&doc.body);
            let stripped = cairns_core::entry::without_title(own);
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
    // while one is running. A README whose listing is generated below the
    // marker has had that listing taken off, so the rows are its listing.
    let listed_by_hand = root.is_some_and(|doc| !crate::reference::has_marker(&doc.body));
    let _ = writeln!(
        body,
        "<div class=\"days doc-index\" id=\"entries\" data-noun=\"pages\"{}>\n{}</div>",
        if listed_by_hand {
            " data-show-on-search hidden"
        } else {
            ""
        },
        sections(log, "", "")
    );
    body.push_str("</article>\n<script src=\"search.js\" defer></script>\n");

    // The sections the site lists are the index's contents, after any
    // headings of the project's own above them.
    if !listed_by_hand {
        for folder in crate::reference::child_sections(log, "") {
            if crate::reference::pages_under(log, &folder).is_empty() {
                continue;
            }
            headings.push(Heading {
                level: 2,
                label: crate::reference::section_title(log, &folder),
                id: section_anchor(&folder),
            });
        }
    }

    shell(
        log,
        Page {
            title: &format!("{label} - {}", log.project.name),
            description: "What is true, flatly.",
            url: &format!("{base}/docs/"),
            rel: "../",
            nav: Nav::Docs,
            aside: &docs_tree(log, "../", ""),
            contents: &contents_nav(&headings),
            body: &body,
        },
    )
}

/// The pages of one section, and then each section below it.
///
/// `within` is the section being listed; `rel` is the path from the page doing
/// the listing back to the docs root.
pub(crate) fn sections(log: &Log, within: &str, rel: &str) -> String {
    let mut out = String::new();

    let here: Vec<&DocPage> = log
        .docs
        .iter()
        .filter(|doc| !doc.is_index && doc.section == within)
        .collect();
    out.push_str(&doc_cards(log, &here, rel));

    // Every folder under this one is a section: named and introduced by its
    // own README when it has one, by its directory when it does not. Only
    // folders with a README used to be listed, so a tree like piney_apples',
    // fifty-two pages in folders without one, had an index that listed none.
    // In the order `cairns.toml` declares them, under the titles it gives.
    for folder in crate::reference::child_sections(log, within) {
        let index = crate::reference::section_page(log, &folder);
        // Everything under the folder, however deep, under its one heading.
        let inside = crate::reference::pages_under(log, &folder);
        if inside.is_empty() {
            continue;
        }
        let title = crate::reference::section_title(log, &folder);
        let heading = match index {
            Some(index) => format!(
                "<a href=\"{rel}{}/\">{}</a>",
                escape(&index.slug),
                escape(&title)
            ),
            None => escape(&title),
        };
        // Headed the way the entry list heads a day, so the two indexes read as
        // one site rather than two.
        let _ = writeln!(
            out,
            "<section class=\"day doc-section\" id=\"{}\">\n<h2 class=\"day-label\">{heading}\
             <span class=\"day-count\">{}</span></h2>",
            section_anchor(&folder),
            inside.len()
        );
        let lead = crate::reference::section_about(log, &folder).or_else(|| {
            index.and_then(|index| first_sentence_of(crate::reference::above_marker(&index.body)))
        });
        if let Some(lead) = lead {
            let _ = writeln!(
                out,
                "<p class=\"section-lead\">{}</p>",
                escape(&plain_md(&lead))
            );
        }
        out.push_str(&doc_cards(log, &inside, rel));
        out.push_str("</section>\n");
    }
    out
}

/// The anchor a section's heading on an index has: `section-formats`, so it
/// cannot collide with a heading in the prose above it.
pub(crate) fn section_anchor(folder: &str) -> String {
    format!("section-{}", cairns_core::entry::slugify(folder))
}

/// Pages as rows - the entry list's rows, title, summary and a meta line - so a
/// reader moving between the two indexes is on the same kind of page.
pub(crate) fn doc_cards(log: &Log, pages: &[&DocPage], rel: &str) -> String {
    if pages.is_empty() {
        return String::new();
    }
    let mut out = String::from("<ul class=\"entries doc-list\">\n");
    for doc in pages {
        out.push_str(&doc_row(log, doc, rel));
    }
    out.push_str("</ul>\n");
    out
}

pub(crate) fn doc_dir(doc: &DocPage) -> &str {
    cairns_core::dir_of(&doc.path)
}

pub(crate) fn doc_row(log: &Log, doc: &DocPage, rel: &str) -> String {
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
    // The project's own fields, as the generated README's columns have them -
    // `volumes: all` - each titled with its label for a reader hovering it.
    // `covers` is left to the page: on a row it ran to a paragraph.
    let fields: String = crate::reference::columns(log)
        .into_iter()
        .filter(|field| field.name != "covers")
        .filter_map(|field| {
            let value = crate::reference::field_value(doc, &field.name)?;
            Some(format!(
                "<span title=\"{}\">{}</span>",
                escape(&field.label()),
                escape(&value)
            ))
        })
        .collect();
    let evidence = match doc.worklog.len() {
        0 => String::new(),
        1 => "<span>from 1 entry</span>".to_string(),
        n => format!("<span>from {n} entries</span>"),
    };
    format!(
        "<li data-n=\"{slug}\">\n<a class=\"row\" href=\"{rel}{slug}/\">\n<span class=\"row-main\">\n\
         <span class=\"row-title\">{title}</span>\n{summary}\
         <span class=\"meta\">{status}{fields}{evidence}</span>\n</span>\n</a>\n</li>\n",
        slug = escape(&doc.slug),
        title = escape(&doc.title)
    )
}

/// What a reader should know before trusting a page, above it: evidence a
/// later entry corrected, what the page says it does not know, and where it
/// is linked from - the notice an entry has, for a page.
pub(crate) fn doc_notice(log: &Log, doc: &DocPage, rel: &str, backlinks: &Backlinks) -> String {
    let link = |number: u32| match log.entry(number) {
        Some(entry) => format!(
            "<a href=\"{rel}{}-{}/\">#{number} {}</a>",
            entry.number,
            escape(&entry.slug),
            escape(&entry.title)
        ),
        None => format!("#{number}"),
    };
    let mut lines = Vec::new();

    // Evidence a later entry overturned, unless the page cites the
    // correction too - then it was most likely brought up to date with it.
    // `cairns doc list` flags the same thing; here the reader sees it.
    let mut cited: Vec<u32> = doc.worklog.clone();
    for section in &doc.sections {
        cited.extend(&section.worklog);
    }
    cited.sort_unstable();
    cited.dedup();
    for number in &cited {
        let Some(entry) = log.entry(*number) else {
            continue;
        };
        let later: Vec<u32> = entry
            .superseded_by
            .iter()
            .copied()
            .filter(|n| !cited.contains(n))
            .collect();
        if later.is_empty() {
            continue;
        }
        lines.push(format!(
            "<p><strong>Rests on a corrected entry.</strong> {} was corrected by {} - \
             this page may still say what it claimed.</p>",
            link(*number),
            later
                .iter()
                .map(|n| link(*n))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // What it says it does not know, counted, one click from the top.
    if let Some(unknown) = cairns_core::doc::unknown_section(&doc.body) {
        let anchor = cairns_core::doc::unknown_heading(&doc.body)
            .map(|name| cairns_core::entry::slugify(&name.replace('`', "")));
        let count = question_count(&unknown);
        let what = if count == 1 {
            "1 thing".to_string()
        } else {
            format!("{count} things")
        };
        lines.push(match anchor {
            Some(anchor) => format!(
                "<p>This page names {what} it does not know yet - \
                 <a href=\"#{anchor}\">see what</a>.</p>"
            ),
            None => format!("<p>This page names {what} it does not know yet.</p>"),
        });
    }

    if let Some((pages, entries)) = backlinks.0.get(&doc.path) {
        let mut from: Vec<String> = pages
            .iter()
            .filter_map(|slug| log.docs.iter().find(|other| &other.slug == slug))
            .map(|other| {
                format!(
                    "<a href=\"{rel}docs/{}/\">{}</a>",
                    escape(&other.slug),
                    escape(&other.title)
                )
            })
            .collect();
        from.extend(entries.iter().map(|n| link(*n)));
        if !from.is_empty() {
            lines.push(format!("<p>Linked from {}.</p>", from.join(", ")));
        }
    }

    if lines.is_empty() {
        return String::new();
    }
    format!("<div class=\"notice\">\n{}\n</div>\n", lines.join("\n"))
}

/// Under the page: the entries it rests on, by title and date, each with the
/// sections it backs - the dateline's numbered pills, said in full. Then where
/// the page itself is kept.
pub(crate) fn doc_evidence(log: &Log, doc: &DocPage, rel: &str) -> String {
    let mut backs: BTreeMap<u32, Vec<(&str, &str)>> = BTreeMap::new();
    for number in &doc.worklog {
        backs.entry(*number).or_default();
    }
    for section in &doc.sections {
        for number in &section.worklog {
            backs
                .entry(*number)
                .or_default()
                .push((section.heading.as_str(), section.anchor.as_str()));
        }
    }
    let mut out = String::new();
    if !backs.is_empty() {
        out.push_str("<section class=\"doc-evidence\">\n<h2>How this was found out</h2>\n<ul>\n");
        for (number, sections) in &backs {
            let Some(entry) = log.entry(*number) else {
                let _ = writeln!(out, "<li>#{number}, which does not exist</li>");
                continue;
            };
            let under = if sections.is_empty() {
                String::new()
            } else {
                format!(
                    "<span class=\"backs\">{}</span>",
                    sections
                        .iter()
                        .map(|(heading, anchor)| format!(
                            "<a href=\"#{}\">{}</a>",
                            escape(anchor),
                            escape(&plain_md(heading))
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            let corrected = if entry.superseded_by.is_empty() {
                String::new()
            } else {
                format!(
                    "<span class=\"corrected\">corrected by {}</span>",
                    entry
                        .superseded_by
                        .iter()
                        .map(|n| format!("#{n}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            let _ = writeln!(
                out,
                "<li><a href=\"{rel}{}-{}/\"><span class=\"n\">#{number}</span> {}</a>\
                 <span class=\"when\">{}</span>{corrected}{under}</li>",
                entry.number,
                escape(&entry.slug),
                escape(&entry.title),
                short_date(entry.date)
            );
        }
        out.push_str("</ul>\n");
        // Other worklogs' evidence, by name; their titles are in the pills.
        let elsewhere: Vec<String> = doc
            .elsewhere
            .iter()
            .chain(doc.sections.iter().flat_map(|section| &section.elsewhere))
            .map(std::string::ToString::to_string)
            .collect();
        if !elsewhere.is_empty() {
            let _ = writeln!(
                out,
                "<p class=\"elsewhere\">And from other worklogs: {}.</p>",
                escape(&elsewhere.join(", "))
            );
        }
        out.push_str("</section>\n");
    }
    if let Some(repo) = log.project.repository.as_deref() {
        let _ = writeln!(
            out,
            "<p class=\"doc-source\"><a href=\"{}/blob/HEAD/{}\">{}</a></p>",
            escape(repo.trim_end_matches('/')),
            escape(&doc.path),
            escape(&doc.path)
        );
    }
    out
}

/// Where each reference page is linked from, by its path: the pages that link
/// to it, by slug, and the entries, by number. Links are only ever written
/// forwards; this is the way back, worked out once for the whole site.
#[derive(Default)]
pub struct Backlinks(BTreeMap<String, (Vec<String>, Vec<u32>)>);

pub fn backlinks(log: &Log) -> Backlinks {
    let mut found: BTreeMap<String, (Vec<String>, Vec<u32>)> = BTreeMap::new();
    // The page a link leads to, when it leads to one: a file, or a folder,
    // which is its README.
    let page_at = |from_dir: &str, url: &str| -> Option<String> {
        let path = repo_path(from_dir, url)?;
        let target = path.trim_end_matches('/');
        [
            target.to_string(),
            format!("{target}/README.md"),
            format!("{target}/index.md"),
        ]
        .into_iter()
        .find(|candidate| log.doc_at(candidate).is_some())
    };
    for doc in &log.docs {
        // An index page links every page under it; that it does says nothing.
        if doc.is_index {
            continue;
        }
        for url in link_targets(&doc.body) {
            if let Some(target) = page_at(doc_dir(doc), &url)
                && target != doc.path
            {
                let from = &mut found.entry(target).or_default().0;
                if !from.contains(&doc.slug) {
                    from.push(doc.slug.clone());
                }
            }
        }
    }
    for entry in &log.entries {
        let dir = cairns_core::dir_of(&entry.path);
        for url in link_targets(&entry.body) {
            if let Some(target) = page_at(dir, &url) {
                let from = &mut found.entry(target).or_default().1;
                if !from.contains(&entry.number) {
                    from.push(entry.number);
                }
            }
        }
    }
    Backlinks(found)
}

/// One reference page. The dateline is the entry page's dateline.
pub fn doc_page(log: &Log, doc: &DocPage, backlinks: &Backlinks) -> String {
    let depth = doc.slug.matches('/').count() + 2;
    let rel = "../".repeat(depth);

    // The same head an entry has - a label above the title, then a line of
    // pills - so a reference page reads as part of the same site. It had a
    // bare title and a line of plain words, sitting higher than an entry's.
    let label = log.docs_label.as_deref().unwrap_or("Reference");
    let section = (!doc.section.is_empty() && !doc.is_index)
        .then(|| crate::reference::section_title(log, &doc.section));
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
        body.push_str(&evidence_chips(log, &doc.worklog, &doc.elsewhere, &rel));
    }
    // When it last changed: a reference page is kept true by being edited,
    // so its age is part of how far to trust it.
    if let Some(changed) = &doc.changed {
        let shown = changed
            .parse::<cairns_core::Date>()
            .map_or_else(|_| changed.clone(), long_date);
        let _ = write!(
            body,
            "<span class=\"from\">changed <time datetime=\"{}\">{}</time></span>",
            escape(changed),
            escape(&shown)
        );
    }
    body.push_str("</p>\n");
    body.push_str(&doc_notice(log, doc, &rel, backlinks));
    body.push_str(&covers_html(&doc.covers));

    let from_dir = doc_dir(doc);
    let links = Links {
        log,
        from_dir,
        rel: &rel,
        base: None,
    };
    let own = if doc.is_index {
        crate::reference::above_marker(&doc.body)
    } else {
        doc.body.as_str()
    };
    let stripped = cairns_core::entry::without_title(own);
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
            evidence_chips(log, &section.worklog, &section.elsewhere, &rel)
        );
        html.insert_str(end, &line);
    }
    body.push_str(&html);
    body.push_str(&doc_evidence(log, doc, &rel));

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
        Page {
            title: &format!("{} - {}", doc.title, log.project.name),
            description: &doc.title,
            url: &doc.url,
            rel: &rel,
            nav: Nav::Docs,
            aside: &docs_tree(log, &rel, &doc.slug),
            contents: &contents_nav(&headings),
            body: &body,
        },
    )
}
