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
    /// Questions this one answers: an earlier entry's whole trailer (`54`) or
    /// one question of its list (`54.2`). Distinct from `supersedes`:
    /// answering a question does not mean the entry that asked it was wrong.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolves: Vec<QuestionRef>,
    /// Questions this one takes over unanswered - a triage entry gathering
    /// what is open into one list. Not `resolves`: nothing was answered, and
    /// the entry that asked keeps its question, pointing here.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub carries: Vec<QuestionRef>,
    /// When the work began, as `cairns start` stamped it: a local timestamp
    /// with its offset. Text, because it is a record of what the clock said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started: Option<String>,
    /// How long the work took, in minutes - wall-clock from `cairns start` to
    /// `cairns new`, or whatever the writer gave instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took: Option<u32>,
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
        let (front_text, body) = split_front_matter(text).ok_or_else(|| {
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

    /// The state of an entry's open-question trailer.
    ///
    /// Three of these four are silent failures if nothing looks for them: an
    /// entry with no trailer, and an entry whose trailer is blank, both read as
    /// "nothing open" while meaning "nobody said". Hellbender lost twenty-six
    /// entries' worth of open questions that way without a single complaint.
    pub fn trailer(&self) -> Trailer {
        let Some(start) = self
            .body
            .match_indices(STILL_UNKNOWN)
            .map(|(at, _)| at)
            .filter(|at| *at == 0 || self.body[..*at].ends_with('\n'))
            .last()
        else {
            return Trailer::Missing;
        };
        // Everything from the marker to the end of the entry, as markdown. It
        // used to stop at the first blank line and fold what it kept onto one
        // line, which read a sentence correctly and a list as one run-on
        // paragraph with its dashes left in - and 151 of piney_apples' 360
        // trailers are lists. What came after a blank line was dropped from
        // both the trailer and the page, since the page cuts the prose here.
        let text = self.body[start + STILL_UNKNOWN.len()..].trim();
        // Closed when the first sentence is `nothing`. Whatever follows it is
        // a note - "nothing. The test runs on Mutation only." - not a question.
        // Requiring the whole trailer to be the one word listed every entry
        // that said nothing and then explained as an open question, starting
        // "nothing.", and there was no way to close one and keep the note.
        let first = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let first = first.split(['.', ';', '\n']).next().unwrap_or("").trim();
        let closed = first.eq_ignore_ascii_case("nothing");

        if text.is_empty() {
            Trailer::Blank
        } else if closed {
            Trailer::Closed
        } else {
            Trailer::Open(text.to_string())
        }
    }

    /// What the entry says it still does not know, or `None` if it closed out.
    ///
    /// The trailer has to begin a line, and the last one wins, because an entry
    /// is perfectly entitled to mention the marker in its prose - entry 2 of
    /// this log does, and an unanchored search read that instead of the trailer.
    pub fn still_unknown(&self) -> Option<String> {
        match self.trailer() {
            Trailer::Open(text) => Some(text),
            _ => None,
        }
    }
}

/// What an entry said about what it did not know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trailer {
    /// No `**Still unknown:**` line at all. Nobody said.
    Missing,
    /// The line is there and says nothing after it.
    Blank,
    /// `nothing` - the deliberate act of closing the entry out.
    Closed,
    Open(String),
}

impl FrontMatter {
    /// Parse the strict `key: value` subset defined in `docs/spec/entry.md`.
    ///
    /// Not a YAML parser, and deliberately so: the subset is small enough to
    /// have exactly one reading, which is worth more here than the convenience
    /// of nesting nobody needs.
    pub fn parse(text: &str) -> std::result::Result<Self, String> {
        let mut fields = fields(text)?;

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
        // `supersedes` and `resolves` are both lists of earlier entry numbers,
        // and both are wrong in the same ways.
        let numbers = |fields: &mut BTreeMap<String, String>, key: &'static str| {
            fields
                .remove(key)
                .map(|value| {
                    list(&value)
                        .iter()
                        .map(|n| {
                            n.parse::<u32>().map_err(|_| {
                                format!("`{key}` has {n:?} in it, which is not an entry number")
                            })
                        })
                        .collect::<std::result::Result<Vec<_>, _>>()
                })
                .transpose()
        };
        let supersedes = numbers(&mut fields, "supersedes")?.unwrap_or_default();
        let questions = |fields: &mut BTreeMap<String, String>, key: &'static str| {
            fields
                .remove(key)
                .map(|value| {
                    list(&value)
                        .iter()
                        .map(|q| {
                            q.parse::<QuestionRef>().map_err(|_| {
                                format!(
                                    "`{key}` has {q:?} in it, which is not an entry number or \
                                     one question of one (`54.2`)"
                                )
                            })
                        })
                        .collect::<std::result::Result<Vec<_>, _>>()
                })
                .transpose()
        };
        let resolves = questions(&mut fields, "resolves")?.unwrap_or_default();
        let carries = questions(&mut fields, "carries")?.unwrap_or_default();

        let took = fields
            .remove("took")
            .filter(|took| !took.is_empty())
            .map(|took| parse_duration(&took))
            .transpose()?;

        Ok(FrontMatter {
            number,
            title,
            date,
            areas,
            files,
            summary: fields.remove("summary").filter(|s| !s.is_empty()),
            slug: fields.remove("slug").filter(|s| !s.is_empty()),
            supersedes,
            resolves,
            carries,
            started: fields.remove("started").filter(|s| !s.is_empty()),
            took,
            extra: fields,
        })
    }
}

/// Split `---\n...\n---\n` off the front of a file.
/// The strict `key: value` subset, as one map. Shared with reference pages,
/// which use the same front matter with different keys in it.
pub fn fields(text: &str) -> std::result::Result<BTreeMap<String, String>, String> {
    let mut fields: BTreeMap<String, String> = BTreeMap::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| format!("front matter line {line:?} has no `key: value`"))?;
        fields.insert(
            key.trim().to_ascii_lowercase(),
            unquote(value.trim()).to_string(),
        );
    }
    Ok(fields)
}

/// A value wrapped in one pair of matching quotes, without them.
///
/// The subset never quotes, but writers do - a title with a colon or a
/// semicolon in it looks like it needs protecting, and a model writing front
/// matter quotes it on reflex. piney_apples had 103 of 360 titles quoted, and
/// the quotes were printed on every one. A YAML reader strips them, so reading
/// them as delimiters is also the reading the spec already promises agrees
/// with YAML. Only a pair with no quote of the same kind inside comes off, so
/// a title that merely starts and ends with quoted words keeps them.
fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
            && !inner.contains(quote)
        {
            return inner;
        }
    }
    value
}

pub fn split_front_matter(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    let after = rest[end + 4..]
        .strip_prefix('\n')
        .unwrap_or(&rest[end + 4..]);
    Some((&rest[..end], after))
}

/// A question in an earlier entry: its whole trailer, or one question of a
/// trailer written as a list, counted from 1 as the entry's page numbers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QuestionRef {
    pub entry: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<u32>,
}

impl std::str::FromStr for QuestionRef {
    type Err = ();
    fn from_str(text: &str) -> std::result::Result<Self, ()> {
        let (entry, item) = match text.trim().split_once('.') {
            Some((entry, item)) => (entry, Some(item.parse::<u32>().map_err(|_| ())?)),
            None => (text.trim(), None),
        };
        if item == Some(0) {
            return Err(());
        }
        Ok(QuestionRef {
            entry: entry.parse().map_err(|_| ())?,
            item,
        })
    }
}

impl std::fmt::Display for QuestionRef {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.item {
            Some(item) => write!(f, "{}.{item}", self.entry),
            None => write!(f, "{}", self.entry),
        }
    }
}

/// A trailer split into what comes before its list and the list's questions,
/// each with whatever continues it. A trailer that is not a list has none.
///
/// Only items at the start of a line count, so a list inside a question - or a
/// dash in a sentence - is part of the question it is in.
pub fn question_items(trailer: &str) -> (String, Vec<String>) {
    let starts = |line: &str| {
        line.starts_with("- ")
            || line.starts_with("* ")
            || line.starts_with("+ ")
            || line.split_once(['.', ')']).is_some_and(|(n, rest)| {
                !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) && rest.starts_with(' ')
            })
    };
    let mut preamble = Vec::new();
    let mut items: Vec<Vec<&str>> = Vec::new();
    for line in trailer.lines() {
        if starts(line) {
            items.push(vec![line]);
        } else if let Some(current) = items.last_mut() {
            current.push(line);
        } else {
            preamble.push(line);
        }
    }
    let items = items
        .into_iter()
        .map(|lines| {
            let first = lines[0];
            let marker = first.find(' ').map(|at| at + 1).unwrap_or(0);
            let mut text = first[marker..].to_string();
            for line in &lines[1..] {
                text.push('\n');
                text.push_str(line);
            }
            text.trim().to_string()
        })
        .collect();
    (preamble.join("\n").trim().to_string(), items)
}

/// Whether a trailer opens with "nothing" and then qualifies it - `nothing
/// about the offset`, `nothing new from this entry` - which reads as closed and
/// is not: what follows may be a real question, and no reader of the text can
/// tell which. `nothing.` closes an entry; anything else names what is open.
pub fn hedged_nothing(trailer: &str) -> bool {
    let text = trailer.trim_start();
    let Some(rest) = text
        .get(..7)
        .filter(|w| w.eq_ignore_ascii_case("nothing"))
        .map(|_| &text[7..])
    else {
        return false;
    };
    rest.starts_with([' ', '\t'])
        && !rest.trim_start().starts_with(['.', ';'])
        && !rest.trim().is_empty()
}

/// A duration as a writer puts it - `1h 23m`, `45m`, `2h` - in minutes.
///
/// Hours and minutes only, each at most once, hours first. Strict, because a
/// number that reads two ways - is `90` minutes or hours? - is a number that is
/// wrong half the time, and this one ends up summed across a whole log.
pub fn parse_duration(text: &str) -> std::result::Result<u32, String> {
    let bad = || format!("`took` is {text:?}; write it as 1h 23m, 45m or 2h");
    let mut minutes = 0u32;
    let mut seen_hours = false;
    let mut seen_minutes = false;
    for part in text.split_whitespace() {
        let (number, unit) =
            part.split_at(part.find(|c: char| !c.is_ascii_digit()).ok_or_else(bad)?);
        let number: u32 = number.parse().map_err(|_| bad())?;
        match unit {
            "h" if !seen_hours && !seen_minutes => {
                seen_hours = true;
                minutes += number * 60;
            }
            "m" if !seen_minutes => {
                seen_minutes = true;
                minutes += number;
            }
            _ => return Err(bad()),
        }
    }
    if !seen_hours && !seen_minutes {
        return Err(bad());
    }
    Ok(minutes)
}

/// Minutes as a person writes them: `1h 23m`, `45m`, `2h`.
pub fn format_duration(minutes: u32) -> String {
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
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
    if let Some(stop) = prose.find(". ") {
        return Some(prose[..=stop].trim().to_string());
    }
    // One sentence with no ". " in it. It may already end in a full stop that
    // trailing emphasis hides - `a wall.*` - and appending another gives `..`.
    let bare = prose.trim_end_matches(['*', '_', '`', ')', ']']);
    if bare.ends_with(['.', '!', '?']) {
        return Some(prose.to_string());
    }
    Some(format!("{prose}."))
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
    fn a_code_reference_is_a_path_a_range_or_a_name() {
        let parse = |s: &str| CodeRef::parse(s);
        let r = parse("src/entry.rs:120-158@3fbdc65|the scanner").unwrap();
        assert_eq!(
            (
                r.path.as_str(),
                r.lines,
                r.rev.as_deref(),
                r.label.as_deref()
            ),
            (
                "src/entry.rs",
                Some((120, 158)),
                Some("3fbdc65"),
                Some("the scanner")
            )
        );
        assert_eq!(
            parse("src/entry.rs#question_items")
                .unwrap()
                .symbol
                .as_deref(),
            Some("question_items")
        );
        assert_eq!(parse("Cargo.toml").unwrap().lines, None);
        assert_eq!(parse("src/a.rs:7").unwrap().key(), "src/a.rs:7");
        for not in [
            "12",
            "area",
            "publish",
            "https://example.com/x.rs",
            "a b.rs",
            "/etc/passwd",
        ] {
            assert!(parse(not).is_none(), "{not:?} parsed");
        }
        // A bad range is not a range: the colon stays part of the path.
        assert_eq!(parse("src/a.rs:40-12").unwrap().lines, None);
    }

    #[test]
    fn code_references_are_found_beside_entry_references_and_not_in_code() {
        let body = "See [[12]], [[src/a.rs#parse]] and\n![[src/b.rs:3-9]]\n\n`[[src/c.rs]]` stays.\n\n```\n[[src/d.rs]]\n```\n";
        let found = code_references(body);
        assert_eq!(
            found.iter().map(|c| (c.key(), c.embed)).collect::<Vec<_>>(),
            vec![
                ("src/a.rs#parse".to_string(), false),
                ("src/b.rs:3-9".to_string(), true)
            ]
        );
        assert_eq!(references(body).len(), 1);
        let rewritten = rewrite_code_references(body, &|c| Some(format!("<{}>", c.key())));
        assert!(
            rewritten.contains("See [[12]], <src/a.rs#parse> and\n<src/b.rs:3-9>\n"),
            "{rewritten}"
        );
    }

    #[test]
    fn a_question_is_an_entry_or_one_item_of_its_list() {
        assert_eq!(
            "54".parse(),
            Ok(QuestionRef {
                entry: 54,
                item: None
            })
        );
        assert_eq!(
            "54.2".parse(),
            Ok(QuestionRef {
                entry: 54,
                item: Some(2)
            })
        );
        for bad in ["54.0", "x", "54.x", "", "54.2.1"] {
            assert!(bad.parse::<QuestionRef>().is_err(), "{bad:?} parsed");
        }
        assert_eq!(
            QuestionRef {
                entry: 54,
                item: Some(2)
            }
            .to_string(),
            "54.2"
        );
    }

    #[test]
    fn a_trailer_list_splits_into_its_questions() {
        let (preamble, items) = question_items(
            "Two things are untested:\n- the first,\n  wrapped\n- the second - with a dash\n\n1. a third, numbered",
        );
        assert_eq!(preamble, "Two things are untested:");
        assert_eq!(
            items,
            vec![
                "the first,\n  wrapped",
                "the second - with a dash",
                "a third, numbered"
            ]
        );
        assert_eq!(
            question_items("just a sentence."),
            ("just a sentence.".to_string(), vec![])
        );
    }

    #[test]
    fn nothing_with_a_qualifier_is_hedged() {
        assert!(hedged_nothing("nothing about the offset. Whether ..."));
        assert!(hedged_nothing("Nothing new from this entry."));
        assert!(!hedged_nothing("nothing"));
        assert!(!hedged_nothing("nothing. The test runs on Mutation only."));
        assert!(!hedged_nothing("nothing ; settled by [[4]]"));
        assert!(!hedged_nothing("nothingness of the void"));
        assert!(!hedged_nothing("whether nothing reads it"));
    }

    #[test]
    fn nothing_closes_an_entry_even_with_a_note_after_it() {
        let trailer = |text: &str| {
            entry(&format!(
                "# 1. A title\n\nProse.\n\n**Still unknown:** {text}\n"
            ))
            .trailer()
        };
        assert_eq!(trailer("nothing"), Trailer::Closed);
        assert_eq!(trailer("Nothing."), Trailer::Closed);
        assert_eq!(
            trailer("nothing. The session test runs on Mutation only."),
            Trailer::Closed
        );
        assert_eq!(
            trailer("nothing; the rest is settled by [[4]]."),
            Trailer::Closed
        );
        // "nothing about X" is a qualifier, and what follows is a real question.
        assert!(matches!(
            trailer("nothing about these two paths. Whether any dungeon has room 15."),
            Trailer::Open(_)
        ));
        assert!(matches!(
            trailer("nothing compares the pictures with the game's."),
            Trailer::Open(_)
        ));
    }

    #[test]
    fn a_duration_reads_one_way_only() {
        assert_eq!(parse_duration("1h 23m"), Ok(83));
        assert_eq!(parse_duration("45m"), Ok(45));
        assert_eq!(parse_duration("2h"), Ok(120));
        assert_eq!(parse_duration("0m"), Ok(0));
        for bad in ["90", "1.5h", "23m 1h", "1h 1h", "1d", "", "an hour"] {
            assert!(parse_duration(bad).is_err(), "{bad:?} parsed");
        }
        for minutes in [0, 7, 60, 83, 600] {
            assert_eq!(parse_duration(&format_duration(minutes)), Ok(minutes));
        }
    }

    #[test]
    fn a_trailer_that_is_a_list_keeps_its_items() {
        let open = entry(
            "# 1. A title\n\nProse.\n\n**Still unknown:**\n- the first thing\n- the second, \n  wrapped\n\n- a third, after a blank line\n",
        )
        .still_unknown()
        .unwrap();
        assert_eq!(
            open,
            "- the first thing\n- the second, \n  wrapped\n\n- a third, after a blank line"
        );
    }

    #[test]
    fn a_trailer_on_one_line_reads_as_before() {
        assert_eq!(
            entry("# 1. A title\n\nProse.\n\n**Still unknown:** whether it   holds.\n")
                .still_unknown()
                .as_deref(),
            Some("whether it   holds.")
        );
    }

    #[test]
    fn a_quoted_value_loses_its_quotes() {
        let fields = fields(
            "title: \"Field 28's flag; the pilot\"\nsummary: 'One line'\n\
             area: \"spec, core\"",
        )
        .unwrap();
        assert_eq!(fields["title"], "Field 28's flag; the pilot");
        assert_eq!(fields["summary"], "One line");
        assert_eq!(list(&fields["area"]), ["spec", "core"]);
    }

    #[test]
    fn a_value_that_only_starts_and_ends_with_quotes_keeps_them() {
        let fields = fields("title: \"Kite\" and \"Black Rose\"\nother: 'tis Kite's'").unwrap();
        assert_eq!(fields["title"], "\"Kite\" and \"Black Rose\"");
        assert_eq!(fields["other"], "'tis Kite's'");
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

#[cfg(test)]
mod summary_tests {
    use super::*;

    fn summary(body: &str) -> Option<String> {
        let text = format!(
            "---\nnumber: 1\ntitle: t\ndate: 2026-09-20\narea: spec\n---\n\n# 1. t\n\n{body}\n"
        );
        Entry::parse(&RawEntry {
            path: "worklog/0001-t.md".into(),
            bytes: text.into_bytes(),
        })
        .unwrap()
        .summary()
    }

    #[test]
    fn a_sentence_ending_under_emphasis_is_not_given_a_second_full_stop() {
        assert_eq!(
            summary("He said *throwing shit at a wall.*").as_deref(),
            Some("He said *throwing shit at a wall.*")
        );
        assert_eq!(
            summary("No punctuation here").as_deref(),
            Some("No punctuation here.")
        );
        assert_eq!(
            summary("First one. Second one.").as_deref(),
            Some("First one.")
        );
    }
}

/// A reference to another entry, written `[[12]]` or `[[12|in other words]]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub number: u32,
    /// The words to show, when the reference gave some.
    pub label: Option<String>,
}

/// Rewrite every `[[12]]` in a body, skipping code.
///
/// `link` is given the reference and returns the markdown destination to use,
/// or `None` to leave the text exactly as written. This has to happen on the
/// source rather than on parsed events: a markdown parser reads `[[12]]` as
/// nested bracket tokens and hands it over in pieces, so `[[` is never
/// present in one text run to match on.
pub fn rewrite_references(body: &str, link: &dyn Fn(&Reference) -> Option<String>) -> String {
    let mut out = String::with_capacity(body.len());
    scan(body, |found| match found {
        Found::Text(text) | Found::Code(text, _) => out.push_str(text),
        Found::Reference(whole, reference) => match link(&reference) {
            Some(destination) => {
                let label = reference
                    .label
                    .clone()
                    .unwrap_or_else(|| reference.number.to_string());
                out.push_str(&format!("[{label}]({destination})"));
            }
            None => out.push_str(whole),
        },
    });
    out
}

enum Found<'a> {
    Text(&'a str),
    Reference(&'a str, Reference),
    Code(&'a str, CodeRef),
}

/// A reference to code in the repository: `[[src/a.rs]]`, a line
/// `[[src/a.rs:12]]`, a block `[[src/a.rs:12-40]]`, or a definition by name
/// `[[src/a.rs#parse_header]]` - which, unlike line numbers, still finds the
/// code after it moves. `@rev` pins it to a commit; `|words` labels it; a `!`
/// before it embeds the code itself rather than linking to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeRef {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<(u32, u32)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub embed: bool,
}

impl CodeRef {
    /// The inside of `[[...]]`, or `None` when it is not a code reference. A
    /// path needs a `/` or a `.`, so `[[12]]` and `[[area]]` are not one.
    pub fn parse(inner: &str) -> Option<CodeRef> {
        let (target, label) = match inner.split_once('|') {
            Some((target, label)) => (target.trim(), Some(label.trim().to_string())),
            None => (inner.trim(), None),
        };
        let (target, rev) = match target.rsplit_once('@') {
            Some((target, rev))
                if !rev.is_empty()
                    && rev
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "._-/".contains(c)) =>
            {
                (target, Some(rev.to_string()))
            }
            _ => (target, None),
        };
        let (path, symbol, lines) = if let Some((path, symbol)) = target.split_once('#') {
            (
                path,
                Some(symbol.trim().to_string()).filter(|s| !s.is_empty()),
                None,
            )
        } else if let Some((path, range)) = target.rsplit_once(':')
            && let Some(lines) = parse_lines(range)
        {
            (path, None, Some(lines))
        } else {
            (target, None, None)
        };
        let path = path.trim().trim_start_matches("./");
        let looks_like_a_path = (path.contains('/') || path.contains('.'))
            && !path.contains(char::is_whitespace)
            && !path.contains("://")
            && !path.starts_with('/');
        looks_like_a_path.then(|| CodeRef {
            path: path.to_string(),
            lines,
            symbol,
            rev,
            label,
            embed: false,
        })
    }

    /// What identifies the code - path, place and revision, not label or
    /// embedding - so the same code referred to twice resolves once.
    pub fn key(&self) -> String {
        let mut key = self.path.clone();
        if let Some(symbol) = &self.symbol {
            key.push('#');
            key.push_str(symbol);
        } else if let Some((start, end)) = self.lines {
            key.push_str(&if start == end {
                format!(":{start}")
            } else {
                format!(":{start}-{end}")
            });
        }
        if let Some(rev) = &self.rev {
            key.push('@');
            key.push_str(rev);
        }
        key
    }
}

fn parse_lines(range: &str) -> Option<(u32, u32)> {
    let (start, end) = match range.split_once('-') {
        Some((start, end)) => (start.trim().parse().ok()?, end.trim().parse().ok()?),
        None => {
            let line = range.trim().parse().ok()?;
            (line, line)
        }
    };
    (start >= 1 && end >= start).then_some((start, end))
}

/// Walk a body, handing back its text and its references in order.
///
/// Code is skipped because a fenced TOML block full of `[[area]]` is not a
/// reference to anything, and a page documenting the syntax should be able to
/// show it without linking it.
fn scan<'a>(body: &'a str, mut hand: impl FnMut(Found<'a>)) {
    let mut fenced = false;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            hand(Found::Text(line));
            continue;
        }
        if fenced {
            hand(Found::Text(line));
            continue;
        }

        let mut at = 0;
        let mut in_code = false;
        let mut emitted = 0;
        while at < line.len() {
            // Step by characters, not bytes. `at += 1` walks into the middle
            // of anything multi-byte, and the next `line[at..]` panics on a
            // char boundary - which one `±` in a log was enough to do.
            let Some(ch) = line[at..].chars().next() else {
                break;
            };
            let step = ch.len_utf8();

            if ch == '`' {
                in_code = !in_code;
                at += step;
                continue;
            }
            if in_code || !line[at..].starts_with("[[") {
                at += step;
                continue;
            }
            let Some(close) = line[at..].find("]]") else {
                break;
            };
            let whole = &line[at..at + close + 2];
            let inner = &line[at + 2..at + close];
            let (digits, label) = match inner.split_once('|') {
                Some((digits, label)) => (digits.trim(), Some(label.trim().to_string())),
                None => (inner.trim(), None),
            };

            if !digits.is_empty()
                && digits.chars().all(|c| c.is_ascii_digit())
                && let Ok(number) = digits.parse()
            {
                hand(Found::Text(&line[emitted..at]));
                hand(Found::Reference(whole, Reference { number, label }));
                emitted = at + close + 2;
            } else if let Some(mut code) = CodeRef::parse(inner) {
                // `![[...]]` embeds: the `!` belongs to the reference.
                let bang = at > emitted && line[..at].ends_with('!');
                let start = if bang { at - 1 } else { at };
                code.embed = bang;
                hand(Found::Text(&line[emitted..start]));
                hand(Found::Code(&line[start..at + close + 2], code));
                emitted = at + close + 2;
            }
            at += close + 2;
        }
        hand(Found::Text(&line[emitted..]));
    }
}

/// Rewrite every code reference in a body, skipping code blocks: `with` gets
/// the reference and returns what replaces it, or `None` to leave it as written.
pub fn rewrite_code_references(body: &str, with: &dyn Fn(&CodeRef) -> Option<String>) -> String {
    let mut out = String::with_capacity(body.len());
    scan(body, |found| match found {
        Found::Text(text) | Found::Reference(text, _) => out.push_str(text),
        Found::Code(whole, code) => match with(&code) {
            Some(replacement) => out.push_str(&replacement),
            None => out.push_str(whole),
        },
    });
    out
}

/// Every code reference in a body, in order, skipping code blocks.
pub fn code_references(body: &str) -> Vec<CodeRef> {
    let mut found = Vec::new();
    scan(body, |item| {
        if let Found::Code(_, code) = item {
            found.push(code);
        }
    });
    found
}

/// Every `[[12]]` in a body, in order, skipping code.
///
/// Code is skipped because a fenced TOML block full of `[[area]]` is not a
/// reference to anything - and because a spec page that documents the syntax
/// should be able to show it without linking it. Only digits count as a
/// number, which alone rules out `[[area]]`, but the code rules are what make
/// that a guarantee rather than a coincidence.
pub fn references(body: &str) -> Vec<Reference> {
    let mut found = Vec::new();
    scan(body, |item| {
        if let Found::Reference(_, reference) = item {
            found.push(reference);
        }
    });
    found
}

#[cfg(test)]
mod reference_tests {
    use super::{Reference, references};

    fn numbers(body: &str) -> Vec<u32> {
        references(body).iter().map(|r| r.number).collect()
    }

    #[test]
    fn a_reference_is_a_number_in_double_brackets() {
        assert_eq!(
            numbers("As [[12]] showed, and [[6]] before it."),
            vec![12, 6]
        );
        assert_eq!(
            references("[[12|the stylesheet disaster]]")[0],
            Reference {
                number: 12,
                label: Some("the stylesheet disaster".into())
            }
        );
    }

    #[test]
    fn toml_in_a_fence_is_not_a_reference() {
        let body = "Config:\n\n```toml\n[[area]]\nname = \"spec\"\n[[publish]]\n```\n\nSee [[3]].";
        assert_eq!(numbers(body), vec![3]);
    }

    #[test]
    fn a_reference_inside_inline_code_is_left_alone() {
        assert_eq!(numbers("Write `[[12]]` to link to [[12]]."), vec![12]);
        assert_eq!(numbers("The `[[area]]` table."), Vec::<u32>::new());
    }
}

#[cfg(test)]
mod utf8_tests {
    use super::{references, rewrite_references};

    /// Walking the scanner a byte at a time sliced into the middle of a
    /// multi-byte character and panicked. One `±` in a worklog was enough.
    #[test]
    fn a_body_with_multibyte_characters_does_not_panic() {
        for body in [
            "The tolerance is ±0.5 and see [[3]].",
            "±",
            "`±` in code, then [[3]].",
            "Em dash — and [[3]] after it.",
            "Emoji 🍺 then [[3]].",
            "±[[3]]±",
            "```\n±\n```\nThen [[3]].",
        ] {
            let found = references(body);
            let rewritten = rewrite_references(body, &|r| Some(format!("cairns:{}", r.number)));
            assert!(
                !rewritten.is_empty() || body.is_empty(),
                "{body:?} rewrote to nothing"
            );
            if body.contains("[[3]]") && !body.starts_with("```") {
                assert_eq!(found.len(), 1, "{body:?} -> {found:?}");
            }
        }
    }

    #[test]
    fn multibyte_text_around_a_reference_survives_the_rewrite() {
        let out = rewrite_references("±0.5 and [[3]] and ±1", &|r| {
            Some(format!("cairns:{}", r.number))
        });
        assert_eq!(out, "±0.5 and [3](cairns:3) and ±1");
    }
}
