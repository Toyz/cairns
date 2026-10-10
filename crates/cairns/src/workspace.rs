//! Other worklogs, named under `[workspace]`.
//!
//! A reference page in one project often rests on findings logged in another -
//! a monorepo with a log per crate, or sibling repositories. `[[piney:361]]`
//! names entry 361 of the worklog called `piney` here. A local project is read
//! so the reference has a title and can be checked; a published one is known
//! only by its URL and linked by number.

use cairns_core::config::WorkspaceTarget;
use cairns_core::entry::{EntryRef, Reference};
use cairns_core::log::{Workspace, WorkspaceEntry};
use cairns_core::{Config, Doc, Entry};
use std::collections::BTreeMap;
use std::path::Path;

/// Every workspace this project names, read as far as it can be.
pub fn load(root: &Path, config: &Config) -> BTreeMap<String, Workspace> {
    config
        .workspace
        .iter()
        .map(|(name, target)| (name.clone(), read(root, target).unwrap_or_default()))
        .collect()
}

/// A workspace: its project when the path is there - titles, slugs, and the
/// URL it says it is published at - else just the URL it was given. Fails
/// only when neither is to be had.
fn read(root: &Path, target: &WorkspaceTarget) -> Result<Workspace, String> {
    let given_url = target
        .url()
        .map(|url| url.trim_end_matches('/').to_string());
    if let Some(path) = target.path() {
        let other = root.join(path);
        match read_project(&other, path) {
            Ok(mut workspace) => {
                if let Some(url) = given_url {
                    workspace.url = url;
                }
                return Ok(workspace);
            }
            // Not checked out here - in CI, say, with only this repository.
            // The URL stands in when there is one.
            Err(why) if given_url.is_none() => return Err(why),
            Err(_) => {}
        }
    }
    let url = given_url.unwrap_or_default();
    Ok(Workspace {
        entries: published(root, &url).unwrap_or_default(),
        url,
    })
}

/// How long a downloaded `ids.json` is used before it is fetched again.
const FRESH_FOR: std::time::Duration = std::time::Duration::from_secs(60 * 60);

/// A published worklog's entries, from the `ids.json` its site publishes -
/// fetched, and kept in `.cairns/workspaces/` for an hour so a rebuild under
/// `serve` does not fetch on every change. `None` when it cannot be had: a
/// site from before `ids.json`, or no network. A stale copy is better than
/// none, so it is used when a fetch fails.
fn published(root: &Path, url: &str) -> Option<BTreeMap<u32, WorkspaceEntry>> {
    if url.is_empty() {
        return None;
    }
    let cache_dir = root.join(".cairns/workspaces");
    let key: String = url
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let cache = cache_dir.join(format!("{key}.json"));
    let fresh = std::fs::metadata(&cache)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|at| at.elapsed().ok())
        .is_some_and(|age| age < FRESH_FOR);
    let text = if fresh {
        std::fs::read_to_string(&cache).ok()
    } else {
        None
    };
    let text = text.or_else(|| match fetch(&format!("{url}/ids.json")) {
        Some(body) => {
            let _ = std::fs::create_dir_all(&cache_dir);
            let _ = std::fs::write(root.join(".cairns/.gitignore"), "*\n");
            let _ = std::fs::write(&cache, &body);
            Some(body)
        }
        None => std::fs::read_to_string(&cache).ok(),
    })?;
    let ids: serde_json::Value = serde_json::from_str(&text).ok()?;
    let entries = ids.get("entries")?.as_object()?;
    Some(
        entries
            .iter()
            .filter_map(|(number, entry)| {
                Some((
                    number.parse().ok()?,
                    WorkspaceEntry {
                        title: entry.get("title")?.as_str()?.to_string(),
                        slug: entry.get("slug")?.as_str()?.to_string(),
                    },
                ))
            })
            .collect(),
    )
}

fn fetch(url: &str) -> Option<String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(5)))
        .build()
        .into();
    agent.get(url).call().ok()?.body_mut().read_to_string().ok()
}

fn read_project(other: &Path, path: &str) -> Result<Workspace, String> {
    let text = std::fs::read_to_string(other.join("cairns.toml"))
        .map_err(|_| format!("no cairns project at {path}"))?;
    let config =
        Config::parse(&text).map_err(|problem| format!("{path}/cairns.toml: {problem}"))?;
    let entries = crate::read_entries(other, &config).map_err(|problem| problem.to_string())?;
    Ok(Workspace {
        url: config.site.base_url.trim_end_matches('/').to_string(),
        entries: entries
            .iter()
            .map(|entry| {
                (
                    entry.front.number,
                    WorkspaceEntry {
                        title: entry.front.title.clone(),
                        slug: entry.slug(),
                    },
                )
            })
            .collect(),
    })
}

/// What `check` says about references into other worklogs: a name not under
/// `[workspace]`, a workspace that cannot be read, an entry a readable one
/// does not have.
pub fn problems(root: &Path, config: &Config, entries: &[Entry], docs: &[Doc]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut loaded: BTreeMap<&str, Option<Workspace>> = BTreeMap::new();
    for (name, target) in &config.workspace {
        match read(root, target) {
            // Checkable when its entries are known - the project read here,
            // or its published ids.json; one known by URL alone, from a site
            // that publishes none or with no network, is linked, not checked.
            Ok(workspace) => {
                let known = !workspace.entries.is_empty();
                loaded.insert(name, known.then_some(workspace));
            }
            Err(why) => {
                problems.push(format!("cairns.toml: [workspace] {name}: {why}"));
                loaded.insert(name, None);
            }
        }
    }
    let mut look = |path: &str, wanted: &EntryRef| match loaded.get(wanted.workspace.as_str()) {
        None => problems.push(format!(
            "{path}: refers to {wanted}, but there is no workspace called {:?} - name it under \
             [workspace] in cairns.toml",
            wanted.workspace
        )),
        Some(Some(workspace)) if !workspace.entries.contains_key(&wanted.number) => problems.push(
            format!("{path}: refers to {wanted}, which does not exist in that worklog"),
        ),
        _ => {}
    };
    let elsewhere = |reference: Reference| {
        reference.workspace.map(|workspace| EntryRef {
            workspace,
            number: reference.number,
        })
    };
    for entry in entries {
        for wanted in cairns_core::entry::references(&entry.body)
            .into_iter()
            .filter_map(elsewhere)
        {
            look(&entry.path, &wanted);
        }
    }
    for doc in docs {
        let mut wanted: Vec<EntryRef> = doc.elsewhere.clone();
        for section in &doc.sections {
            wanted.extend(section.elsewhere.iter().cloned());
        }
        wanted.extend(
            cairns_core::entry::references(&doc.body)
                .into_iter()
                .filter_map(elsewhere),
        );
        for wanted in wanted {
            look(&doc.path, &wanted);
        }
    }
    problems
}
