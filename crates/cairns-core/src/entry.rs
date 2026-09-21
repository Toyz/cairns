use crate::date::Date;
use crate::error::{Error, Result};
use crate::source::RawEntry;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// The longest a derived slug may be, in characters, before it is cut back to a
/// word boundary. The slug is the URL, so it is cut kindly.
pub const MAX_SLUG: usize = 60;

/// The trailer that closes an entry and feeds the log's open questions.
pub const STILL_UNKNOWN: &str = "**Still unknown:**";

/// An entry's front matter, as defined in `docs/spec/entry.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontMatter {
    pub number: u32,
    pub title: String,
    pub date: Date,
    pub areas: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<u32>,
    /// Keys the spec does not define, passed through untouched so a project can
    /// carry its own metadata without forking the format.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, String>,
}

/// One parsed entry: its front matter, its prose, and the hash of the file it
/// came from.
#[derive(Debug, Clone)]
pub struct Entry {
    pub front: FrontMatter,
    pub body: String,
    pub path: String,
    pub content_hash: String,
}

impl Entry {
    pub fn parse(raw: &RawEntry) -> Result<Self> {
        let text =
            std::str::from_utf8(&raw.bytes).map_err(|_| Error::entry(&raw.path, "not UTF-8"))?;
        let (front_text, body) = split(text).ok_or_else(|| {
            Error::entry(&raw.path, "no front matter - a file must open with `---`")
        })?;
        let front =
            FrontMatter::parse(front_text).map_err(|problem| Error::entry(&raw.path, problem))?;

        Ok(Entry {
            front,
            body: body.trim_start_matches('\n').to_string(),
            path: raw.path.clone(),
            content_hash: hash(&raw.bytes),
        })
    }

    /// The URL segment: the explicit slug if the entry froze one, else derived
    /// from the title.
    pub fn slug(&self) -> String {
        self.front
            .slug
            .clone()
            .unwrap_or_else(|| slugify(&self.front.title))
    }

    /// The filename this entry should have, given its number and slug.
    pub fn filename(&self) -> String {
        format!("{:04}-{}.md", self.front.number, self.slug())
    }

    /// The one sentence used as the index blurb and the link preview: written
    /// explicitly if the author bothered, else the body's first sentence.
    pub fn summary(&self) -> Option<String> {
        if let Some(written) = &self.front.summary {
            return Some(written.clone());
        }
        first_sentence(&self.body)
    }

    /// What the entry says it still does not know, or `None` if it closed out.
    ///
    /// The trailer has to begin a line, and the last one wins, because an entry
    /// is perfectly entitled to mention the marker in its prose - entry 2 of
    /// this log does, and an unanchored search read that instead of the trailer.
    pub fn still_unknown(&self) -> Option<String> {
        let start = self
            .body
            .match_indices(STILL_UNKNOWN)
            .map(|(at, _)| at)
            .filter(|at| *at == 0 || self.body[..*at].ends_with('\n'))
            .last()?
            + STILL_UNKNOWN.len();
        let rest = &self.body[start..];
        let end = rest.find("\n\n").unwrap_or(rest.len());
        let text = rest[..end].split_whitespace().collect::<Vec<_>>().join(" ");
        let closed = text.trim_end_matches('.').eq_ignore_ascii_case("nothing");
        (!text.is_empty() && !closed).then_some(text)
    }
}

impl FrontMatter {
    /// Parse the strict `key: value` subset defined in `docs/spec/entry.md`.
    ///
    /// Not a YAML parser, and deliberately so: the subset is small enough to
    /// have exactly one reading, which is worth more here than the convenience
    /// of nesting nobody needs.
    pub fn parse(text: &str) -> std::result::Result<Self, String> {
        let mut fields: BTreeMap<String, String> = BTreeMap::new();
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let (key, value) = line
                .split_once(':')
                .ok_or_else(|| format!("front matter line {line:?} has no `key: value`"))?;
            fields.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }

        let take = |fields: &mut BTreeMap<String, String>, key: &str| fields.remove(key);
        let need = |fields: &mut BTreeMap<String, String>, key: &str| {
            take(fields, key).ok_or_else(|| format!("front matter has no `{key}`"))
        };

        let number = need(&mut fields, "number")?
            .parse()
            .map_err(|_| "`number` is not a number".to_string())?;
        let title = need(&mut fields, "title")?;
        let date: Date = need(&mut fields, "date")?.parse()?;
        let areas = list(&need(&mut fields, "area")?);
        if areas.is_empty() {
            return Err("front matter has no `area`".into());
        }

        let files = fields.remove("files").map(|v| list(&v)).unwrap_or_default();
        let supersedes = fields
            .remove("supersedes")
            .map(|v| {
                list(&v)
                    .iter()
                    .map(|n| {
                        n.parse::<u32>().map_err(|_| {
                            format!("`supersedes` has {n:?} in it, which is not an entry number")
                        })
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();

        Ok(FrontMatter {
            number,
            title,
            date,
            areas,
            files,
            summary: fields.remove("summary").filter(|s| !s.is_empty()),
            slug: fields.remove("slug").filter(|s| !s.is_empty()),
            supersedes,
            extra: fields,
        })
    }
}

/// Split `---\n...\n---\n` off the front of a file.
fn split(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    let after = rest[end + 4..]
        .strip_prefix('\n')
        .unwrap_or(&rest[end + 4..]);
    Some((&rest[..end], after))
}

/// A comma-separated field, trimmed, with the empties dropped.
///
/// Both `format, tooling` and `decomp,format` occur in the wild - hellbender's
/// log has 31 of the first and 19 of the second - so both are read, and one of
/// them is written.
fn list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

/// Title to URL segment: ASCII, lower case, hyphenated, and cut back to a word
/// boundary rather than mid-word at exactly [`MAX_SLUG`].
pub fn slugify(title: &str) -> String {
    let mut out = String::new();
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-');

    if out.len() <= MAX_SLUG {
        return out.to_string();
    }
    let cut = &out[..MAX_SLUG];
    match cut.rfind('-') {
        Some(boundary) if boundary > 0 => cut[..boundary].to_string(),
        _ => cut.to_string(),
    }
}

/// The first sentence of the prose, skipping the entry's own `# N. Title`.
fn first_sentence(body: &str) -> Option<String> {
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
    match prose.find(". ") {
        Some(stop) => Some(prose[..=stop].trim().to_string()),
        None => Some(prose.trim_end_matches('.').to_string() + "."),
    }
}

fn hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::from("sha256:");
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(body: &str) -> Entry {
        let text =
            format!("---\nnumber: 1\ntitle: A title\ndate: 2026-09-20\narea: spec\n---\n\n{body}");
        Entry::parse(&RawEntry {
            path: "worklog/0001-a-title.md".into(),
            bytes: text.into_bytes(),
        })
        .expect("parses")
    }

    #[test]
    fn slug_is_cut_at_a_word_boundary() {
        let slug = slugify("A perspective frame, and two convention bugs the picture found");
        assert!(slug.len() <= MAX_SLUG);
        assert_eq!(
            slug,
            "a-perspective-frame-and-two-convention-bugs-the-picture"
        );
    }

    #[test]
    fn short_titles_are_untouched() {
        assert_eq!(slugify("The doors open"), "the-doors-open");
        assert_eq!(slugify("The player's guns"), "the-player-s-guns");
    }

    #[test]
    fn the_trailer_is_the_last_one_that_begins_a_line() {
        // An entry about this format mentions the marker in prose; the trailer
        // is still the trailer. This is the bug worklog 2 records.
        let parsed = entry(
            "# 1. A title\n\nProse that mentions `**Still unknown:**` in passing.\n\n\
             **Still unknown:** the actual open question.\n",
        );
        assert_eq!(
            parsed.still_unknown().as_deref(),
            Some("the actual open question.")
        );
    }

    #[test]
    fn nothing_closes_an_entry_out() {
        assert_eq!(
            entry("# 1. A title\n\nProse.\n\n**Still unknown:** nothing.\n").still_unknown(),
            None
        );
        assert_eq!(entry("# 1. A title\n\nProse.\n").still_unknown(), None);
    }

    #[test]
    fn both_area_spellings_read() {
        let spaced =
            FrontMatter::parse("number: 1\ntitle: t\ndate: 2026-09-20\narea: format, tooling")
                .unwrap();
        let tight =
            FrontMatter::parse("number: 1\ntitle: t\ndate: 2026-09-20\narea: format,tooling")
                .unwrap();
        assert_eq!(spaced.areas, tight.areas);
        assert_eq!(spaced.areas, vec!["format", "tooling"]);
    }

    #[test]
    fn unknown_front_matter_keys_survive() {
        let front = FrontMatter::parse(
            "number: 1\ntitle: t\ndate: 2026-09-20\narea: spec\nticket: ENG-412",
        )
        .unwrap();
        assert_eq!(
            front.extra.get("ticket").map(String::as_str),
            Some("ENG-412")
        );
    }

    #[test]
    fn a_missing_required_field_names_itself() {
        let problem = FrontMatter::parse("number: 1\ntitle: t\narea: spec").unwrap_err();
        assert!(problem.contains("date"), "{problem}");
    }

    #[test]
    fn summary_falls_back_to_the_first_sentence() {
        let parsed = entry("# 1. A title\n\nThe first sentence. The second one.\n");
        assert_eq!(parsed.summary().as_deref(), Some("The first sentence."));
    }
}
