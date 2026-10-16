//! Reading the log from the command line: `ls`, `refs` and `open`.

use crate::Result;
use crate::project::{build_log, read_docs, read_entries};
use cairns_core::{Config, Log};
use std::path::Path;

/// `cairns ls`: the entries a query matches.
pub fn ls(root: &Path, config: &Config, query: &[String]) -> Result<()> {
    let query = cairns_core::query::parse(query)?;
    let built = Log::build_with(
        config,
        read_entries(root, config)?,
        &read_docs(root, config)?,
        None,
    );
    let mut shown = 0;
    for entry in built.entries.iter().filter(|e| query.matches(&built, e)) {
        shown += 1;
        println!(
            "{:>4}  {}  {}  [{}]{}",
            entry.number,
            entry.date,
            entry.title,
            entry.areas.join(", "),
            entry
                .took_minutes
                .map(|m| format!("  {}", cairns_core::entry::format_duration(m)))
                .unwrap_or_default()
        );
    }
    eprintln!("{shown} of {} entries", built.entries.len());
    Ok(())
}

/// `cairns refs`: everything that mentions an entry.
pub fn refs(root: &Path, config: &Config, number: u32) -> Result<()> {
    let built = build_log(root, config, None)?;
    let Some(entry) = built.entry(number) else {
        return Err(format!("no entry {number}").into());
    };
    println!("{number}. {}", entry.title);
    let mut any = false;
    let mut say = |line: String| {
        any = true;
        println!("{line}");
    };
    for other in &entry.referenced_by {
        if let Some(other) = built.entry(*other) {
            say(format!("{}:1: links to it with [[{number}]]", other.path));
        }
    }
    for (key, list) in [
        ("corrects", &entry.superseded_by),
        ("answers", &entry.resolved_by),
        ("carries", &entry.carried_to),
    ] {
        for other in list {
            if let Some(other) = built.entry(*other) {
                say(format!("{}:1: {key} it", other.path));
            }
        }
    }
    for doc in built
        .docs
        .iter()
        .filter(|doc| doc.worklog.contains(&number))
    {
        say(format!("{}:1: cites it as evidence", doc.path));
    }
    for mention in built.mentions.get(&number).into_iter().flatten() {
        say(format!(
            "{}:{}: {}",
            mention.path, mention.line, mention.text
        ));
    }
    if !any {
        println!("nothing mentions it");
    }
    Ok(())
}

/// `cairns open`: every open question, by entry.
pub fn open(root: &Path, config: &Config) -> Result<()> {
    let built = build_log(root, config, None)?;
    if built.open_questions.is_empty() {
        println!("nothing open");
    }
    // A trailer may run to several lines - a list of questions - and
    // each continues under the first, clear of the number column.
    for question in &built.open_questions {
        let mut lines = question.text.lines();
        println!(
            "{:>4}  {}",
            question.entry,
            lines.next().unwrap_or_default()
        );
        for line in lines {
            println!("      {line}");
        }
    }
    Ok(())
}
