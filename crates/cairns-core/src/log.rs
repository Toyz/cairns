use crate::config::Config;
use crate::date::Date;
use crate::entry::Entry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// The whole worklog as one structured document. See `docs/spec/log-json.md`.
///
/// This is the only path from markdown to structured data. The site renders
/// from it, a publish delivers it, and anything added later consumes it - so
/// there is never a second reader of the entries to disagree with the first.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub spec_version: u32,
    pub generator: String,
    /// Omitted by a reproducible export, so CI can diff the payload without a
    /// timestamp defeating it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated: Option<String>,
    pub project: ProjectInfo,
    pub areas: Vec<AreaCount>,
    pub entries: Vec<LogEntry>,
    pub open_questions: Vec<OpenQuestion>,
    /// The project's README, as markdown, when it has one. Held here rather
    /// than read by the renderer so that `log.json` stays the only thing the
    /// site is built from - and so an ingest gets it too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readme: Option<String>,
    /// Reference pages, if the project keeps any.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub docs: Vec<DocPage>,
    /// What the site calls them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs_label: Option<String>,
    /// The project's own links, for the rail.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<crate::config::Link>,
}

/// One reference page in the canonical document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocPage {
    /// Its path under the docs root, without the extension: `formats/pod`.
    pub slug: String,
    pub url: String,
    pub path: String,
    pub title: String,
    /// The grouping the index uses, taken from the tree the author made.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub section: String,
    /// True when this page *is* a directory rather than a page inside one.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_index: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<crate::doc::Status>,
    /// The entries that established this page.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub worklog: Vec<u32>,
    pub body: String,
    pub content_hash: String,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub slug: String,
    pub description: String,
    pub base_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaCount {
    pub name: String,
    pub about: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// `{project}/{number}`, because a bare number collides the moment two
    /// projects sit in one place.
    pub id: String,
    pub number: u32,
    pub slug: String,
    pub url: String,
    pub path: String,
    pub title: String,
    pub date: Date,
    pub areas: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<u32>,
    /// Derived by inverting every `supersedes` in the log: the correction is
    /// recorded once, on the only entry that could have known about it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub superseded_by: Vec<u32>,
    /// Entries whose open question this one answers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolves: Vec<u32>,
    /// Derived: the entries that answered this one's open question. While this
    /// is non-empty the question is closed and is not in `open_questions`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved_by: Vec<u32>,
    /// Derived: reference pages that name this entry as their evidence.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documented_by: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub still_unknown: Option<String>,
    /// Markdown, not HTML. A newer renderer can re-render an old log, and a
    /// consumer wanting plain text is not unpicking someone else's markup.
    pub body: String,
    pub content_hash: String,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenQuestion {
    pub entry: u32,
    pub text: String,
}

impl Log {
    /// Build the canonical document. `entries` may arrive in any order.
    pub fn build(config: &Config, entries: Vec<Entry>, generated: Option<String>) -> Log {
        Log::build_with(config, entries, Vec::new(), generated)
    }

    /// Build the document from the entries and the reference pages together,
    /// so the edges between them - which page cites which entry - are derived
    /// once, here, and never recomputed by anything downstream.
    pub fn build_with(
        config: &Config,
        mut entries: Vec<Entry>,
        docs: Vec<crate::Doc>,
        generated: Option<String>,
    ) -> Log {
        entries.sort_by_key(|entry| entry.front.number);

        let mut corrected: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        let mut answered: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for entry in &entries {
            for older in &entry.front.supersedes {
                corrected
                    .entry(*older)
                    .or_default()
                    .push(entry.front.number);
            }
            for older in &entry.front.resolves {
                answered.entry(*older).or_default().push(entry.front.number);
            }
        }

        let base = config.site.base_url.trim_end_matches('/');
        let docs_root = config
            .docs
            .as_ref()
            .map(|docs| docs.dir.as_str())
            .unwrap_or("docs");

        // A page names the entries it rests on; an entry learns which pages
        // rest on it by inverting that.
        let mut cited: BTreeMap<u32, Vec<String>> = BTreeMap::new();
        let pages: Vec<DocPage> = docs
            .iter()
            .map(|doc| {
                let slug = doc.slug(docs_root);
                for number in &doc.worklog {
                    cited.entry(*number).or_default().push(doc.title.clone());
                }
                DocPage {
                    url: format!("{base}/docs/{slug}"),
                    slug,
                    path: doc.path.clone(),
                    title: doc.title.clone(),
                    section: doc.section(docs_root),
                    is_index: doc.is_index(),
                    status: doc.status,
                    worklog: doc.worklog.clone(),
                    body: doc.body.clone(),
                    content_hash: doc.content_hash.clone(),
                    extra: doc
                        .extra
                        .iter()
                        .map(|(key, value)| (key.clone(), Value::String(value.clone())))
                        .collect(),
                }
            })
            .collect();
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        let mut open_questions = Vec::new();
        let mut built = Vec::with_capacity(entries.len());

        for entry in &entries {
            for area in &entry.front.areas {
                *counts.entry(area.as_str()).or_default() += 1;
            }
            let still_unknown = entry.still_unknown();
            let resolved_by = answered
                .get(&entry.front.number)
                .cloned()
                .unwrap_or_default();
            // A question a later entry answered is no longer open. It stays on
            // the entry that asked it, pointing at the one that closed it.
            if let (Some(text), true) = (&still_unknown, resolved_by.is_empty()) {
                open_questions.push(OpenQuestion {
                    entry: entry.front.number,
                    text: text.clone(),
                });
            }

            let slug = entry.slug();
            built.push(LogEntry {
                id: format!("{}/{}", config.project.slug, entry.front.number),
                number: entry.front.number,
                url: format!("{base}/{}-{slug}", entry.front.number),
                slug,
                path: entry.path.clone(),
                title: entry.front.title.clone(),
                date: entry.front.date,
                areas: entry.front.areas.clone(),
                files: entry.front.files.clone(),
                summary: entry.summary(),
                supersedes: entry.front.supersedes.clone(),
                superseded_by: corrected
                    .get(&entry.front.number)
                    .cloned()
                    .unwrap_or_default(),
                resolves: entry.front.resolves.clone(),
                resolved_by,
                documented_by: cited.get(&entry.front.number).cloned().unwrap_or_default(),
                still_unknown,
                body: entry.body.clone(),
                content_hash: entry.content_hash.clone(),
                extra: entry
                    .front
                    .extra
                    .iter()
                    .map(|(key, value)| (key.clone(), Value::String(value.clone())))
                    .collect(),
            });
        }

        // Declared order, not count order, because the areas are a taxonomy the
        // project chose and reordering it by popularity hides that.
        let areas = config
            .areas
            .iter()
            .map(|area| AreaCount {
                name: area.name.clone(),
                about: area.about.clone(),
                count: counts.get(area.name.as_str()).copied().unwrap_or(0),
            })
            .collect();

        Log {
            spec_version: crate::SPEC_VERSION,
            generator: format!("cairns {}", env!("CARGO_PKG_VERSION")),
            generated,
            project: ProjectInfo {
                name: config.project.name.clone(),
                slug: config.project.slug.clone(),
                description: config.project.description.clone(),
                base_url: config.site.base_url.clone(),
                repository: config.project.repository.clone(),
            },
            areas,
            entries: built,
            open_questions,
            readme: None,
            docs: pages,
            docs_label: config.docs.as_ref().map(|docs| docs.label.clone()),
            links: config.links.clone(),
        }
    }
}

/// What `cairns check` reports about the reference pages.
pub fn doc_problems(entries: &[Entry], docs: &[crate::Doc]) -> Vec<String> {
    let numbers: Vec<u32> = entries.iter().map(|entry| entry.front.number).collect();
    let mut problems = Vec::new();
    for doc in docs {
        if doc.title.trim().is_empty() {
            problems.push(format!("{}: has no title", doc.path));
        }
        for number in &doc.worklog {
            if !numbers.contains(number) {
                problems.push(format!(
                    "{}: cites worklog {number}, which does not exist",
                    doc.path
                ));
            }
        }
    }
    problems
}

/// Everything `cairns check` reports, in the order a reader would want it.
///
/// Returns descriptions rather than erroring on the first one: a log with three
/// problems should print three, not send someone round the loop three times.
pub fn problems(config: &Config, entries: &[Entry]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen: BTreeMap<u32, &str> = BTreeMap::new();
    let numbers: Vec<u32> = entries.iter().map(|entry| entry.front.number).collect();

    for entry in entries {
        let number = entry.front.number;
        if let Some(first) = seen.insert(number, &entry.path) {
            problems.push(format!(
                "{}: number {number} is already used by {first}",
                entry.path
            ));
        }

        let expected = entry.filename();
        if !entry.path.ends_with(&expected) {
            problems.push(format!("{}: should be named {expected}", entry.path));
        }

        if config.check.open_questions == crate::config::Insistence::Required {
            match entry.trailer() {
                crate::entry::Trailer::Missing => problems.push(format!(
                    "{}: has no `**Still unknown:**` line - write `nothing` to close it out",
                    entry.path
                )),
                crate::entry::Trailer::Blank => problems.push(format!(
                    "{}: `**Still unknown:**` is empty - write `nothing` to close it out",
                    entry.path
                )),
                _ => {}
            }
        }

        for area in &entry.front.areas {
            if !config.knows_area(area) {
                problems.push(format!(
                    "{}: unknown area {area:?} - declare it under [area] in cairns.toml",
                    entry.path
                ));
            }
        }

        for older in &entry.front.supersedes {
            if !numbers.contains(older) {
                problems.push(format!(
                    "{}: supersedes {older}, which does not exist",
                    entry.path
                ));
            }
            if *older >= number {
                problems.push(format!(
                    "{}: supersedes {older}, which is not an earlier entry",
                    entry.path
                ));
            }
        }

        // A `[[12]]` pointing at nothing is the link-rot the wiki form exists
        // to prevent, so it is checked rather than silently left as text.
        for reference in crate::entry::references(&entry.body) {
            if !numbers.contains(&reference.number) {
                problems.push(format!(
                    "{}: references [[{}]], which does not exist",
                    entry.path, reference.number
                ));
            }
        }

        for older in &entry.front.resolves {
            match entries.iter().find(|other| other.front.number == *older) {
                None => problems.push(format!(
                    "{}: resolves {older}, which does not exist",
                    entry.path
                )),
                // Answering an entry that asked nothing is a sign the number is
                // wrong, and it is the kind of mistake nothing else would show.
                Some(other) if other.still_unknown().is_none() => problems.push(format!(
                    "{}: resolves {older}, which left no open question",
                    entry.path
                )),
                Some(_) => {}
            }
            if *older >= number {
                problems.push(format!(
                    "{}: resolves {older}, which is not an earlier entry",
                    entry.path
                ));
            }
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Entry, RawEntry};

    fn config() -> Config {
        Config::parse(
            "spec_version = 1\n[project]\nname = \"P\"\nslug = \"p\"\n[[area]]\nname = \"spec\"\n",
        )
        .unwrap()
    }

    fn entry(number: u32, front: &str, body: &str) -> Entry {
        let text = format!(
            "---\nnumber: {number}\ntitle: Title {number}\ndate: 2026-09-20\narea: spec\n{front}---\n\n\
             # {number}. Title {number}\n\n{body}\n"
        );
        Entry::parse(&RawEntry {
            path: format!("worklog/{number:04}-title-{number}.md"),
            bytes: text.into_bytes(),
        })
        .unwrap()
    }

    /// Worklog 12: this validation was written twice and landed once, because
    /// the edit that added it missed its anchor and said nothing.
    #[test]
    fn resolving_an_entry_that_asked_nothing_is_rejected() {
        let closed = entry(1, "", "Prose.\n\n**Still unknown:** nothing");
        let answering = entry(2, "resolves: 1\n", "Prose.\n\n**Still unknown:** nothing");
        let found = problems(&config(), &[closed, answering]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("left no open question"), "{found:?}");
    }

    #[test]
    fn resolving_a_question_that_was_asked_is_accepted_and_closes_it() {
        let asking = entry(1, "", "Prose.\n\n**Still unknown:** whether it works.");
        let answering = entry(
            2,
            "resolves: 1\n",
            "It works.\n\n**Still unknown:** nothing",
        );
        let entries = vec![asking, answering];
        assert!(problems(&config(), &entries).is_empty());

        let built = Log::build(&config(), entries, None);
        assert!(
            built.open_questions.is_empty(),
            "the question is still listed as open"
        );
        assert_eq!(built.entries[0].resolved_by, vec![2]);
        // It stays on the entry that asked it; only the open list drops it.
        assert!(built.entries[0].still_unknown.is_some());
    }

    #[test]
    fn a_resolves_pointing_forwards_or_nowhere_is_rejected() {
        let asking = entry(1, "", "Prose.\n\n**Still unknown:** whether it works.");
        let wrong = entry(2, "resolves: 9\n", "Prose.\n\n**Still unknown:** nothing");
        let found = problems(&config(), &[asking, wrong]);
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found.iter().any(|p| p.contains("does not exist")));
        assert!(found.iter().any(|p| p.contains("not an earlier entry")));
    }
}

#[cfg(test)]
mod trailer_tests {
    use super::*;
    use crate::config::Insistence;
    use crate::{Entry, RawEntry};

    fn config(insist: Insistence) -> Config {
        let relax = if insist == Insistence::Optional {
            "[check]\nopen_questions = \"optional\"\n"
        } else {
            ""
        };
        Config::parse(&format!(
            "spec_version = 1\n[project]\nname = \"P\"\nslug = \"p\"\n\
             [[area]]\nname = \"spec\"\n{relax}"
        ))
        .unwrap()
    }

    fn entry(number: u32, body: &str) -> Entry {
        let text = format!(
            "---\nnumber: {number}\ntitle: T{number}\ndate: 2026-09-20\narea: spec\n---\n\n\
             # {number}. T{number}\n\n{body}\n"
        );
        Entry::parse(&RawEntry {
            path: format!("worklog/{number:04}-t{number}.md"),
            bytes: text.into_bytes(),
        })
        .unwrap()
    }

    /// Hellbender lost twenty-six entries' worth of open questions by simply
    /// not writing the line, and nothing complained: a missing convention is
    /// not a broken one until something insists on it.
    #[test]
    fn an_entry_that_never_says_what_is_unknown_is_a_problem() {
        let missing = entry(1, "Prose with no trailer.");
        let found = problems(&config(Insistence::Required), &[missing]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("has no `**Still unknown:**`"),
            "{found:?}"
        );
    }

    #[test]
    fn a_blank_trailer_is_a_problem_too() {
        let blank = entry(1, "Prose.\n\n**Still unknown:**");
        let found = problems(&config(Insistence::Required), &[blank]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("is empty"), "{found:?}");
    }

    #[test]
    fn nothing_is_the_deliberate_act_and_passes() {
        let closed = entry(1, "Prose.\n\n**Still unknown:** nothing");
        let open = entry(2, "Prose.\n\n**Still unknown:** whether it holds.");
        assert!(problems(&config(Insistence::Required), &[closed, open]).is_empty());
    }

    #[test]
    fn a_log_that_predates_the_convention_can_opt_out() {
        let missing = entry(1, "Prose with no trailer.");
        assert!(problems(&config(Insistence::Optional), &[missing]).is_empty());
    }
}
