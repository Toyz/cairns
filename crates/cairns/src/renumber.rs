//! `cairns renumber`: one entry moved to a new number, after a merge.

use crate::Result;
use crate::check::located;
use crate::project::{next_number, read_docs, read_entries, write_index};
use cairns_core::{Config, Entry};
use std::fmt::Write as _;
use std::path::Path;

/// Move one entry to a new number.
///
/// What belongs to the entry is changed: its `number:`, its `# N.` heading and
/// its file name - the slug, which is its URL, is kept. What does not belong
/// to it is only listed. After a merge, `[[34]]` in another entry may mean the
/// entry that kept 34 or the one that moved, and the text cannot say which;
/// guessing would quietly relink the past.
pub fn renumber(root: &Path, config: &Config, which: &str, to: Option<u32>) -> Result<()> {
    let entries = read_entries(root, config)?;
    let moving =
        match which.parse::<u32>() {
            Ok(number) => {
                let found: Vec<&Entry> = entries
                    .iter()
                    .filter(|e| e.front.number == number)
                    .collect();
                match found.as_slice() {
                    [one] => *one,
                    [] => return Err(format!("no entry {number}").into()),
                    many => return Err(format!(
                        "{} entries are numbered {number} - name the one to move by its path:\n{}",
                        many.len(),
                        many.iter()
                            .map(|e| format!("    {}", e.path))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                    .into()),
                }
            }
            Err(_) => {
                let wanted = which.trim_start_matches("./");
                entries
                    .iter()
                    .find(|e| e.path == wanted || e.path.ends_with(&format!("/{wanted}")))
                    .ok_or_else(|| format!("no entry at {which}"))?
            }
        };
    let old = moving.front.number;
    let dir = root.join(&config.paths.entries);
    let new = match to {
        Some(new) => new,
        None => next_number(&dir)?,
    };
    if entries.iter().any(|e| e.front.number == new) {
        return Err(format!("entry {new} already exists").into());
    }

    let old_path = root.join(&moving.path);
    let text = std::fs::read_to_string(&old_path)?;
    let mut rewritten = String::with_capacity(text.len());
    let mut in_front = false;
    let mut front_done = false;
    let mut heading_done = false;
    for (at, line) in text.split_inclusive('\n').enumerate() {
        let bare = line.trim_end_matches('\n');
        if at == 0 && bare == "---" {
            in_front = true;
        } else if in_front && bare == "---" {
            in_front = false;
            front_done = true;
        } else if in_front && bare.starts_with("number:") {
            let _ = writeln!(rewritten, "number: {new}");
            continue;
        } else if front_done && !heading_done && bare.starts_with(&format!("# {old}. ")) {
            heading_done = true;
            let _ = writeln!(
                rewritten,
                "# {new}. {}",
                &bare[format!("# {old}. ").len()..]
            );
            continue;
        }
        rewritten.push_str(line);
    }
    let name = old_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let rest = name.trim_start_matches(|c: char| c.is_ascii_digit());
    let new_path = old_path.with_file_name(format!("{new:04}{rest}"));
    // Its attachments' folder moves with it, and the entry's links into it.
    let old_stem = name.strip_suffix(".md").unwrap_or(name).to_string();
    let new_stem = format!("{new:04}{}", rest.strip_suffix(".md").unwrap_or(rest));
    let old_dir = old_path.with_file_name(&old_stem);
    if old_dir.is_dir() {
        std::fs::rename(&old_dir, old_path.with_file_name(&new_stem))?;
        rewritten = rewritten.replace(&format!("]({old_stem}/"), &format!("]({new_stem}/"));
    }
    std::fs::write(&new_path, rewritten)?;
    if new_path != old_path {
        std::fs::remove_file(&old_path)?;
    }
    println!(
        "{} -> {} (entry {old} is now {new})",
        moving.path,
        new_path.strip_prefix(root).unwrap_or(&new_path).display()
    );

    // Everything else that names the old number, for a person to check.
    let mut mentions = Vec::new();
    for entry in entries.iter().filter(|e| e.path != moving.path) {
        if cairns_core::entry::references(&entry.body)
            .iter()
            .any(|r| r.workspace.is_none() && r.number == old)
        {
            mentions.push(format!("{}: references [[{old}]]", entry.path));
        }
        for (key, list) in [
            ("resolves", &entry.front.resolves),
            ("carries", &entry.front.carries),
        ] {
            if list.iter().any(|q| q.entry == old) {
                mentions.push(format!("{}: {key} {old}", entry.path));
            }
        }
        if entry.front.supersedes.contains(&old) {
            mentions.push(format!("{}: supersedes {old}", entry.path));
        }
    }
    for doc in read_docs(root, config)? {
        if doc.worklog.contains(&old) {
            mentions.push(format!("{}: worklog {old}", doc.path));
        }
    }
    if !mentions.is_empty() {
        println!(
            "\n{} other {} entry {old}. Each means either the entry that kept {old} or the \
             one now {new}; change the ones that meant this one:",
            mentions.len(),
            if mentions.len() == 1 {
                "place names"
            } else {
                "places name"
            }
        );
        for mention in mentions {
            println!("    {}", located(root, &mention));
        }
    }
    let count = write_index(root, config)?;
    println!("{count} entries -> {}", config.paths.index);
    Ok(())
}
