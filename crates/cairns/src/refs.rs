//! Where the repository mentions an entry.
//!
//! The log points at code with `[[src/a.rs#name]]`; code points back at the log
//! in comments and messages - "see worklog 50", "(worklog 361)". Finding those
//! closes the loop: an entry's page can say where the code still mentions it,
//! and `cairns refs 50` can list them, the way `tatr ref` greps for a task.

use cairns_core::log::CodeMention;
use std::collections::BTreeMap;
use std::path::Path;

/// Every mention of an entry by number across the repository's own files -
/// tracked by git when it is one, walked otherwise - leaving out the log, its
/// index and the reference, whose links the log already knows.
pub fn scan(root: &Path, config: &cairns_core::Config) -> BTreeMap<u32, Vec<CodeMention>> {
    let entries_dir = config.paths.entries.trim_end_matches('/').to_string();
    let skip = |path: &str| {
        path.starts_with(&format!("{entries_dir}/"))
            || path == config.paths.index
            || config
                .docs
                .as_ref()
                .is_some_and(|docs| path.starts_with(&format!("{}/", docs.dir)))
            || path.starts_with(".cairns/")
            || path.starts_with(".claude/")
    };
    let mut found: BTreeMap<u32, Vec<CodeMention>> = BTreeMap::new();
    for path in files(root) {
        if skip(&path) {
            continue;
        }
        let Ok(bytes) = std::fs::read(root.join(&path)) else {
            continue;
        };
        // Text only, and nothing large enough to be data rather than code.
        if bytes.len() > 512 * 1024 || bytes.contains(&0) {
            continue;
        }
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        for (at, line) in text.lines().enumerate() {
            // `worklog: 12, 15` at the start of a line is front matter - an
            // example of a reference page's, in a template or a spec - not a
            // mention. Real pages are read as pages.
            if line.trim_start().starts_with("worklog:") {
                continue;
            }
            for number in mentions(line, &entries_dir) {
                found.entry(number).or_default().push(CodeMention {
                    path: path.clone(),
                    line: at as u32 + 1,
                    text: line.trim().chars().take(160).collect(),
                });
            }
        }
    }
    found
}

/// The repository's files: the ones git tracks, when it is a git repository,
/// so ignored files - build output, dependencies - are not read; a walk of the
/// tree otherwise. Read from the index in-process, with no `git` to run.
fn files(root: &Path) -> Vec<String> {
    match crate::git::tracked(root) {
        Some(tracked) => tracked,
        None => {
            let mut found = Vec::new();
            walk(root, root, &mut found);
            found
        }
    }
}

fn walk(root: &Path, dir: &Path, found: &mut Vec<String>) {
    let Ok(items) = std::fs::read_dir(dir) else {
        return;
    };
    for item in items.flatten() {
        let path = item.path();
        let name = item.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || matches!(name.as_str(), "target" | "node_modules" | "site") {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, found);
        } else if let Ok(inside) = path.strip_prefix(root) {
            found.push(inside.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// The entry numbers a line mentions: `worklog 50`, `worklog #50`,
/// `worklog: 50`, any case, or a path into the entries directory,
/// `worklog/0050-...`.
pub fn mentions(line: &str, entries_dir: &str) -> Vec<u32> {
    let lower = line.to_ascii_lowercase();
    let mut numbers = Vec::new();
    for word in ["worklog", entries_dir] {
        let word = word.to_ascii_lowercase();
        for (at, _) in lower.match_indices(&word) {
            // A whole word: not the end of `myworklog`.
            if lower[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
            {
                continue;
            }
            let rest = &lower[at + word.len()..];
            let rest = rest.trim_start_matches([' ', '#', ':', '/']);
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            let after = rest[digits.len()..].chars().next();
            if !digits.is_empty()
                && !after.is_some_and(|c| c.is_alphanumeric() && c != '-')
                && let Ok(number) = digits.parse::<u32>()
                && number > 0
                && !numbers.contains(&number)
            {
                numbers.push(number);
            }
        }
    }
    numbers
}

#[cfg(test)]
mod tests {
    use super::mentions;

    #[test]
    fn a_line_mentions_an_entry_by_number_or_by_path() {
        assert_eq!(mentions("// see worklog 50 for why", "worklog"), vec![50]);
        assert_eq!(mentions("fix the hold (worklog 361)", "worklog"), vec![361]);
        assert_eq!(
            mentions("# Worklog #12, and worklog: 7", "worklog"),
            vec![12, 7]
        );
        assert_eq!(
            mentions("[x](worklog/0050-the-table.md)", "worklog"),
            vec![50]
        );
        assert_eq!(mentions("see log/0009-x.md", "log"), vec![9]);
        assert!(mentions("myworklog 5", "worklog").is_empty());
        assert!(mentions("the worklog shows", "worklog").is_empty());
        assert!(mentions("worklog 12px", "worklog").is_empty());
    }
}
