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

        let status = fields
            .remove("status")
            .and_then(|text| Status::parse(&text));
        let worklog = fields
            .remove("worklog")
            .map(|value| cited(&value))
            .transpose()
            .map_err(|problem| Error::entry(&raw.path, problem))?
            .unwrap_or_default();

        Ok(Doc {
            path: raw.path.clone(),
            title,
            status,
            worklog,
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
fn cited(value: &str) -> std::result::Result<Vec<u32>, String> {
    let mut numbers = Vec::new();
    for part in value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let range = part
            .split_once(" to ")
            .or_else(|| part.split_once('-'))
            .map(|(from, to)| (from.trim(), to.trim()));

        let bad = || format!("`worklog` has {part:?} in it, which is not an entry number");
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
    Ok(numbers)
}

#[cfg(test)]
mod tests {
    use super::cited;

    #[test]
    fn a_page_may_cite_numbers_or_a_range() {
        assert_eq!(cited("3, 29, 30").unwrap(), vec![3, 29, 30]);
        assert_eq!(cited("7 to 10").unwrap(), vec![7, 8, 9, 10]);
        assert_eq!(cited("7-9, 12").unwrap(), vec![7, 8, 9, 12]);
        assert_eq!(cited("5").unwrap(), vec![5]);
        assert!(cited("nine").is_err());
        assert!(cited("20 to 7").unwrap_err().contains("backwards"));
    }
}
