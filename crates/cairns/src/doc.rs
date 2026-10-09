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

/// Write a new page and print its path.
pub fn new(
    root: &Path,
    config: &Config,
    title: &str,
    section: Option<&str>,
    status: Option<&str>,
    from: &[u32],
    body: Option<String>,
) -> Result<()> {
    let dir = docs_dir(config)?;
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

/// Add entries to a page's evidence, keeping the ones it has.
pub fn cite(root: &Path, config: &Config, page: &str, cited: &[u32]) -> Result<()> {
    let dir = docs_dir(config)?;
    check_entries_exist(root, config, cited)?;
    let path = find_page(root, config, dir, page)?;
    let text = std::fs::read_to_string(&path)?;

    let (updated, all) = with_citations(&text, cited, &path)?;
    std::fs::write(&path, updated)?;
    println!("{} cites {}", path.display(), numbers(&all));
    Ok(())
}

/// The page's text with `cited` added to its `worklog:` line, and the full list
/// it ends up with. A page with no front matter gets some.
fn with_citations(text: &str, cited: &[u32], path: &Path) -> Result<(String, Vec<u32>)> {
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
        front.push_str(&format!("worklog: {}\n---\n\n", numbers(cited)));
        return Ok((format!("{front}{text}"), dedup(cited.to_vec())));
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
                for part in value.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                    all.push(part.parse::<u32>().map_err(|_| {
                        format!(
                            "{}: worklog has {part:?}, not an entry number",
                            path.display()
                        )
                    })?);
                }
                all.extend_from_slice(cited);
                all = dedup(all);
                lines.push(format!("worklog: {}", numbers(&all)));
            }
            _ => lines.push(line.to_string()),
        }
    }
    if !found {
        all = dedup(cited.to_vec());
        lines.push(format!("worklog: {}", numbers(&all)));
    }
    Ok((format!("---\n{}{after}", lines.join("\n")), all))
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

/// In the order given, each once.
fn dedup(values: Vec<u32>) -> Vec<u32> {
    let mut out = Vec::new();
    for value in values {
        if !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cited(text: &str, add: &[u32]) -> String {
        with_citations(text, add, Path::new("docs/x.md")).unwrap().0
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
