//! Pieces more than one page shows: dates, evidence, code references, files.

use super::*;
use std::fmt::Write as _;

pub(crate) fn path_of(entry: &LogEntry) -> String {
    format!("{}-{}", entry.number, entry.slug)
}

/// The time a run of entries took, summed, or `None` if none of them say.
/// Entries that do not say are not counted as zero - they are not counted.
pub(crate) fn total_took<'a>(entries: impl Iterator<Item = &'a LogEntry>) -> Option<String> {
    let minutes: Vec<u32> = entries.filter_map(|entry| entry.took_minutes).collect();
    (!minutes.is_empty()).then(|| cairns_core::entry::format_duration(minutes.iter().sum()))
}

/// Days since an arbitrary epoch, for spacing dates apart. Howard Hinnant's
/// `days_from_civil`, the inverse of what `Date` already carries.
pub(crate) fn day_number(date: cairns_core::Date) -> i64 {
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
pub(crate) fn short_date(date: cairns_core::Date) -> String {
    let long = long_date(date);
    let mut parts = long.splitn(3, ' ');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(day), Some(month), Some(year)) => {
            format!("{day} {} {year}", month.get(..3).unwrap_or(month))
        }
        _ => long,
    }
}

/// `2 October 2026`. Unambiguous in every locale, which `10/02` is not.
pub(crate) fn long_date(date: cairns_core::Date) -> String {
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

/// Evidence as pills: this log's entries as `#12`, other worklogs' as
/// `piney:361`, each linking to its entry with its title on hover.
pub(crate) fn evidence_chips(
    log: &Log,
    here: &[u32],
    elsewhere: &[cairns_core::entry::EntryRef],
    rel: &str,
) -> String {
    let mut out = String::new();
    for number in here {
        match log.entry(*number) {
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
pub(crate) fn elsewhere_title(log: &Log, workspace: &str, number: u32) -> String {
    match log
        .workspaces
        .get(workspace)
        .and_then(|w| w.entries.get(&number))
    {
        Some(entry) => format!("{workspace} {number}: {}", entry.title),
        None => format!("entry {number} of {workspace}"),
    }
}

/// Where an entry in another worklog is, and what to call it on hover: its
/// page when the worklog was read, its number's short URL when it is known
/// only by where it is published. `None` when it is known not at all.
pub(crate) fn elsewhere_link(log: &Log, workspace: &str, number: u32) -> Option<(String, String)> {
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
pub(crate) fn code_label(code: &cairns_core::entry::CodeRef) -> String {
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
pub(crate) fn code_url(log: &Log, code: &cairns_core::log::ResolvedCode) -> Option<String> {
    let repo = log.project.repository.as_deref()?.trim_end_matches('/');
    let mut url = format!(
        "{repo}/blob/{}/{}",
        code.rev.as_deref().unwrap_or("HEAD"),
        code.path
    );
    if let Some((start, end)) = code.lines {
        let _ = write!(url, "#L{start}");
        if start != end {
            let _ = write!(url, "-L{end}");
        }
    }
    Some(url)
}

/// A code reference as HTML: a link to its lines, or - `![[...]]` - the code
/// itself, highlighted, under a caption linking to where it lives. One that
/// could not be found says why, rather than linking somewhere that is not.
pub(crate) fn code_reference_html(log: &Log, code: &cairns_core::entry::CodeRef) -> String {
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
pub(crate) const FILES_SHOWN: usize = 5;

/// An entry's files, grouped under their directories.
///
/// Printed flat, fifteen paths were a wall between the title and the first
/// sentence, and most of it was the same `crates/x/src/` over and over. The
/// directory is said once and the names follow it, in the order written. Past
/// [`FILES_SHOWN`] the list is still the entry's link to the code, but not the
/// thing a reader came for, so it starts closed.
pub(crate) fn files_html(files: &[String], links: &Links<'_>) -> String {
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

/// Where the repository's own files mention this entry - "worklog 50" in a
/// comment - shown the way its files are, each linking to the line.
pub(crate) fn mentions_html(log: &Log, number: u32) -> String {
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
pub(crate) fn covers_html(groups: &[Vec<String>]) -> String {
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

/// The search field a list page opens its toolbar with: a magnifier and the
/// box `search.js` reads, saying what it searches.
pub(crate) fn search_field(placeholder: &str) -> String {
    format!(
        "<div class=\"search\">\n<svg viewBox=\"0 0 16 16\" aria-hidden=\"true\" focusable=\"false\">\
         <circle cx=\"7\" cy=\"7\" r=\"4.5\" fill=\"none\" stroke=\"currentColor\" \
         stroke-width=\"1.5\"/><path d=\"M10.5 10.5 L14 14\" stroke=\"currentColor\" \
         stroke-width=\"1.5\" stroke-linecap=\"round\"/></svg>\n\
         <input type=\"search\" id=\"q\" placeholder=\"{}\" \
         autocomplete=\"off\" spellcheck=\"false\">\n</div>\n",
        escape(placeholder)
    )
}

/// Entries named in the trailer: `No. 12 - Title` in a sentence, `#12` beside
/// a question.
pub(crate) fn numbered(log: &Log, numbers: &[u32], long: bool) -> String {
    numbers
        .iter()
        .map(|number| match log.entry(*number) {
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
}
