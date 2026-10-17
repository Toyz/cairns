//! `cairns check`: everything that can be wrong with a log, reported at once,
//! each problem at the line it is about.

use crate::project::{published, read_docs, read_entries, resolve_icon};
use crate::{Result, doc, workspace};
use cairns_core::{Config, Log, log};
use std::path::Path;

/// Run every check; whether the log passed. `fix` regenerates a stale index
/// rather than reporting it.
pub fn run(root: &Path, config: &Config, fix: bool) -> Result<bool> {
    let entries = read_entries(root, config)?;
    let docs = read_docs(root, config)?;
    let mut problems = log::problems(config, &entries);
    problems.extend(log::doc_problems(&entries, &docs));
    problems.extend(doc::problems(root, config, &docs));
    problems.extend(doc::image_problems(root, &entries));
    problems.extend(workspace::problems(root, config, &entries, &docs));

    // An icon name that is not built in renders nothing. That is the
    // right behaviour at build time - a typo should not stop a site -
    // and the wrong behaviour to stay quiet about.
    for link in &config.links {
        let Some(icon) = &link.icon else { continue };
        if cairns_site::icon::is_path(icon) {
            if let Err(problem) = resolve_icon(root, icon) {
                problems.push(format!(
                    "cairns.toml: link {:?} icon {icon:?} {problem}",
                    link.label
                ));
            }
        } else if !cairns_site::icon::is_url(icon) && cairns_site::icon::svg(icon).is_none() {
            problems.push(format!(
                "cairns.toml: link {:?} wants icon {icon:?}, which is not one of: {} \
                 - nor a URL, nor a path to an image in the repository",
                link.label,
                cairns_site::icon::NAMES.join(", ")
            ));
        }
    }

    // A stale index is the most common way a worklog starts lying, and
    // the cheapest to catch: render it again and compare.
    let built = Log::build_with(config, entries, &docs, None);
    let wanted =
        cairns_site::render_index(&built, config.index.header.as_deref(), published(config));
    let index = root.join(&config.paths.index);
    let found = std::fs::read_to_string(&index).ok();
    if found.as_deref() != Some(wanted.as_str()) {
        let why = match &found {
            Some(found) => format!("is stale: {}", stale_because(found, &wanted)),
            None => "is missing".to_string(),
        };
        // The index is derived from the entries and the config and
        // nothing else, so regenerating it can lose nothing. `--fix` does
        // that; plain `check` stays a check, because in CI a stale index
        // means someone forgot a step, and that is worth failing on.
        if fix {
            std::fs::write(&index, &wanted)?;
            println!("regenerated {} - it {why}", config.paths.index);
        } else {
            problems.push(format!(
                "{} {why} - run `cairns index`, or `cairns check --fix`",
                config.paths.index
            ));
        }
    }

    // The reference's generated index pages, the same way.
    for (path, found, wanted) in doc::indexes(root, &built) {
        if found == wanted {
            continue;
        }
        if fix {
            std::fs::write(root.join(&path), &wanted)?;
            println!("regenerated {path} - its listing was stale");
        } else {
            problems.push(format!(
                "{path} is stale: {} - run `cairns index`, or `cairns check --fix`",
                doc::stale_because(&found, &wanted)
            ));
        }
    }

    for problem in &problems {
        eprintln!("{}", located(root, problem));
    }
    if !problems.is_empty() {
        return Ok(false);
    }
    println!("ok");
    Ok(true)
}

/// Why an index differs from the one the log would generate now: its own table
/// of entries, or only what comes before it - the header and description,
/// which live in `cairns.toml`. An index stale from a config edit is expected
/// and harmless, and the message should not read like a problem with entries.
pub fn stale_because(found: &str, wanted: &str) -> String {
    let table = |text: &str| -> Vec<String> {
        text.lines()
            .skip_while(|line| !line.starts_with("| # |"))
            .take_while(|line| line.starts_with('|'))
            .map(str::to_string)
            .collect()
    };
    let (have, want) = (table(found), table(wanted));
    if have == want {
        return "its header or its open questions changed since it was generated".into();
    }
    // Rows by entry number, so a renamed entry is one change, not a row gone
    // and a row added.
    let rows = |lines: &[String]| -> std::collections::BTreeMap<String, String> {
        lines
            .iter()
            .skip(2)
            .filter_map(|row| {
                let number = row.trim_start_matches('|').split('|').next()?.trim();
                Some((number.to_string(), row.clone()))
            })
            .collect()
    };
    let (have, want) = (rows(&have), rows(&want));
    let added = want.keys().filter(|n| !have.contains_key(*n)).count();
    let changed = want
        .iter()
        .filter(|(n, row)| have.get(*n).is_some_and(|old| old != *row))
        .count()
        + have.keys().filter(|n| !want.contains_key(*n)).count();
    let entries = |n: usize| {
        if n == 1 {
            "1 entry is".to_string()
        } else {
            format!("{n} entries are")
        }
    };
    match (added, changed) {
        (0, 0) => "its layout differs from the one this version writes".into(),
        (added, 0) => format!("{} not listed yet", entries(added)),
        (0, changed) => format!("{} not as it lists them", entries(changed)),
        (added, changed) => format!("{} not listed yet and {} changed", entries(added), changed),
    }
}

/// A `check` problem as `path:line: message`, the form editors and terminals
/// jump to - the line the problem is about, as near as the message says: the
/// front matter key it names, the line holding a bad `[[N]]`, the trailer.
/// A problem with no file of its own is left as it is.
pub fn located(root: &Path, problem: &str) -> String {
    let Some((path, message)) = problem.split_once(": ") else {
        return problem.to_string();
    };
    let Ok(text) = std::fs::read_to_string(root.join(path)) else {
        return problem.to_string();
    };
    let lines: Vec<&str> = text.lines().collect();
    let first =
        |pred: &dyn Fn(&str) -> bool| lines.iter().position(|line| pred(line)).map(|at| at + 1);
    let key = |name: &str| {
        let prefix = format!("{name}:");
        first(&|line: &str| line.starts_with(&prefix))
    };
    let named = [
        "number",
        "title",
        "date",
        "area",
        "files",
        "supersedes",
        "resolves",
        "carries",
        "took",
        "worklog",
        "status",
    ]
    .into_iter()
    .find(|name| {
        message.starts_with(&format!("{name} "))
            || message.starts_with(&format!("`{name}`"))
            || message.contains(&format!("unknown {name} "))
            || (*name == "worklog" && message.starts_with("cites worklog"))
    });
    let marker = cairns_core::entry::STILL_UNKNOWN;
    let line = if let Some(name) = named {
        key(name)
    } else if let Some(rest) = message.strip_prefix("references [[") {
        let number = rest.split(']').next().unwrap_or("");
        let needle = format!("[[{number}");
        first(&|line: &str| line.contains(&needle))
    } else if message.contains("is empty") && message.contains(marker) {
        first(&|line: &str| line.starts_with(marker))
    } else if message.contains(marker) {
        Some(lines.len().max(1))
    } else {
        None
    };
    format!("{path}:{}: {message}", line.unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stale_index_says_whether_the_config_or_the_entries_moved() {
        let index = |header: &str, rows: &[&str]| {
            format!(
                "{header}\n\n2 entries: spec 2.\n\n| # | Entry | Date | Area |\n| ---: | --- | --- | --- |\n{}\n",
                rows.join("\n")
            )
        };
        let one = "| 1 | [A](worklog/0001-a.md) | 2026-09-20 | spec |";
        let two = "| 2 | [B](worklog/0002-b.md) | 2026-09-21 | spec |";
        assert_eq!(
            stale_because(&index("# Old", &[one, two]), &index("# New", &[one, two])),
            "its header or its open questions changed since it was generated"
        );
        assert_eq!(
            stale_because(&index("# Log", &[one]), &index("# Log", &[one, two])),
            "1 entry is not listed yet"
        );
        let renamed = "| 2 | [B, renamed](worklog/0002-b.md) | 2026-09-21 | spec |";
        assert_eq!(
            stale_because(
                &index("# Log", &[one, two]),
                &index("# Log", &[one, renamed])
            ),
            "1 entry is not as it lists them"
        );
    }
}
