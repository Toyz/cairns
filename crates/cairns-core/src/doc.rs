//! Reference pages: the other half of a worklog.
//!
//! The log says how something was found out; a doc page says what is true,
//! flatly, for someone who only wants to use it. They are different documents
//! with different jobs, and the useful thing is the edge between them - a doc
//! page names the entries that established it, so a reader can get from the
//! claim to the evidence.

use crate::error::{Error, Result};
use crate::source::RawEntry;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// How much a page is to be believed. The honesty knob: a `solid` page that is
/// wrong costs more than no page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Round-trips, or every field is accounted for across every sample.
    Solid,
    /// Parses, but named fields remain unknown or unverified.
    Partial,
    /// A hypothesis written down so the next session can attack it.
    Guess,
}

impl Status {
    pub fn parse(text: &str) -> Option<Status> {
        match text.trim().to_ascii_lowercase().as_str() {
            "solid" => Some(Status::Solid),
            "partial" => Some(Status::Partial),
            "guess" => Some(Status::Guess),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Status::Solid => "solid",
            Status::Partial => "partial",
            Status::Guess => "guess",
        }
    }
}

/// One reference page.
#[derive(Debug, Clone)]
pub struct Doc {
    pub path: String,
    pub title: String,
    pub status: Option<Status>,
    /// The entries that established this page, by number.
    pub worklog: Vec<u32>,
    /// Entries in other worklogs that did - `worklog: piney:361`.
    pub elsewhere: Vec<crate::entry::EntryRef>,
    /// Evidence given section by section, under each heading as
    /// `<!-- worklog: 40, piney:12 -->`.
    pub sections: Vec<SectionEvidence>,
    /// What the page covers in the thing it documents - the files, addresses,
    /// functions - as groups (`;`) of items (`,`).
    pub covers: Vec<Vec<String>>,
    pub body: String,
    pub content_hash: String,
    pub extra: BTreeMap<String, String>,
}

impl Doc {
    pub fn parse(raw: &RawEntry) -> Result<Self> {
        let text =
            std::str::from_utf8(&raw.bytes).map_err(|_| Error::entry(&raw.path, "not UTF-8"))?;
        let (front_text, body) = crate::entry::split_front_matter(text).unwrap_or(("", text));
        let mut fields =
            crate::entry::fields(front_text).map_err(|problem| Error::entry(&raw.path, problem))?;

        // A page with no front matter still has a name: its heading, or failing
        // that its filename. Docs predate this tool in most repos.
        let title = fields
            .remove("title")
            .filter(|title| !title.is_empty())
            .or_else(|| heading(body))
            .unwrap_or_else(|| slug_of(&raw.path).replace('-', " "));

        // A status that is none of the three is kept as written, among the
        // other keys, for `check` to name - dropped, it read as no status.
        let status = match fields.remove("status") {
            Some(text) => match Status::parse(&text) {
                Some(status) => Some(status),
                None => {
                    fields.insert("status".into(), text);
                    None
                }
            },
            None => None,
        };
        let (worklog, elsewhere) = fields
            .remove("worklog")
            .map(|value| cited(&value))
            .transpose()
            .map_err(|problem| Error::entry(&raw.path, problem))?
            .unwrap_or_default();
        let sections =
            section_evidence(body).map_err(|problem| Error::entry(&raw.path, problem))?;

        let covers = fields
            .remove("covers")
            .map(|value| covered(&value))
            .unwrap_or_default();

        Ok(Doc {
            path: raw.path.clone(),
            title,
            status,
            worklog,
            elsewhere,
            sections,
            covers,
            body: body.trim_start_matches('\n').to_string(),
            content_hash: {
                let mut out = String::from("sha256:");
                for byte in Sha256::digest(&raw.bytes) {
                    out.push_str(&format!("{byte:02x}"));
                }
                out
            },
            extra: fields,
        })
    }

    /// A `README.md` or `index.md` is not a page inside its directory - it is
    /// that directory's own page. `docs/spec/README.md` is the spec section,
    /// not a page sitting beside the things it introduces.
    pub fn is_index(&self) -> bool {
        let name = self
            .path
            .rsplit('/')
            .next()
            .unwrap_or(&self.path)
            .to_ascii_lowercase();
        name == "readme.md" || name == "index.md"
    }

    /// The page's URL path *under* the docs root, without its extension, so
    /// with a root of `docs` the page `docs/formats/pod.md` is served at
    /// `docs/formats/pod/`. A directory's index takes the directory itself.
    pub fn slug(&self, root: &str) -> String {
        let inside = self.inside(root);
        let stem = if self.is_index() {
            inside.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("")
        } else {
            inside.strip_suffix(".md").unwrap_or(inside)
        };
        stem.split('/')
            .filter(|part| !part.is_empty())
            .map(crate::entry::slugify)
            .collect::<Vec<_>>()
            .join("/")
    }

    fn inside<'a>(&'a self, root: &str) -> &'a str {
        self.path
            .strip_prefix(root)
            .unwrap_or(&self.path)
            .trim_start_matches('/')
    }

    /// The directory it sits in, relative to the docs root - the grouping the
    /// index uses, because a docs tree is already organised by its author.
    pub fn section(&self, root: &str) -> String {
        let inside = self.inside(root);
        // A directory's index belongs to the directory *above* it, the way a
        // chapter title belongs to the book rather than to the chapter.
        let dir = match inside.rsplit_once('/') {
            Some((dir, _)) => dir,
            None => "",
        };
        if self.is_index() {
            return dir
                .rsplit_once('/')
                .map(|(above, _)| above)
                .unwrap_or("")
                .to_string();
        }
        dir.to_string()
    }
}

fn heading(body: &str) -> Option<String> {
    body.lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(|title| title.trim().to_string())
}

fn slug_of(path: &str) -> String {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .strip_suffix(".md")
        .unwrap_or(path)
        .to_string()
}

/// The entry numbers a page cites.
///
/// Comma separated, and a part may be a range: a page established over a run of
/// entries is naturally written `7 to 20`, and hellbender's port plan was
/// written that way before this tool existed. Both `7 to 20` and `7-20` expand,
/// inclusive.
/// `covers:` as groups of items: `;` between groups, `,` between items, so
/// `INF a.prg:0x10 f, 0x20 g; INF b.prg:0x30 h` is two groups - each usually
/// one binary - of what the page accounts for.
/// A page's `## Unknown` section - what it says it does not know - as markdown,
/// or `None` when it has none or it says nothing is unknown.
///
/// Reference pages keep their open questions in a section of their own, the
/// way entries keep them in a trailer; collected, they belong beside the log's.
/// `Unknowns`, `Still unknown` and `Open questions` are taken as the same.
pub fn unknown_section(body: &str) -> Option<String> {
    let mut lines = body.lines();
    lines.find(|line| {
        let heading = line.trim();
        let Some(name) = heading.strip_prefix("## ") else {
            return false;
        };
        let name = name.trim().trim_end_matches(':').to_ascii_lowercase();
        matches!(
            name.as_str(),
            "unknown" | "unknowns" | "still unknown" | "open questions"
        )
    })?;
    let section: Vec<&str> = lines
        .take_while(|line| !line.starts_with("## ") && !line.starts_with("# "))
        .collect();
    let text = section.join("\n").trim().to_string();
    // As an entry's trailer: a first sentence of "None" or "Nothing" says
    // nothing is open, whatever note follows - "None: every file is
    // accounted for above." is not a question.
    let first = text
        .split(['.', ':', ';', '\n'])
        .next()
        .unwrap_or("")
        .trim();
    let says_nothing = first.eq_ignore_ascii_case("nothing") || first.eq_ignore_ascii_case("none");
    (!text.is_empty() && !says_nothing).then_some(text)
}

fn covered(value: &str) -> Vec<Vec<String>> {
    value
        .split(';')
        .map(|group| {
            group
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|group| !group.is_empty())
        .collect()
}

/// One section's evidence: the heading it is under, its anchor on the page,
/// and the entries it rests on - in this log and in others.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionEvidence {
    pub heading: String,
    pub anchor: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub worklog: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elsewhere: Vec<crate::entry::EntryRef>,
}

/// The `<!-- worklog: ... -->` comments in a page, each belonging to the `##`
/// or `###` heading above it. A comment, so a forge rendering the page shows
/// nothing; the site shows it under the heading.
pub fn section_evidence(body: &str) -> std::result::Result<Vec<SectionEvidence>, String> {
    let mut found: Vec<SectionEvidence> = Vec::new();
    let mut heading: Option<String> = None;
    let mut fenced = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        if let Some(text) = trimmed
            .strip_prefix("### ")
            .or_else(|| trimmed.strip_prefix("## "))
        {
            heading = Some(text.trim().to_string());
            continue;
        }
        let Some(inner) = trimmed
            .strip_prefix("<!--")
            .and_then(|rest| rest.strip_suffix("-->"))
            .map(str::trim)
            .and_then(|inner| inner.strip_prefix("worklog:"))
        else {
            continue;
        };
        let Some(heading) = &heading else {
            return Err("a `<!-- worklog: -->` comment comes before any `##` heading - put it under the heading it is evidence for, or use `worklog:` in the front matter".into());
        };
        let (worklog, elsewhere) = cited(inner)?;
        match found.iter_mut().find(|s| &s.heading == heading) {
            Some(section) => {
                section.worklog.extend(worklog);
                section.elsewhere.extend(elsewhere);
            }
            None => found.push(SectionEvidence {
                anchor: crate::entry::slugify(&heading.replace('`', "")),
                heading: heading.clone(),
                worklog,
                elsewhere,
            }),
        }
    }
    Ok(found)
}

/// A `worklog:` list: entry numbers and ranges in this log (`12, 15-18`), and
/// entries in others (`piney:361`).
fn cited(value: &str) -> std::result::Result<(Vec<u32>, Vec<crate::entry::EntryRef>), String> {
    let mut numbers = Vec::new();
    let mut elsewhere = Vec::new();
    for part in value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if let Some(other) = crate::entry::parse_entry_ref(part) {
            if !elsewhere.contains(&other) {
                elsewhere.push(other);
            }
            continue;
        }
        let range = part
            .split_once(" to ")
            .or_else(|| part.split_once('-'))
            .map(|(from, to)| (from.trim(), to.trim()));

        let bad = || {
            format!(
                "`worklog` has {part:?} in it, which is not an entry number, a range, or an \
                 entry in another worklog (`name:12`)"
            )
        };
        match range {
            Some((from, to)) => {
                let from: u32 = from.parse().map_err(|_| bad())?;
                let to: u32 = to.parse().map_err(|_| bad())?;
                if to < from {
                    return Err(format!(
                        "`worklog` has {part:?} in it, which counts backwards"
                    ));
                }
                numbers.extend(from..=to);
            }
            None => numbers.push(part.parse().map_err(|_| bad())?),
        }
    }
    numbers.dedup();
    Ok((numbers, elsewhere))
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_page_says_what_it_does_not_know_under_its_own_heading() {
        let body = "# Page\n\nWhat is true.\n\n## Unknown\n\n- the second table\n- its last word\n\n## Checked\n\nAll of it.\n";
        assert_eq!(
            super::unknown_section(body).as_deref(),
            Some("- the second table\n- its last word")
        );
        assert_eq!(super::unknown_section("## Unknowns\n\nNothing.\n"), None);
        assert_eq!(
            super::unknown_section("## Unknown\n\nNone: every file is accounted for above.\n"),
            None
        );
        assert!(super::unknown_section("## Unknown\n\nNone of the offsets past 0x40.\n").is_some());
        assert_eq!(super::unknown_section("## Unknown factors\n\nx\n"), None);
        assert_eq!(super::unknown_section("No heading at all."), None);
    }

    #[test]
    fn a_section_cites_its_own_evidence() {
        let body = "# Page\n\n<!-- not evidence -->\n\n## Layout\n<!-- worklog: 12, 15-16 -->\n\ntext\n\n### The `index` table\n<!-- worklog: piney:40 -->\n\n```\n<!-- worklog: 99 -->\n```\n";
        let found = super::section_evidence(body).unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].heading, "Layout");
        assert_eq!(found[0].worklog, vec![12, 15, 16]);
        assert_eq!(found[1].anchor, "the-index-table");
        assert_eq!(found[1].elsewhere[0].to_string(), "piney:40");
        assert!(super::section_evidence("<!-- worklog: 3 -->\n## Late\n").is_err());
    }

    #[test]
    fn covers_is_groups_of_items() {
        assert_eq!(
            super::covered("INF a.prg:0x10 f, 0x20 g; INF b.prg:0x30 h ;; ,"),
            vec![
                vec!["INF a.prg:0x10 f".to_string(), "0x20 g".to_string()],
                vec!["INF b.prg:0x30 h".to_string()],
            ]
        );
    }

    use super::cited;

    #[test]
    fn a_page_may_cite_numbers_or_a_range() {
        assert_eq!(cited("3, 29, 30").unwrap().0, vec![3, 29, 30]);
        assert_eq!(cited("7 to 10").unwrap().0, vec![7, 8, 9, 10]);
        assert_eq!(cited("7-9, 12").unwrap().0, vec![7, 8, 9, 12]);
        assert_eq!(cited("5").unwrap().0, vec![5]);
        assert!(cited("nine").is_err());
        assert!(cited("20 to 7").unwrap_err().contains("backwards"));
        // Another worklog's entry, beside this one's - and a name with a dash
        // is not read as a range.
        let (here, elsewhere) = cited("12, piney-apples:361").unwrap();
        assert_eq!(here, vec![12]);
        assert_eq!(elsewhere[0].to_string(), "piney-apples:361");
    }
}
