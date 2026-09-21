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
    pub fn build(config: &Config, mut entries: Vec<Entry>, generated: Option<String>) -> Log {
        entries.sort_by_key(|entry| entry.front.number);

        let mut corrected: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for entry in &entries {
            for older in &entry.front.supersedes {
                corrected
                    .entry(*older)
                    .or_default()
                    .push(entry.front.number);
            }
        }

        let base = config.site.base_url.trim_end_matches('/');
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        let mut open_questions = Vec::new();
        let mut built = Vec::with_capacity(entries.len());

        for entry in &entries {
            for area in &entry.front.areas {
                *counts.entry(area.as_str()).or_default() += 1;
            }
            let still_unknown = entry.still_unknown();
            if let Some(text) = &still_unknown {
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
        }
    }
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

        for area in &entry.front.areas {
            if !config.knows_area(area) {
                problems.push(format!("{}: unknown area {area:?}", entry.path));
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
    }
    problems
}
