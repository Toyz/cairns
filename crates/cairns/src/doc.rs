//! `cairns doc`: the reference pages, from the command line.
//!
//! The same reason `cairns new` takes its prose on stdin: whoever writes a page
//! is usually a model, and a page made in one command - front matter, heading
//! and body - is a page whose front matter is right. Editing the evidence list
//! by hand is the step that goes wrong, so `cite` does it.

use cairns_core::{Config, Doc, Status};
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn docs_dir(config: &Config) -> Result<&str> {
    config
        .docs
        .as_ref()
        .map(|docs| docs.dir.as_str())
        .ok_or_else(|| {
            "this project keeps no reference - add a [docs] section to cairns.toml:\n    \
             [docs]\n    dir = \"docs\""
                .into()
        })
}

/// The index pages under the docs root that carry the marker, each as it is
/// on disk and as it should be: `(path, found, wanted)`.
pub fn indexes(root: &Path, log: &cairns_core::Log) -> Vec<(String, String, String)> {
    log.docs
        .iter()
        .filter(|doc| doc.is_index)
        .filter_map(|doc| {
            let found = std::fs::read_to_string(root.join(&doc.path)).ok()?;
            let wanted = cairns_site::reference::regenerate(log, doc, &found)?;
            Some((doc.path.clone(), found, wanted))
        })
        .collect()
}

/// Why an index page is stale, in terms of its pages: rows keyed by the page
/// they link, so a page whose status changed is one change.
pub fn stale_because(found: &str, wanted: &str) -> String {
    let rows = |text: &str| -> std::collections::BTreeMap<String, String> {
        let listing = text
            .find(cairns_site::reference::MARKER)
            .map(|at| &text[at..])
            .unwrap_or("");
        listing
            .lines()
            .filter(|line| line.starts_with("| ["))
            .filter_map(|row| {
                let link = row.split("](").nth(1)?.split(')').next()?;
                Some((link.to_string(), row.to_string()))
            })
            .collect()
    };
    let (have, want) = (rows(found), rows(wanted));
    let added = want.keys().filter(|page| !have.contains_key(*page)).count();
    let gone = have.keys().filter(|page| !want.contains_key(*page)).count();
    let changed = want
        .iter()
        .filter(|(page, row)| have.get(*page).is_some_and(|old| old != *row))
        .count();
    let pages = |n: usize| {
        if n == 1 {
            "1 page".to_string()
        } else {
            format!("{n} pages")
        }
    };
    let mut why = Vec::new();
    if added > 0 {
        why.push(format!("{} not listed yet", pages(added)));
    }
    if changed > 0 {
        why.push(format!("{} not as it lists them", pages(changed)));
    }
    if gone > 0 {
        why.push(format!("{} listed that are gone", pages(gone)));
    }
    if why.is_empty() {
        return "its listing differs from the one this version writes".into();
    }
    why.join(", ")
}

/// Regenerate every index page that carries the marker, and - when `create`
/// - write the docs root's `README.md` if it has none. The paths written.
pub fn write_indexes(
    root: &Path,
    config: &Config,
    log: &cairns_core::Log,
    create: bool,
) -> Result<Vec<String>> {
    let mut written = Vec::new();
    for (path, found, wanted) in indexes(root, log) {
        if found != wanted {
            std::fs::write(root.join(&path), wanted)?;
            written.push(path);
        }
    }
    if create
        && let Some(docs) = &config.docs
        && !log
            .docs
            .iter()
            .any(|doc| doc.is_index && doc.slug.is_empty())
    {
        let dir = docs.dir.trim_matches('/');
        let path = format!("{dir}/README.md");
        std::fs::create_dir_all(root.join(dir))?;
        std::fs::write(root.join(&path), cairns_site::reference::fresh(log, dir))?;
        written.push(path);
    }
    Ok(written)
}

/// What `check` holds the reference to, beyond the evidence existing: the
/// fields the project declared, a status that is one of the three, folders the
/// project declared, and links that lead somewhere.
pub fn problems(root: &Path, config: &Config, docs: &[Doc]) -> Vec<String> {
    let Some(settings) = &config.docs else {
        return Vec::new();
    };
    let dir = settings.dir.trim_matches('/');
    let mut problems = Vec::new();
    let mut undeclared: Vec<String> = Vec::new();
    for doc in docs {
        if let Some(status) = doc.extra.get("status") {
            problems.push(format!(
                "{}: status {status:?} is not one of solid, partial, guess",
                doc.path
            ));
        }
        let section = doc.section(dir);
        if !doc.is_index()
            && !settings.sections.is_empty()
            && !section.is_empty()
            && !settings.sections.iter().any(|declared| {
                let declared = declared.dir.trim_matches('/');
                section == declared || section.starts_with(&format!("{declared}/"))
            })
        {
            let top = section.split('/').next().unwrap_or(&section).to_string();
            if !undeclared.contains(&top) {
                problems.push(format!(
                    "{}: is in {dir}/{top}/, which is not a [[docs.section]] in cairns.toml - \
                     declare it, so the index gives it a title and a place",
                    doc.path
                ));
                undeclared.push(top);
            }
        }
        for field in &settings.fields {
            if doc.is_index() {
                break;
            }
            let value = match field.name.as_str() {
                "title" => Some(doc.title.clone()),
                // One that is there but not a status was reported above.
                "status" => doc
                    .status
                    .map(|status| status.name().to_string())
                    .or_else(|| doc.extra.get("status").cloned()),
                "worklog" => (!doc.worklog.is_empty() || !doc.elsewhere.is_empty())
                    .then(|| "cited".to_string()),
                "covers" => (!doc.covers.is_empty()).then(|| "covered".to_string()),
                name => doc
                    .extra
                    .get(name)
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
            };
            let Some(value) = value else {
                if field.required {
                    problems.push(format!("{}: has no `{}`", doc.path, field.name));
                }
                continue;
            };
            if field.values.is_empty()
                || matches!(
                    field.name.as_str(),
                    "title" | "status" | "worklog" | "covers"
                )
            {
                continue;
            }
            for item in value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
            {
                if !field.values.iter().any(|allowed| allowed == item) {
                    problems.push(format!(
                        "{}: {} has {item:?}, which is not one of {}",
                        doc.path,
                        field.name,
                        field.values.join(", ")
                    ));
                }
            }
        }
        // A dead link in a reference page sends the reader looking for a page
        // that was renamed or never written. Only pages: an entry is never
        // edited, so a link in one that has since gone stale stays as written.
        let from = doc.path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
        for target in cairns_site::html::link_targets(&doc.body) {
            let Some(path) = local_target(from, &target) else {
                continue;
            };
            if !root.join(&path).exists() {
                problems.push(format!(
                    "{}: links to {target}, which does not exist",
                    doc.path
                ));
            }
        }
    }
    problems
}

/// A link's target as a path from the repository root, when it is a file in
/// the repository at all: not a URL, an anchor, or a path from the site root.
fn local_target(from: &str, target: &str) -> Option<String> {
    let outside = target.is_empty()
        || target.starts_with('#')
        || target.starts_with('/')
        || target.contains("://")
        || target.starts_with("mailto:")
        || target.starts_with("cairns:");
    if outside {
        return None;
    }
    let path = target
        .split(['#', '?'])
        .next()
        .unwrap_or("")
        .replace("%20", " ");
    if path.is_empty() {
        return None;
    }
    let mut parts: Vec<&str> = from.split('/').filter(|part| !part.is_empty()).collect();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                // Above the repository is not somewhere a page can link to.
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

/// Write a new page and print its path.
/// What `cairns doc new` was given.
pub struct NewPage<'a> {
    pub title: &'a str,
    /// The section, a folder under the docs root.
    pub section: Option<&'a str>,
    pub status: Option<&'a str>,
    /// The entries that established it.
    pub from: &'a [u32],
    /// Other front matter, `key=value` each.
    pub set: &'a [String],
    pub body: Option<String>,
}

pub fn new(root: &Path, config: &Config, page: NewPage<'_>) -> Result<()> {
    let NewPage {
        title,
        section,
        status,
        from,
        set,
        body,
    } = page;
    let dir = docs_dir(config)?;
    // The project's own front matter, `key=value` each - written as given and
    // held to `cairns.toml` by `check`, but a value it does not allow is
    // refused here, before there is a page with it in.
    let mut fields: Vec<(String, String)> = Vec::new();
    for pair in set {
        let Some((key, value)) = pair.split_once('=') else {
            return Err(format!("--set {pair:?} is not key=value").into());
        };
        let (key, value) = (key.trim(), value.trim());
        if matches!(key, "title" | "status" | "worklog") {
            return Err(format!("`{key}` has its own option").into());
        }
        if key.is_empty()
            || !key
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(
                format!("`{key}` is not a front matter key - lower-case, no spaces").into(),
            );
        }
        if let Some(field) = config
            .docs
            .as_ref()
            .and_then(|docs| docs.fields.iter().find(|field| field.name == key))
            && !field.values.is_empty()
        {
            for item in value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
            {
                if !field.values.iter().any(|allowed| allowed == item) {
                    return Err(format!(
                        "{key} cannot be {item:?} - it is one of {}",
                        field.values.join(", ")
                    )
                    .into());
                }
            }
        }
        fields.push((key.to_string(), value.to_string()));
    }
    let title = title.trim();
    if title.is_empty() {
        return Err("a page needs a title".into());
    }
    let status = match status {
        Some(text) => Some(Status::parse(text).ok_or_else(|| {
            format!("unknown status {text:?}; a page is solid, partial or guess")
        })?),
        None => None,
    };
    check_entries_exist(root, config, from)?;

    let section = section
        .map(|section| section.trim().trim_matches('/'))
        .filter(|section| !section.is_empty());
    let mut path = root.join(dir);
    if let Some(section) = section {
        path = path.join(section);
    }
    let path = path.join(format!("{}.md", cairns_core::entry::slugify(title)));
    if path.exists() {
        return Err(format!(
            "{} already exists - edit it, and `cairns doc cite` the entry that changed it",
            path.display()
        )
        .into());
    }

    let mut front = format!("---\ntitle: {title}\n");
    if let Some(status) = status {
        front.push_str(&format!("status: {}\n", status.name()));
    }
    if !from.is_empty() {
        front.push_str(&format!("worklog: {}\n", numbers(from)));
    }
    for (key, value) in &fields {
        front.push_str(&format!("{key}: {value}\n"));
    }
    front.push_str("---\n\n");

    let body = body.map(|body| {
        let body = body.trim();
        // The command writes the heading; a body that brings its own would
        // give the page two.
        match body.strip_prefix("# ") {
            Some(rest) => rest
                .split_once('\n')
                .map(|(_, rest)| rest.trim())
                .unwrap_or(""),
            None => body,
        }
        .to_string()
    });
    let text = match body.as_deref() {
        Some(body) if !body.is_empty() => format!("{front}# {title}\n\n{body}\n"),
        _ => format!("{front}# {title}\n\n"),
    };

    std::fs::create_dir_all(path.parent().expect("a page has a directory"))?;
    std::fs::write(&path, text)?;
    println!("{}", path.display());
    Ok(())
}

/// Add entries to a page's evidence, keeping the ones it has - in its front
/// matter, or under one of its headings with `section`. An entry is a number
/// here, or `name:12` in another worklog.
pub fn cite(
    root: &Path,
    config: &Config,
    page: &str,
    cited: &[String],
    section: Option<&str>,
) -> Result<()> {
    let dir = docs_dir(config)?;
    let mut here = Vec::new();
    for item in cited {
        let item = item.trim();
        if let Ok(number) = item.parse::<u32>() {
            here.push(number);
        } else if cairns_core::entry::parse_entry_ref(item).is_none() {
            return Err(format!(
                "{item:?} is not an entry number, or one in another worklog (`name:12`)"
            )
            .into());
        }
    }
    check_entries_exist(root, config, &here)?;
    let cited: Vec<String> = cited.iter().map(|c| c.trim().to_string()).collect();
    let path = find_page(root, config, dir, page)?;
    let text = std::fs::read_to_string(&path)?;

    let (updated, all) = match section {
        Some(heading) => with_section_citations(&text, heading, &cited)?,
        None => with_citations(&text, &cited, &path)?,
    };
    std::fs::write(&path, updated)?;
    println!(
        "{}{} cites {}",
        path.display(),
        section
            .map(|h| format!(" under \"{h}\""))
            .unwrap_or_default(),
        all.join(", ")
    );
    Ok(())
}

/// Items already in a list, then the new ones not among them.
fn merged(existing: &str, cited: &[String]) -> Vec<String> {
    let mut all: Vec<String> = existing
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect();
    for item in cited {
        if !all.contains(item) {
            all.push(item.clone());
        }
    }
    all
}

/// The page's text with `cited` added to its `worklog:` line, and the full list
/// it ends up with. A page with no front matter gets some.
fn with_citations(text: &str, cited: &[String], path: &Path) -> Result<(String, Vec<String>)> {
    let Some(rest) = text.strip_prefix("---\n") else {
        let title = text
            .lines()
            .find_map(|line| line.strip_prefix("# "))
            .map(str::trim)
            .unwrap_or("")
            .to_string();
        let mut front = String::from("---\n");
        if !title.is_empty() {
            front.push_str(&format!("title: {title}\n"));
        }
        let all = merged("", cited);
        front.push_str(&format!("worklog: {}\n---\n\n", all.join(", ")));
        return Ok((format!("{front}{text}"), all));
    };
    let end = rest
        .find("\n---")
        .ok_or_else(|| format!("{}: front matter is never closed", path.display()))?;
    let (front, after) = rest.split_at(end);

    let mut all = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    let mut found = false;
    for line in front.lines() {
        match line.split_once(':') {
            Some((key, value)) if key.trim().eq_ignore_ascii_case("worklog") => {
                found = true;
                all = merged(value, cited);
                lines.push(format!("worklog: {}", all.join(", ")));
            }
            _ => lines.push(line.to_string()),
        }
    }
    if !found {
        all = merged("", cited);
        lines.push(format!("worklog: {}", all.join(", ")));
    }
    Ok((format!("---\n{}{after}", lines.join("\n")), all))
}

/// A `##` or `###` heading's text, or `None` when the line is not one.
fn heading_of(line: &str) -> Option<&str> {
    line.strip_prefix("### ")
        .or_else(|| line.strip_prefix("## "))
        .map(str::trim)
}

/// The page's text with `cited` added under one of its `##` or `###`
/// headings, in the `<!-- worklog: ... -->` comment there - written when there
/// is none.
fn with_section_citations(
    text: &str,
    heading: &str,
    cited: &[String],
) -> Result<(String, Vec<String>)> {
    let lines: Vec<&str> = text.lines().collect();

    let Some(at) = lines
        .iter()
        .position(|line| heading_of(line).is_some_and(|h| h.eq_ignore_ascii_case(heading.trim())))
    else {
        let known: Vec<&str> = lines.iter().filter_map(|line| heading_of(line)).collect();
        return Err(format!(
            "no section {heading:?} - the page's sections are: {}",
            if known.is_empty() {
                "none".to_string()
            } else {
                known.join(", ")
            }
        )
        .into());
    };
    let mut out: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
    let section_end = lines[at + 1..]
        .iter()
        .position(|line| line.starts_with('#'))
        .map(|offset| at + 1 + offset)
        .unwrap_or(lines.len());
    let comment = (at + 1..section_end).find(|&i| {
        let line = lines[i].trim();
        line.starts_with("<!--") && line.ends_with("-->") && line.contains("worklog:")
    });
    let all = match comment {
        Some(i) => {
            let inner = lines[i]
                .trim()
                .trim_start_matches("<!--")
                .trim_end_matches("-->");
            let existing = inner.trim().trim_start_matches("worklog:");
            let all = merged(existing, cited);
            out[i] = format!("<!-- worklog: {} -->", all.join(", "));
            all
        }
        None => {
            let all = merged("", cited);
            out.insert(at + 1, format!("<!-- worklog: {} -->", all.join(", ")));
            all
        }
    };
    let mut text_out = out.join("\n");
    if text.ends_with('\n') {
        text_out.push('\n');
    }
    Ok((text_out, all))
}

/// Every page, how sure it is, what it rests on, and what needs looking at.
/// Returns how many pages were flagged, so `--strict` can fail on them.
pub fn list(root: &Path, config: &Config) -> Result<usize> {
    let dir = docs_dir(config)?;
    let entries = crate::read_entries(root, config)?;
    let docs = crate::read_docs(root, config)?;
    let built = cairns_core::Log::build_with(config, entries, docs.clone(), None);
    let corrected = |number: u32| {
        built
            .entries
            .iter()
            .find(|entry| entry.number == number)
            .map(|entry| entry.superseded_by.clone())
            .unwrap_or_default()
    };

    let mut pages: Vec<&Doc> = docs.iter().filter(|doc| !doc.is_index()).collect();
    pages.sort_by_key(|doc| doc.slug(dir));
    if pages.is_empty() {
        println!("no pages under {dir}/ - `cairns doc new` writes one");
        return Ok(0);
    }

    let width = pages
        .iter()
        .map(|doc| doc.slug(dir).len())
        .max()
        .unwrap_or(0);
    let mut flagged = 0;
    for doc in &pages {
        let status = doc.status.map(Status::name).unwrap_or("-");
        let cites = match doc.worklog.len() {
            0 => String::from("-"),
            _ => numbers(&doc.worklog),
        };
        println!("{status:<8} {:<width$}  {cites}", doc.slug(dir));

        let mut notes = Vec::new();
        if doc.worklog.is_empty() {
            notes.push("cites no entry - where was this found out?".to_string());
        }
        // A page resting on a claim a later entry overturned may still say the
        // old thing. Nothing else would catch it: both files are valid.
        // Unless the page also cites what corrected it: then it was most
        // likely brought up to date at the time.
        for number in &doc.worklog {
            let later: Vec<u32> = corrected(*number)
                .into_iter()
                .filter(|n| !doc.worklog.contains(n))
                .collect();
            if !later.is_empty() {
                notes.push(format!(
                    "rests on #{number}, which #{} corrected - does the page still say the old thing?",
                    numbers(&later).replace(", ", ", #")
                ));
            }
        }
        if doc.status.is_none() {
            notes.push("no status - solid, partial or guess?".to_string());
        }
        // A page is the current truth, so code it names that is no longer
        // there means the page is out of date - unlike an entry, which is
        // allowed to have pointed at where code used to be.
        for code in cairns_core::entry::code_references(&doc.body) {
            if let Some(why) = crate::code::resolve(root, &code, false).missing {
                notes.push(format!("{} - {why}", code.key()));
            }
        }
        if !notes.is_empty() {
            flagged += 1;
        }
        for note in notes {
            println!("{:8} {:width$}  ! {note}", "", "");
        }
    }

    let count = |status: Status| {
        pages
            .iter()
            .filter(|doc| doc.status == Some(status))
            .count()
    };
    println!(
        "\n{} pages: {} solid, {} partial, {} guess; {flagged} to look at",
        pages.len(),
        count(Status::Solid),
        count(Status::Partial),
        count(Status::Guess)
    );
    Ok(flagged)
}

/// A page named by its slug (`formats/pod`), its path under the docs root
/// (`formats/pod.md`), or its path from the repository root.
fn find_page(root: &Path, config: &Config, dir: &str, page: &str) -> Result<PathBuf> {
    let wanted = page.trim().trim_matches('/');
    let docs = crate::read_docs(root, config)?;
    let found = docs.iter().find(|doc| {
        let slug = doc.slug(dir);
        slug == wanted
            || doc.path == wanted
            || doc
                .path
                .strip_prefix(dir)
                .map(|p| p.trim_start_matches('/'))
                == Some(wanted)
            || wanted.strip_suffix(".md").is_some_and(|stem| slug == stem)
    });
    match found {
        Some(doc) => Ok(root.join(&doc.path)),
        None => {
            let mut close: Vec<String> = docs
                .iter()
                .filter(|doc| !doc.is_index())
                .map(|doc| doc.slug(dir))
                .filter(|slug| {
                    let tail = wanted.rsplit('/').next().unwrap_or(wanted);
                    slug.contains(tail)
                })
                .collect();
            close.truncate(5);
            Err(if close.is_empty() {
                format!("no page {wanted:?} under {dir}/ - `cairns doc list` shows them all").into()
            } else {
                format!("no page {wanted:?}; did you mean {}?", close.join(", ")).into()
            })
        }
    }
}

fn check_entries_exist(root: &Path, config: &Config, wanted: &[u32]) -> Result<()> {
    if wanted.is_empty() {
        return Ok(());
    }
    let entries = crate::read_entries(root, config)?;
    let missing: Vec<u32> = wanted
        .iter()
        .copied()
        .filter(|n| !entries.iter().any(|entry| entry.front.number == *n))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("no entry {} in this log", numbers(&missing)).into())
    }
}

fn numbers(values: &[u32]) -> String {
    values
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod link_tests {
    use super::{local_target, problems};

    #[test]
    fn check_holds_pages_to_what_the_project_declared() {
        let config = cairns_core::Config::parse(
            "spec_version = 1\n[project]\nname = \"P\"\nslug = \"p\"\n[site]\nbase_url = \"\"\n\
             [docs]\ndir = \"docs\"\n[[docs.section]]\ndir = \"formats\"\n\
             [[docs.field]]\nname = \"status\"\nrequired = true\n\
             [[docs.field]]\nname = \"volumes\"\nvalues = [\"INF\", \"all\"]\nrequired = true\n\
             [[area]]\nname = \"spec\"\n",
        )
        .unwrap();
        let root = std::env::temp_dir().join(format!("cairns-doc-problems-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("docs/formats")).unwrap();
        std::fs::write(root.join("docs/formats/there.md"), "").unwrap();
        let doc = |path: &str, front: &str, body: &str| {
            cairns_core::Doc::parse(&cairns_core::RawEntry {
                path: path.into(),
                bytes: format!("---\ntitle: T\n{front}---\n\n# T\n\n{body}\n").into_bytes(),
            })
            .unwrap()
        };
        let docs = [
            doc(
                "docs/formats/good.md",
                "status: solid\nvolumes: INF, all\n",
                "[ok](there.md) `[no](code.md)`",
            ),
            doc(
                "docs/formats/bad.md",
                "status: solidish\nvolumes: PS3\n",
                "[gone](gone.md#x) [web](https://x.example/y.md)",
            ),
            doc("docs/formats/bare.md", "", ""),
            doc("docs/extras/x.md", "status: guess\nvolumes: all\n", ""),
        ];
        let found = problems(&root, &config, &docs);
        let _ = std::fs::remove_dir_all(&root);
        let has = |text: &str| found.iter().any(|problem| problem.contains(text));
        assert!(
            !found.iter().any(|p| p.starts_with("docs/formats/good.md")),
            "{found:#?}"
        );
        assert!(
            has("bad.md: status \"solidish\" is not one of"),
            "{found:#?}"
        );
        assert!(
            !has("bad.md: has no `status`"),
            "reported twice: {found:#?}"
        );
        assert!(has("bad.md: volumes has \"PS3\""), "{found:#?}");
        assert!(
            has("bad.md: links to gone.md#x, which does not exist"),
            "{found:#?}"
        );
        assert!(!has("x.example"), "{found:#?}");
        assert!(
            has("bare.md: has no `status`") && has("bare.md: has no `volumes`"),
            "{found:#?}"
        );
        assert!(
            has("x.md: is in docs/extras/, which is not a [[docs.section]]"),
            "{found:#?}"
        );
    }

    #[test]
    fn a_link_is_found_from_the_page_it_is_in() {
        assert_eq!(
            local_target("docs/formats", "ccs.md#layout").as_deref(),
            Some("docs/formats/ccs.md")
        );
        assert_eq!(
            local_target("docs/formats", "../../WORKLOG.md").as_deref(),
            Some("WORKLOG.md")
        );
        assert_eq!(
            local_target("docs", "./a%20b.md").as_deref(),
            Some("docs/a b.md")
        );
        assert_eq!(local_target("docs", "https://example.com/x.md"), None);
        assert_eq!(local_target("docs", "#here"), None);
        assert_eq!(local_target("docs", "../../outside.md"), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cited(text: &str, add: &[u32]) -> String {
        let add: Vec<String> = add.iter().map(u32::to_string).collect();
        with_citations(text, &add, Path::new("docs/x.md"))
            .unwrap()
            .0
    }

    #[test]
    fn a_section_is_cited_under_its_heading() {
        let text = "---\ntitle: X\n---\n\n# X\n\n## Layout\n\nBody.\n\n## Index\n<!-- worklog: 4 -->\nMore.\n";
        let cite = |text: &str, heading: &str, add: &[&str]| {
            let add: Vec<String> = add.iter().map(|s| s.to_string()).collect();
            with_section_citations(text, heading, &add)
        };
        let (once, _) = cite(text, "layout", &["12", "piney:3"]).unwrap();
        assert!(
            once.contains("## Layout\n<!-- worklog: 12, piney:3 -->\n"),
            "{once}"
        );
        let (twice, all) = cite(&once, "Index", &["4", "9"]).unwrap();
        assert!(
            twice.contains("## Index\n<!-- worklog: 4, 9 -->\nMore."),
            "{twice}"
        );
        assert_eq!(all, vec!["4", "9"]);
        assert!(
            cite(text, "Nope", &["1"])
                .unwrap_err()
                .to_string()
                .contains("Layout, Index")
        );
    }

    #[test]
    fn citing_adds_to_the_list_without_repeating() {
        let text = "---\ntitle: X\nworklog: 12, 15\n---\n\n# X\n\nBody.\n";
        assert_eq!(
            cited(text, &[15, 31]),
            "---\ntitle: X\nworklog: 12, 15, 31\n---\n\n# X\n\nBody.\n"
        );
    }

    #[test]
    fn citing_a_page_with_no_list_gives_it_one() {
        let text = "---\ntitle: X\nstatus: guess\n---\n\n# X\n";
        assert_eq!(
            cited(text, &[3]),
            "---\ntitle: X\nstatus: guess\nworklog: 3\n---\n\n# X\n"
        );
    }

    #[test]
    fn citing_a_page_with_no_front_matter_gives_it_some() {
        assert_eq!(
            cited("# The archive\n\nBody.\n", &[7]),
            "---\ntitle: The archive\nworklog: 7\n---\n\n# The archive\n\nBody.\n"
        );
    }

    #[test]
    fn a_cited_page_still_parses() {
        let text = cited("---\ntitle: X\nworklog: 1\n---\n\n# X\n", &[2]);
        let doc = Doc::parse(&cairns_core::RawEntry {
            path: "docs/x.md".into(),
            bytes: text.into_bytes(),
        })
        .unwrap();
        assert_eq!(doc.worklog, vec![1, 2]);
    }
}
