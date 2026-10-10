//! The reference's own index: `docs/README.md`, generated below a marker.
//!
//! A project's reference index is a table of its pages - title, status, what
//! each covers - and kept by hand it drifts from the pages the way a
//! hand-kept `WORKLOG.md` drifts from the entries. piney_apples wrote a script
//! of its own to stop that. Here the prose above `<!-- cairns:index -->` is
//! the project's and everything below it is generated from the pages' front
//! matter, so `check` can say when it is stale and `index` can put it right.
//!
//! Any `README.md` in the tree may carry the marker: the root's lists every
//! section, a section's lists its own pages.

use cairns_core::Log;
use cairns_core::log::DocPage;
use std::fmt::Write as _;

/// Where the generated listing starts. Everything above it is the project's.
pub const MARKER: &str = "<!-- cairns:index -->";

/// The project's own text of an index page: everything before the marker,
/// or the whole of it when there is none.
pub fn above_marker(body: &str) -> &str {
    match body.find(MARKER) {
        Some(at) => &body[..at],
        None => body,
    }
}

pub fn has_marker(body: &str) -> bool {
    body.contains(MARKER)
}

/// The first folder below `within` that `section` is in: `formats` for
/// `formats/inner` from the root, `formats/inner` from `formats`.
pub(crate) fn child_section(within: &str, section: &str) -> Option<String> {
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

/// The folders directly below `within` that hold pages, in the order the
/// project declared them, then the rest by name.
pub(crate) fn child_sections(log: &Log, within: &str) -> Vec<String> {
    let mut folders: Vec<String> = log
        .docs
        .iter()
        .filter_map(|doc| child_section(within, &doc.section))
        .collect();
    folders.sort();
    folders.dedup();
    let declared = |folder: &String| {
        log.docs_sections
            .iter()
            .position(|section| section.dir.trim_matches('/') == folder)
            .unwrap_or(usize::MAX)
    };
    folders.sort_by_key(|folder| declared(folder));
    folders
}

/// A folder's own page, its `README.md`, when it has one.
pub(crate) fn section_page<'a>(log: &'a Log, folder: &str) -> Option<&'a DocPage> {
    log.docs
        .iter()
        .find(|doc| doc.is_index && doc.slug == folder)
}

/// What a section is called: as declared, else its own page's title, else its
/// folder's name.
pub(crate) fn section_title(log: &Log, folder: &str) -> String {
    if let Some(section) = log
        .docs_sections
        .iter()
        .find(|section| section.dir.trim_matches('/') == folder)
        && !section.title.is_empty()
    {
        return section.title.clone();
    }
    match section_page(log, folder) {
        Some(page) => page.title.clone(),
        None => cairns_core::config::capitalised(folder.rsplit('/').next().unwrap_or(folder)),
    }
}

/// The sentence a section is introduced by, when it is declared with one.
pub(crate) fn section_about(log: &Log, folder: &str) -> Option<String> {
    log.docs_sections
        .iter()
        .find(|section| section.dir.trim_matches('/') == folder)
        .map(|section| section.about.trim().to_string())
        .filter(|about| !about.is_empty())
}

/// The pages in `folder` and every folder below it.
pub(crate) fn pages_under<'a>(log: &'a Log, folder: &str) -> Vec<&'a DocPage> {
    log.docs
        .iter()
        .filter(|doc| {
            !doc.is_index
                && (doc.section == folder || doc.section.starts_with(&format!("{folder}/")))
        })
        .collect()
}

/// A declared field's value on a page, as written. `covers`, which the page
/// keeps parsed, is put back together.
pub fn field_value(doc: &DocPage, name: &str) -> Option<String> {
    if name == "covers" {
        return (!doc.covers.is_empty()).then(|| {
            doc.covers
                .iter()
                .map(|group| group.join(", "))
                .collect::<Vec<_>>()
                .join("; ")
        });
    }
    doc.extra
        .get(name)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// The fields that are columns: declared, shown, and not a key the index
/// already has a column for.
pub(crate) fn columns(log: &Log) -> Vec<&cairns_core::config::DocField> {
    log.docs_fields
        .iter()
        .filter(|field| {
            field.column && !matches!(field.name.as_str(), "title" | "status" | "worklog")
        })
        .collect()
}

/// `readme`, regenerated: its text down to the marker, then the listing. `None`
/// when it has no marker - an index written by hand is the project's to keep.
pub fn regenerate(log: &Log, readme: &DocPage, text: &str) -> Option<String> {
    let at = text.find(MARKER)?;
    let mut out = text[..at + MARKER.len()].to_string();
    out.push_str("\n\n");
    out.push_str(&listing(log, readme));
    Some(out)
}

/// A new root `README.md`, for a tree that has none: a line saying what the
/// reference is, the marker, and the listing.
pub fn fresh(log: &Log, docs_dir: &str) -> String {
    let label = log.docs_label.as_deref().unwrap_or("Reference");
    let to_root = "../".repeat(docs_dir.trim_matches('/').split('/').count());
    let mut out = format!(
        "# {label}\n\nWhat is true now, page by page. The [worklog]({to_root}{}) says how it\n\
         was found out; each page names the entries it rests on.\n\n\
         The listing below is generated from the pages' front matter by `cairns index` -\n\
         edit the pages, and the text above the marker, not the listing.\n\n{MARKER}\n\n",
        log.paths.index
    );
    let root = DocPage {
        slug: String::new(),
        url: String::new(),
        path: format!("{}/README.md", docs_dir.trim_matches('/')),
        title: label.to_string(),
        section: String::new(),
        is_index: true,
        status: None,
        worklog: Vec::new(),
        elsewhere: Vec::new(),
        sections: Vec::new(),
        covers: Vec::new(),
        body: String::new(),
        content_hash: String::new(),
        changed: None,
        extra: Default::default(),
    };
    out.push_str(&listing(log, &root));
    out
}

/// The generated part of an index page: its own pages as a table, then a
/// heading and a table for each folder below it.
fn listing(log: &Log, readme: &DocPage) -> String {
    let within = readme.slug.as_str();
    let dir = readme
        .path
        .rsplit_once('/')
        .map(|(dir, _)| dir)
        .unwrap_or("");
    let mut out = String::new();

    let here: Vec<&DocPage> = log
        .docs
        .iter()
        .filter(|doc| !doc.is_index && doc.section == within)
        .collect();
    out.push_str(&table(log, &here, dir));

    let folders = child_sections(log, within);
    for folder in &folders {
        let pages = pages_under(log, folder);
        if pages.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        let title = escape_heading(&section_title(log, folder));
        match section_page(log, folder) {
            Some(page) => {
                let _ = writeln!(out, "## [{title}]({})\n", relative(dir, &page.path));
            }
            None => {
                let _ = writeln!(out, "## {title}\n");
            }
        }
        if let Some(about) = section_about(log, folder) {
            let _ = writeln!(out, "{about}\n");
        }
        out.push_str(&table(log, &pages, dir));
    }
    if out.is_empty() {
        out.push_str("No pages yet.\n");
    }
    out
}

fn table(log: &Log, pages: &[&DocPage], dir: &str) -> String {
    if pages.is_empty() {
        return String::new();
    }
    let columns = columns(log);
    let mut out = String::from("| Page | Status |");
    for field in &columns {
        let _ = write!(out, " {} |", cell(&field.label()));
    }
    out.push_str(" Worklog |\n| --- | --- |");
    for _ in &columns {
        out.push_str(" --- |");
    }
    out.push_str(" --- |\n");
    for doc in pages {
        let _ = write!(
            out,
            "| [{}]({}) | {} |",
            cell(&doc.title),
            relative(dir, &doc.path),
            doc.status.map(|status| status.name()).unwrap_or("-")
        );
        for field in &columns {
            let _ = write!(
                out,
                " {} |",
                cell(&field_value(doc, &field.name).unwrap_or_else(|| "-".into()))
            );
        }
        let mut cited: Vec<String> = doc
            .worklog
            .iter()
            .map(|number| match log.entry(*number) {
                Some(entry) => format!("[{number}]({})", relative(dir, &entry.path)),
                None => number.to_string(),
            })
            .collect();
        cited.extend(doc.elsewhere.iter().map(|other| format!("`{other}`")));
        let _ = writeln!(
            out,
            " {} |",
            if cited.is_empty() {
                "-".to_string()
            } else {
                cited.join(", ")
            }
        );
    }
    out
}

/// `path` - from the repository root - as a link from a file in `dir`.
fn relative(dir: &str, path: &str) -> String {
    let from: Vec<&str> = dir.split('/').filter(|part| !part.is_empty()).collect();
    let to: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    let shared = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut out = "../".repeat(from.len() - shared);
    out.push_str(&to[shared..].join("/"));
    out
}

fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

fn escape_heading(text: &str) -> String {
    text.replace('[', "\\[").replace(']', "\\]")
}

#[cfg(test)]
mod tests {
    use super::{child_section, relative};

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

    #[test]
    fn a_link_is_written_from_where_the_index_is() {
        assert_eq!(relative("docs", "docs/formats/a.md"), "formats/a.md");
        assert_eq!(
            relative("docs", "worklog/0003-x.md"),
            "../worklog/0003-x.md"
        );
        assert_eq!(relative("docs/formats", "docs/formats/a.md"), "a.md");
        assert_eq!(
            relative("docs/formats", "docs/engine/b.md"),
            "../engine/b.md"
        );
        assert_eq!(relative("", "worklog/0003-x.md"), "worklog/0003-x.md");
    }
}
