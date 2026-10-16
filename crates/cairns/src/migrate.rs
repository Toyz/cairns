//! `cairns migrate`: a single-file `WORKLOG.md` split into numbered entries.

use crate::Result;
use crate::project::write_index;
use cairns_core::Config;
use std::path::Path;

/// Convert `from` into entries, every one dated `date` and filed under `area`
/// - the old format carried neither.
pub fn migrate(
    root: &Path,
    config: &Config,
    from: &Path,
    date: Option<String>,
    area: Option<String>,
) -> Result<()> {
    let dir = root.join(&config.paths.entries);

    // Migrating into a directory that already has entries would
    // interleave two numbering schemes. Refuse rather than guess.
    if std::fs::read_dir(&dir).is_ok_and(|mut d| {
        d.any(|e| e.is_ok_and(|e| e.path().extension().is_some_and(|x| x == "md")))
    }) {
        return Err(format!(
            "{} already has entries - migrate into an empty log",
            config.paths.entries
        )
        .into());
    }

    let date = match date {
        Some(given) => given,
        None => jiff::Zoned::now().date().to_string(),
    };
    date.parse::<cairns_core::Date>()?;
    let area = match area {
        Some(given) => given,
        None => config
            .areas
            .first()
            .map(|a| a.name.clone())
            .ok_or("no [area] in cairns.toml to file migrated entries under")?,
    };
    if !config.knows_area(&area) {
        return Err(format!("unknown area {area:?}").into());
    }

    // Resolve before anything is written. `from` arrives as the user
    // typed it, usually relative, and the index path is absolute - a
    // comparison between the two is always false, which is how the
    // first version of this silently overwrote a 9,727-line log.
    let source = std::fs::canonicalize(from).map_err(|e| format!("{}: {e}", from.display()))?;
    let index = root.join(&config.paths.index);
    let overwrites_source = std::fs::canonicalize(&index).ok().as_ref() == Some(&source);

    let text = std::fs::read_to_string(&source)?;
    let entries = split_old_log(&text);
    if entries.is_empty() {
        return Err(format!("{} has no `## ` headings to split on", from.display()).into());
    }
    std::fs::create_dir_all(&dir)?;

    let mut flagged = 0;
    for (number, title, body) in &entries {
        let path = dir.join(format!(
            "{number:04}-{}.md",
            cairns_core::entry::slugify(title)
        ));
        std::fs::write(
            &path,
            format!(
                "---\nnumber: {number}\ntitle: {title}\ndate: {date}\narea: {area}\n\
                 ---\n\n# {number}. {title}\n\n{body}\n"
            ),
        )?;
        if body.contains("## Still not done") || body.contains("## Not done") {
            flagged += 1;
        }
    }

    // The source is about to become the generated index, so it is kept
    // first - and a failure to keep it stops the migration rather than
    // proceeding to destroy it.
    if overwrites_source {
        let backup = source.with_extension("md.bak");
        if backup.exists() {
            return Err(
                format!("{} already exists - move it aside first", backup.display()).into(),
            );
        }
        std::fs::copy(&source, &backup).map_err(|e| format!("could not keep the original: {e}"))?;
        println!("kept the original as {}", backup.display());
    }

    let count = write_index(root, config)?;
    println!("{} entries -> {}", entries.len(), config.paths.entries);
    println!("{count} entries -> {}", config.paths.index);
    println!(
        "every entry is dated {date} and filed under {area:?} - the old format \
         carried neither, so both want correcting by hand"
    );
    if flagged > 0 {
        println!(
            "{flagged} entries have a \"not done\" section that should become a \
             `**Still unknown:**` trailer"
        );
    }
    Ok(())
}

/// Split a single-file worklog on its `## ` headings.
///
/// Returns each entry's number, title and body, with sub-headings demoted one
/// level: in the old format the entry title was an `h2` and its sections were
/// `h3`, and in the new one the title is the `h1`.
fn split_old_log(text: &str) -> Vec<(u32, String, String)> {
    let mut entries = Vec::new();
    let mut current: Option<(u32, String, Vec<&str>)> = None;

    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if let Some((number, title, body)) = current.take() {
                entries.push((number, title, demote(&body)));
            }
            // An unnumbered heading takes the number after the last one.
            let next = entries.last().map_or(1, |(number, _, _)| number + 1);
            let (number, title) = match heading.split_once(". ") {
                Some((digits, rest)) if digits.chars().all(|c| c.is_ascii_digit()) => {
                    (digits.parse().unwrap_or(next), rest.to_string())
                }
                _ => (next, heading.to_string()),
            };
            current = Some((number, title, Vec::new()));
        } else if let Some((_, _, body)) = current.as_mut() {
            body.push(line);
        }
    }
    if let Some((number, title, body)) = current {
        entries.push((number, title, demote(&body)));
    }
    entries
}

fn demote(body: &[&str]) -> String {
    body.iter()
        .map(|line| match line.strip_prefix("###") {
            Some(rest) => format!("##{rest}"),
            None => line.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_old_log_splits_on_its_headings() {
        let old = "# Work log\n\nPreamble.\n\n## 1. Identify the target\n\nProse one.\n\n## 2. The finding\n\nProse two.\n\n### A section\n\nMore.\n";
        let entries = split_old_log(old);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, 1);
        assert_eq!(entries[0].1, "Identify the target");
        assert_eq!(entries[0].2, "Prose one.");
        // The preamble belongs to no entry and is dropped with the old header.
        assert!(!entries[0].2.contains("Preamble"));
        // h3 becomes h2: the entry title is the h1 now.
        assert!(entries[1].2.contains("## A section"));
        assert!(!entries[1].2.contains("### A section"));
    }

    #[test]
    fn an_unnumbered_heading_still_gets_a_number() {
        let entries = split_old_log("## First thing\n\nProse.\n\n## 7. Seventh\n\nMore.\n");
        assert_eq!(entries[0].0, 1);
        assert_eq!(entries[0].1, "First thing");
        assert_eq!(entries[1].0, 7);
    }
}
