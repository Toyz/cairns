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
    /// The reference's sections as `cairns.toml` declares them, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub docs_sections: Vec<crate::config::DocSection>,
    /// The front matter the project's pages carry beyond the built-in keys.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub docs_fields: Vec<crate::config::DocField>,
    /// The project's own links, for the rail.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<crate::config::Link>,
    /// The project's palette overrides, from `[colors]`.
    #[serde(default, skip_serializing_if = "crate::config::Colors::is_empty")]
    pub colors: crate::config::Colors,
    /// The project's own stylesheet, as text. Read by whoever owns the
    /// filesystem, like the README, so the renderer still consumes one thing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stylesheet: Option<String>,
    /// Where the files the site replaces live in the repository, so a link to
    /// one of them can go to its page instead.
    #[serde(default)]
    pub paths: SourcePaths,
    /// Every code reference in the log and the reference, resolved against
    /// the repository by whoever had it - lines found, code read for an
    /// embed - keyed by `CodeRef::key`. Carried here, like the README, so the
    /// renderer reads one thing and a renderer elsewhere needs no checkout.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub code: BTreeMap<String, ResolvedCode>,
    /// Where the repository's own files mention an entry - "worklog 50" in a
    /// comment - by entry number. Found by whoever has the repository.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub mentions: BTreeMap<u32, Vec<CodeMention>>,
    /// The other worklogs this one refers to, by name, as far as they could be
    /// read: where they are published, and - for one that is a local project -
    /// its entries' titles and slugs.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub workspaces: BTreeMap<String, Workspace>,
    /// Entries and pages by number and by path, built on first use. Every
    /// link on every page is resolved against these; looked up by walking the
    /// entries instead, a site's build grew with the square of its length -
    /// 0.5s at 407 entries, 3s at 2,035.
    #[serde(skip)]
    lookup: std::sync::OnceLock<Lookup>,
}

#[derive(Debug, Clone, Default)]
struct Lookup {
    by_number: std::collections::HashMap<u32, usize>,
    by_path: std::collections::HashMap<String, usize>,
    by_folder: std::collections::HashMap<String, usize>,
    doc_by_path: std::collections::HashMap<String, usize>,
}

/// Another worklog, as this one knows it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Workspace {
    /// Where it is published, when that is known: its entries are at
    /// `<url>/<number>/`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    /// Its entries, when it is a project this checkout can read - so a
    /// reference to it carries a title and can be checked. Empty for one known
    /// only by its URL.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub entries: BTreeMap<u32, WorkspaceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    pub title: String,
    pub slug: String,
}

/// A reference page, or a section of one, that names an entry as evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocCitation {
    pub title: String,
    pub slug: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
}

/// A line in the repository that mentions an entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMention {
    pub path: String,
    pub line: u32,
    /// The line itself, trimmed and cut short.
    pub text: String,
}

/// A code reference, resolved.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolvedCode {
    pub path: String,
    /// The lines it spans, 1-based and inclusive, when it names some.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<(u32, u32)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    /// The code itself, for a reference that embeds it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Why it could not be found, when it could not: the file is gone, the
    /// name is not defined in it, the lines are past its end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missing: Option<String>,
}

/// The repository files that have a page of their own on the site. A doc
/// linking `../WORKLOG.md` means the log's index, and on the site that is the
/// entry list, not a file - there is no `WORKLOG.md` there to find.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourcePaths {
    /// The generated index, `WORKLOG.md` by default.
    pub index: String,
    /// The entries directory, `worklog` by default.
    pub entries: String,
    /// The README shown as the About page, if there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readme: Option<String>,
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
    /// Entries in other worklogs that did.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elsewhere: Vec<crate::entry::EntryRef>,
    /// Evidence given section by section.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<crate::doc::SectionEvidence>,
    /// What the page covers, as groups of items.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub covers: Vec<Vec<String>>,
    pub body: String,
    pub content_hash: String,
    /// The day the page last changed, by the repository's history - a page is
    /// edited to stay true, so when it last was is worth knowing. Found by
    /// whoever has the repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed: Option<String>,
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
    /// The code's license, an SPDX expression, declared or found.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// The text's - the log's and the reference's - when it differs from the
    /// code's. `None` means the code's license covers the text too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_license: Option<String>,
    /// The license files in the repository, by path, with the license each
    /// holds when its text says - what a reader is linked to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub license_files: Vec<LicenseFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseFile {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
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
    /// Entries whose whole open question this one answers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolves: Vec<u32>,
    /// Single questions this one answers, as `54.2`: entry 54's second.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolves_questions: Vec<String>,
    /// Entries whose whole open question this one takes over, unanswered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub carries: Vec<u32>,
    /// Single questions this one takes over, as `54.2`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub carries_questions: Vec<String>,
    /// Derived: the entries that answered this one's open question. While this
    /// is non-empty the question is closed and is not in `open_questions`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved_by: Vec<u32>,
    /// Derived: the entries that took this one's question over. It is still
    /// open - there, not here - so it is not in `open_questions` under this
    /// entry, and it is not struck through as answered either.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub carried_to: Vec<u32>,
    /// Derived: a trailer written as a list, question by question, each with
    /// what answered it or took it over. Empty for a trailer that is not a list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<Question>,
    /// Files kept with the entry, in the folder named like it -
    /// `worklog/0050-the-table/` beside `0050-the-table.md` - by name within
    /// it. Listed by whoever has the files; the site copies them next to the
    /// entry's page.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<String>,
    /// Derived: later entries that link to this one with `[[N]]` - the way back
    /// along a link that was only ever written forwards.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub referenced_by: Vec<u32>,
    /// Derived: the pages, and sections of pages, that name this entry as
    /// their evidence - each a link to the place.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documented_in: Vec<DocCitation>,
    /// Derived: reference pages that name this entry as their evidence.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documented_by: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub still_unknown: Option<String>,
    /// When the work began, as the entry recorded it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started: Option<String>,
    /// How long it took, in minutes - wall-clock, see `docs/spec/entry.md`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took_minutes: Option<u32>,
    /// Markdown, not HTML. A newer renderer can re-render an old log, and a
    /// consumer wanting plain text is not unpicking someone else's markup.
    pub body: String,
    pub content_hash: String,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub extra: serde_json::Map<String, Value>,
}

/// One question of a trailer written as a list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    /// Its place in the list, from 1 - what `resolves: 54.2` names.
    pub number: u32,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved_by: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub carried_to: Vec<u32>,
}

impl Question {
    pub fn is_open(&self) -> bool {
        self.resolved_by.is_empty() && self.carried_to.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenQuestion {
    pub entry: u32,
    /// What is still open, as markdown: the trailer, or the part of its list
    /// nothing has answered or taken over.
    pub text: String,
    /// When the trailer is a list: the questions still open, by their number in
    /// it, so they can be shown and named as `54.2` with gaps where others
    /// closed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<Question>,
}

/// The version this build is. A release says its own, stamped from its tag; a
/// build from source is `0.0.0-dev`, unless it was told what `git describe`
/// says - `0.10.1-2-gf958c93`, two commits past 0.10.1 - which is how this
/// repo's own site, built from `main`, says what it was built from.
fn version() -> &'static str {
    match option_env!("CAIRNS_DESCRIBE") {
        Some(described) if !described.is_empty() => described.trim_start_matches('v'),
        _ => env!("CARGO_PKG_VERSION"),
    }
}

/// The references that name a whole entry, as its number.
fn whole_refs(questions: &[crate::entry::QuestionRef]) -> Vec<u32> {
    questions
        .iter()
        .filter(|q| q.item.is_none())
        .map(|q| q.entry)
        .collect()
}

/// The references that name one question, as `54.2`.
fn item_refs(questions: &[crate::entry::QuestionRef]) -> Vec<String> {
    questions
        .iter()
        .filter(|q| q.item.is_some())
        .map(|q| q.to_string())
        .collect()
}

impl Log {
    fn lookup(&self) -> &Lookup {
        self.lookup.get_or_init(|| {
            let mut lookup = Lookup::default();
            for (at, entry) in self.entries.iter().enumerate() {
                lookup.by_number.entry(entry.number).or_insert(at);
                lookup.by_path.insert(entry.path.clone(), at);
                if let Some(stem) = entry.path.strip_suffix(".md") {
                    lookup.by_folder.insert(stem.to_string(), at);
                }
            }
            for (at, doc) in self.docs.iter().enumerate() {
                lookup.doc_by_path.insert(doc.path.clone(), at);
            }
            lookup
        })
    }

    /// The entry with this number.
    pub fn entry(&self, number: u32) -> Option<&LogEntry> {
        self.lookup()
            .by_number
            .get(&number)
            .map(|at| &self.entries[*at])
    }

    /// The entry in this file, by its path from the repository root.
    pub fn entry_at(&self, path: &str) -> Option<&LogEntry> {
        self.lookup().by_path.get(path).map(|at| &self.entries[*at])
    }

    /// The entry whose attachments' folder this is: `worklog/0050-x`.
    pub fn entry_with_folder(&self, folder: &str) -> Option<&LogEntry> {
        self.lookup()
            .by_folder
            .get(folder)
            .map(|at| &self.entries[*at])
    }

    /// The reference page in this file.
    pub fn doc_at(&self, path: &str) -> Option<&DocPage> {
        self.lookup()
            .doc_by_path
            .get(path)
            .map(|at| &self.docs[*at])
    }

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
        let mut linked: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for entry in &entries {
            for reference in crate::entry::references(&entry.body) {
                if reference.workspace.is_some() {
                    continue;
                }
                let from = linked.entry(reference.number).or_default();
                if reference.number != entry.front.number && !from.contains(&entry.front.number) {
                    from.push(entry.front.number);
                }
            }
        }
        // Keyed by question: `(54, None)` is the whole trailer, `(54, Some(2))`
        // its second question.
        let mut answered: BTreeMap<(u32, Option<u32>), Vec<u32>> = BTreeMap::new();
        let mut carried: BTreeMap<(u32, Option<u32>), Vec<u32>> = BTreeMap::new();
        for entry in &entries {
            for older in &entry.front.supersedes {
                corrected
                    .entry(*older)
                    .or_default()
                    .push(entry.front.number);
            }
            for question in &entry.front.resolves {
                answered
                    .entry((question.entry, question.item))
                    .or_default()
                    .push(entry.front.number);
            }
            for question in &entry.front.carries {
                carried
                    .entry((question.entry, question.item))
                    .or_default()
                    .push(entry.front.number);
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
        let mut citations: BTreeMap<u32, Vec<DocCitation>> = BTreeMap::new();
        let pages: Vec<DocPage> = docs
            .iter()
            .map(|doc| {
                let slug = doc.slug(docs_root);
                for number in &doc.worklog {
                    cited.entry(*number).or_default().push(doc.title.clone());
                    citations.entry(*number).or_default().push(DocCitation {
                        title: doc.title.clone(),
                        slug: slug.clone(),
                        section: None,
                        anchor: None,
                    });
                }
                // A section's evidence is evidence for the page too, and the
                // entry it names links to the section itself.
                for section in &doc.sections {
                    for number in &section.worklog {
                        let titles = cited.entry(*number).or_default();
                        if !titles.contains(&doc.title) {
                            titles.push(doc.title.clone());
                        }
                        citations.entry(*number).or_default().push(DocCitation {
                            title: doc.title.clone(),
                            slug: slug.clone(),
                            section: Some(section.heading.clone()),
                            anchor: Some(section.anchor.clone()),
                        });
                    }
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
                    elsewhere: doc.elsewhere.clone(),
                    sections: doc.sections.clone(),
                    covers: doc.covers.clone(),
                    body: doc.body.clone(),
                    content_hash: doc.content_hash.clone(),
                    changed: None,
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
            let number = entry.front.number;
            let still_unknown = entry.still_unknown();
            let whole = |map: &BTreeMap<(u32, Option<u32>), Vec<u32>>| {
                map.get(&(number, None)).cloned().unwrap_or_default()
            };
            let resolved_by = whole(&answered);
            let carried_to = whole(&carried);
            let (preamble, items) = still_unknown
                .as_deref()
                .map(crate::entry::question_items)
                .unwrap_or_default();
            let questions: Vec<Question> = items
                .into_iter()
                .enumerate()
                .map(|(at, text)| {
                    let key = (number, Some(at as u32 + 1));
                    Question {
                        number: at as u32 + 1,
                        text,
                        resolved_by: answered.get(&key).cloned().unwrap_or_default(),
                        carried_to: carried.get(&key).cloned().unwrap_or_default(),
                    }
                })
                .collect();
            // A question answered or taken over is no longer open here. It
            // stays on the entry that asked it, pointing at what closed it or
            // where it went. A list is open while any of its questions is.
            if let Some(text) = &still_unknown
                && resolved_by.is_empty()
                && carried_to.is_empty()
            {
                let open: Vec<Question> =
                    questions.iter().filter(|q| q.is_open()).cloned().collect();
                if questions.is_empty() {
                    open_questions.push(OpenQuestion {
                        entry: number,
                        text: text.clone(),
                        items: Vec::new(),
                    });
                } else if !open.is_empty() {
                    let text = if open.len() == questions.len() {
                        text.clone()
                    } else {
                        let list: Vec<String> =
                            open.iter().map(|q| format!("- {}", q.text)).collect();
                        match preamble.is_empty() {
                            true => list.join("\n"),
                            false => format!("{preamble}\n{}", list.join("\n")),
                        }
                    };
                    open_questions.push(OpenQuestion {
                        entry: number,
                        text,
                        items: open,
                    });
                }
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
                resolves: whole_refs(&entry.front.resolves),
                resolves_questions: item_refs(&entry.front.resolves),
                carries: whole_refs(&entry.front.carries),
                carries_questions: item_refs(&entry.front.carries),
                resolved_by,
                carried_to,
                questions,
                documented_by: cited.get(&entry.front.number).cloned().unwrap_or_default(),
                documented_in: citations
                    .get(&entry.front.number)
                    .cloned()
                    .unwrap_or_default(),
                referenced_by: linked.get(&entry.front.number).cloned().unwrap_or_default(),
                attachments: Vec::new(),
                still_unknown,
                started: entry.front.started.clone(),
                took_minutes: entry.front.took,
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
            generator: format!("cairns {}", version()),
            generated,
            project: ProjectInfo {
                name: config.project.name.clone(),
                slug: config.project.slug.clone(),
                description: config.project.description.clone(),
                base_url: config.site.base_url.clone(),
                repository: config.project.repository.clone(),
                license: config.project.license.clone(),
                text_license: config
                    .project
                    .text_license
                    .clone()
                    .filter(|text| Some(text) != config.project.license.as_ref()),
                license_files: Vec::new(),
            },
            areas,
            entries: built,
            open_questions,
            readme: None,
            docs: pages,
            docs_label: config.docs.as_ref().map(|docs| docs.label.clone()),
            docs_sections: config
                .docs
                .as_ref()
                .map(|docs| docs.sections.clone())
                .unwrap_or_default(),
            docs_fields: config
                .docs
                .as_ref()
                .map(|docs| docs.fields.clone())
                .unwrap_or_default(),
            links: config.links.clone(),
            colors: config.colors.clone(),
            stylesheet: None,
            code: BTreeMap::new(),
            mentions: BTreeMap::new(),
            workspaces: BTreeMap::new(),
            lookup: std::sync::OnceLock::new(),
            paths: SourcePaths {
                index: config.paths.index.clone(),
                entries: config.paths.entries.trim_end_matches('/').to_string(),
                readme: config.site.readme.clone(),
            },
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
        for section in &doc.sections {
            for number in &section.worklog {
                if !numbers.contains(number) {
                    problems.push(format!(
                        "{}: cites worklog {number} under \"{}\", which does not exist",
                        doc.path, section.heading
                    ));
                }
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
            // Two branches each writing the next entry is the usual way here,
            // and the fix has a command.
            problems.push(format!(
                "{}: number {number} is already used by {first} - `cairns renumber {}` \
                 moves this one to the next free number",
                entry.path, entry.path
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
            // Another worklog's entries are checked against that worklog, by
            // whoever can read it.
            if reference.workspace.is_none() && !numbers.contains(&reference.number) {
                problems.push(format!(
                    "{}: references [[{}]], which does not exist",
                    entry.path, reference.number
                ));
            }
        }

        // `resolves` and `carries` both point at an earlier entry's open
        // question, whole or one item of it, and are wrong in the same ways.
        for (key, list) in [
            ("resolves", &entry.front.resolves),
            ("carries", &entry.front.carries),
        ] {
            for question in list {
                let older = question.entry;
                match entries.iter().find(|other| other.front.number == older) {
                    None => problems.push(format!(
                        "{}: {key} {question}, which does not exist",
                        entry.path
                    )),
                    // Pointing at an entry that asked nothing is a sign the
                    // number is wrong, and nothing else would show it.
                    Some(other) => match other.still_unknown() {
                        None => problems.push(format!(
                            "{}: {key} {question}, which left no open question",
                            entry.path
                        )),
                        Some(text) => {
                            let count = crate::entry::question_items(&text).1.len();
                            if let Some(item) = question.item
                                && item as usize > count
                            {
                                problems.push(format!(
                                    "{}: {key} {question}, but entry {older} {}",
                                    entry.path,
                                    match count {
                                        0 => "asks one question, not a list - name it as {older}"
                                            .replace("{older}", &older.to_string()),
                                        1 => "lists 1 question".to_string(),
                                        n => format!("lists {n} questions"),
                                    }
                                ));
                            }
                        }
                    },
                }
                if older >= number {
                    problems.push(format!(
                        "{}: {key} {question}, which is not an earlier entry",
                        entry.path
                    ));
                }
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
    fn one_question_answered_leaves_the_rest_open() {
        let asking = entry(
            1,
            "",
            "Prose.\n\n**Still unknown:**\n- the first\n- the second\n- the third",
        );
        let answering = entry(2, "resolves: 1.2\n", "Prose.\n\n**Still unknown:** nothing");
        let built = Log::build(&config(), vec![asking, answering], None);
        let questions = &built.entries[0].questions;
        assert_eq!(questions.len(), 3);
        assert_eq!(questions[1].resolved_by, vec![2]);
        assert!(built.entries[0].resolved_by.is_empty());
        // Still open, minus the one answered, keeping the others' numbers.
        let open = &built.open_questions[0];
        assert_eq!(
            open.items.iter().map(|q| q.number).collect::<Vec<_>>(),
            vec![1, 3]
        );
        assert_eq!(open.text, "- the first\n- the third");
        assert_eq!(built.entries[1].resolves_questions, vec!["1.2"]);
    }

    #[test]
    fn a_carried_question_leaves_the_open_list_without_being_answered() {
        let asking = entry(1, "", "Prose.\n\n**Still unknown:** whether it holds.");
        let triage = entry(
            2,
            "carries: 1\n",
            "Prose.\n\n**Still unknown:**\n- whether 1's thing holds",
        );
        let built = Log::build(&config(), vec![asking, triage], None);
        assert_eq!(built.entries[0].carried_to, vec![2]);
        assert!(
            built.entries[0].resolved_by.is_empty(),
            "carried is not answered"
        );
        // Open once, where it now lives.
        assert_eq!(built.open_questions.len(), 1);
        assert_eq!(built.open_questions[0].entry, 2);
    }

    #[test]
    fn a_question_number_past_the_list_is_rejected() {
        let asking = entry(1, "", "Prose.\n\n**Still unknown:**\n- one\n- two");
        let past = entry(2, "resolves: 1.3\n", "Prose.\n\n**Still unknown:** nothing");
        let found = problems(&config(), &[asking.clone(), past]);
        assert!(
            found.iter().any(|p| p.contains("lists 2 questions")),
            "{found:?}"
        );
        let single = entry(1, "", "Prose.\n\n**Still unknown:** one thing.");
        let itemised = entry(2, "carries: 1.1\n", "Prose.\n\n**Still unknown:** nothing");
        let found = problems(&config(), &[single, itemised]);
        assert!(
            found
                .iter()
                .any(|p| p.contains("not a list - name it as 1")),
            "{found:?}"
        );
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
