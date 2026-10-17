//! Writing an entry: what `cairns new` and the MCP server's `worklog_new` both
//! do, in one place.
//!
//! They were two copies once, and drifted the way copies do: the server never
//! wrote `resolves`, so a question closed through it stayed open, and it could
//! not attach a file. Each surface now only turns its own input into a
//! [`NewEntry`] and says what came back.

use crate::project::{next_number, write_index};
use crate::{Result, clock, code, doc};
use cairns_core::Config;
use cairns_core::entry::{QuestionRef, format_duration, parse_duration, slugify};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// An entry to write, as either surface was given it.
#[derive(Debug, Default)]
pub struct NewEntry {
    pub title: String,
    /// Area names; a value may hold several, comma-separated.
    pub areas: Vec<String>,
    /// Paths; a value may hold several, comma-separated.
    pub files: Vec<String>,
    pub supersedes: Vec<u32>,
    /// Questions answered: `54` or `54.2`.
    pub resolves: Vec<String>,
    /// Questions taken over: `54` or `54.2`.
    pub carries: Vec<String>,
    /// The prose. `None` writes a stub to fill in.
    pub body: Option<String>,
    /// The `**Still unknown:**` line, unless the body ends with one.
    pub unknown: Option<String>,
    /// How long it took, in place of the clock: `1h 23m`.
    pub took: Option<String>,
    /// Files to copy into the entry's folder.
    pub attach: Vec<PathBuf>,
}

/// An entry written.
#[derive(Debug)]
pub struct Written {
    pub number: u32,
    pub path: PathBuf,
    /// Minutes, by the clock or as given.
    pub took: Option<u32>,
    /// When the clock that timed it started, if one did.
    pub started: Option<String>,
    /// Worth saying, and not worth refusing the entry over.
    pub warnings: Vec<String>,
}

/// Why an entry was not written: something about it, which whoever asked can
/// fix, or something that went wrong writing it.
#[derive(Debug)]
pub enum Problem {
    Refused(String),
    Failed(Box<dyn std::error::Error>),
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Problem::Refused(why) => f.write_str(why),
            Problem::Failed(problem) => problem.fmt(f),
        }
    }
}

impl std::error::Error for Problem {}

impl From<std::io::Error> for Problem {
    fn from(problem: std::io::Error) -> Self {
        Problem::Failed(problem.into())
    }
}

impl From<Box<dyn std::error::Error>> for Problem {
    fn from(problem: Box<dyn std::error::Error>) -> Self {
        Problem::Failed(problem)
    }
}

fn refused<T>(why: impl Into<String>) -> std::result::Result<T, Problem> {
    Err(Problem::Refused(why.into()))
}

/// Write an entry, refresh the index, and stop the clock that timed it.
///
/// Everything that can refuse it is checked before a number is taken, so a
/// refused entry leaves nothing behind - not a file, not a stopped clock.
pub fn write(
    root: &Path,
    config: &Config,
    entry: &NewEntry,
) -> std::result::Result<Written, Problem> {
    let title = entry.title.trim();
    if title.is_empty() {
        return refused("an entry needs a title");
    }
    // `--area "a, b"` and `--area a,b` are the same thing. The front matter
    // parser accepts both spellings because the spec says it must; the
    // command that writes the file has no business being stricter than the
    // one that reads it.
    let areas = trimmed(&entry.areas);
    let files = trimmed(&entry.files);
    if areas.is_empty() {
        return refused("an entry needs at least one area");
    }
    for name in &areas {
        if !config.knows_area(name) {
            let known: Vec<&str> = config.areas.iter().map(|a| a.name.as_str()).collect();
            // Saying how to add one is half the fix: an area that is a chore
            // to add gets filed under the nearest wrong one.
            return refused(format!(
                "unknown area {name:?}; cairns.toml declares {}\n\
                 to add it, write this under [area] in cairns.toml:\n    \
                 {name} = \"what belongs here\"",
                known.join(", ")
            ));
        }
    }
    // Checked here, so a mistyped `54.x` is refused before a number is taken
    // rather than written into an entry that will not parse.
    let questions = |values: &[String], key: &str| -> std::result::Result<Vec<String>, Problem> {
        trimmed(values)
            .iter()
            .map(|value| {
                value
                    .parse::<QuestionRef>()
                    .map(|question| question.to_string())
                    .map_err(|_| {
                        Problem::Refused(format!(
                            "{key} {value:?}: give an entry (54) or one question of it (54.2)"
                        ))
                    })
            })
            .collect()
    };
    let resolves = questions(&entry.resolves, "resolves")?;
    let carries = questions(&entry.carries, "carries")?;
    let mut prose =
        compose_body(entry.body.as_deref(), entry.unknown.as_deref()).map_err(Problem::Refused)?;
    // The clock is read before anything is written, and stopped only after
    // the entry is, so a refused entry leaves it running.
    let clock = clock::read(root)?;
    let took = match entry
        .took
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
    {
        Some(given) => Some(parse_duration(given).map_err(Problem::Refused)?),
        None => clock.as_ref().map(clock::Clock::minutes),
    };
    for file in &entry.attach {
        if !file.is_file() {
            return refused(format!("attach {}: no such file", file.display()));
        }
    }

    let dir = root.join(&config.paths.entries);
    std::fs::create_dir_all(&dir)?;
    let number = next_number(&dir)?;
    let stem = format!("{number:04}-{}", slugify(title));
    let path = dir.join(format!("{stem}.md"));
    if path.exists() {
        return refused(format!("{} already exists", path.display()));
    }
    // The body linked its attachments by bare name, since the number was not
    // known when it was written; they are about to be in the entry's folder.
    for file in &entry.attach {
        let name = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        prose = prose.replace(&format!("]({name})"), &format!("]({stem}/{name})"));
    }

    let mut front = format!(
        "---\nnumber: {number}\ntitle: {title}\ndate: {}\narea: {}\n",
        jiff::Zoned::now().date(),
        areas.join(", ")
    );
    let numbers: Vec<String> = entry.supersedes.iter().map(u32::to_string).collect();
    for (key, values) in [
        ("files", &files),
        ("supersedes", &numbers),
        ("resolves", &resolves),
        ("carries", &carries),
    ] {
        if !values.is_empty() {
            let _ = writeln!(front, "{key}: {}", values.join(", "));
        }
    }
    if let Some(clock) = &clock {
        let _ = writeln!(front, "started: {}", clock.started);
    }
    if let Some(minutes) = took {
        let _ = writeln!(front, "took: {}", format_duration(minutes));
    }
    std::fs::write(
        &path,
        format!("{front}---\n\n# {number}. {title}\n\n{prose}\n"),
    )?;

    if !entry.attach.is_empty() {
        let folder = dir.join(&stem);
        std::fs::create_dir_all(&folder)?;
        for file in &entry.attach {
            let name = file
                .file_name()
                .ok_or_else(|| Problem::Refused("an attachment needs a file name".into()))?;
            std::fs::copy(file, folder.join(name))?;
        }
    }
    // The index is refreshed here so it is never stale between creating an
    // entry and remembering to regenerate it.
    write_index(root, config)?;

    // A code reference that does not resolve is worth hearing about while the
    // entry can still be fixed. Not refused: code moves, and a log is not
    // wrong for having pointed at where it was.
    let mut warnings: Vec<String> = cairns_core::entry::code_references(&prose)
        .iter()
        .filter_map(|code| {
            code::resolve(root, code, false)
                .missing
                .map(|why| format!("{} - {why}", code.key()))
        })
        .collect();
    // An image that shows nothing, while the entry is still being written:
    // `cairns check` would refuse it at the next commit.
    let written = format!("{}/{stem}.md", config.paths.entries.trim_end_matches('/'));
    warnings.extend(doc::missing_images(root, &written, &prose));
    if clock.is_some() {
        clock::stop(root)?;
        // Wall-clock includes every break. Past a working day it almost
        // certainly does, and the number should not go into the log
        // unremarked.
        if entry.took.is_none() && took.is_some_and(|minutes| minutes > 8 * 60) {
            warnings.push(format!(
                "that is longer than a working day - if it spans a break, correct `took:` in {}",
                path.display()
            ));
        }
    }
    Ok(Written {
        number,
        path,
        took,
        started: clock.map(|clock| clock.started),
        warnings,
    })
}

/// What `cairns new` prints for an entry written: its path first, alone on
/// stdout's first line, for a script to take.
pub fn report(written: &Written) {
    println!("{}", written.path.display());
    for warning in &written.warnings {
        eprintln!("cairns: {warning}");
    }
    if let Some(minutes) = written.took {
        let took = format_duration(minutes);
        match &written.started {
            Some(started) => println!("took {took}, by the clock started {started}"),
            None => println!("took {took}"),
        }
    }
}

/// Everything in an entry after its heading: the prose, then the
/// `**Still unknown:**` line.
///
/// Built here so that whoever writes an entry - usually a model - can hand the
/// prose over on stdin instead of creating a stub and then editing it, which
/// was the step that went wrong. The trailer is the other thing that went
/// wrong, so there is exactly one way for it to arrive and a refusal that says
/// how when there is none: a trailer already in the body, or `--unknown`, never
/// both and never neither. Defaulting to `nothing` would be the silent hole
/// `check` exists to close.
///
/// A leading `# heading` in the body is dropped, because the command writes the
/// entry's own and a second one is the most common thing handed over with it.
pub fn compose_body(body: Option<&str>, unknown: Option<&str>) -> Result<String, String> {
    let unknown = unknown.map(str::trim);
    // `-` means stdin for `--body`, and it is easy to give it here by habit -
    // which wrote an entry whose open question was a lone dash.
    if unknown == Some("-") {
        return Err(
            "--unknown takes the text itself, not `-`: say what is open, or \
                    `nothing`, or put a **Still unknown:** list at the end of the body"
                .into(),
        );
    }
    let Some(body) = body else {
        // The stub: prose to write, the trailer filled if it was given.
        return Ok(format!(
            "\n\n{} {}",
            cairns_core::entry::STILL_UNKNOWN,
            unknown.unwrap_or_default()
        ));
    };

    let body = cairns_core::entry::without_title(body).trim();
    if body.is_empty() {
        return Err("the body is empty".into());
    }

    let marker = cairns_core::entry::STILL_UNKNOWN;
    // "nothing about X" reads as closed and may not be. The entry being
    // written can still say which it means; history cannot, so only new
    // entries are held to it.
    let trailer = unknown.map(str::to_string).or_else(|| {
        body.match_indices(marker)
            .filter(|(at, _)| *at == 0 || body[..*at].ends_with('\n'))
            .last()
            .map(|(at, _)| body[at + marker.len()..].to_string())
    });
    if trailer
        .as_deref()
        .is_some_and(cairns_core::entry::hedged_nothing)
    {
        return Err(
            "the trailer starts with \"nothing\" and then qualifies it, which reads \
                    as closed and may not be. Write `nothing.` if this entry leaves nothing open \
                    (a note can follow the full stop), or say what is still open without the \
                    \"nothing\""
                .into(),
        );
    }
    match (has_trailer(body), unknown) {
        (true, None) => Ok(body.to_string()),
        (true, Some(_)) => Err(format!(
            "the body already has a `{marker}` line - drop it or drop --unknown, not both"
        )),
        (false, Some("")) | (false, None) => Err(format!(
            "say what this entry leaves open: --unknown \"the open question\", or \
             --unknown nothing if it closes out (or end the body with a `{marker}` line)"
        )),
        (false, Some(unknown)) => Ok(format!("{body}\n\n{marker} {unknown}")),
    }
}

/// Whether the text has a `**Still unknown:**` line of its own - at the start
/// of a line, as the spec requires, not merely mentioned in a sentence.
pub fn has_trailer(text: &str) -> bool {
    text.match_indices(cairns_core::entry::STILL_UNKNOWN)
        .any(|(at, _)| at == 0 || text[..at].ends_with('\n'))
}

/// Split values, trimmed, with the empties dropped.
fn trimmed(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

/// A `--body` value: the text itself, or `-` for stdin.
pub fn read_body(body: Option<String>) -> Result<Option<String>> {
    Ok(match body.as_deref() {
        Some("-") => {
            use std::io::IsTerminal;
            if std::io::stdin().is_terminal() {
                eprintln!("reading the body from stdin; end it with ctrl-d");
            }
            Some(std::io::read_to_string(std::io::stdin())?)
        }
        _ => body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dash_for_unknown_is_refused() {
        assert!(compose_body(Some("Prose."), Some("-")).is_err());
    }

    #[test]
    fn a_hedged_nothing_is_refused_for_a_new_entry() {
        assert!(compose_body(Some("Prose."), Some("nothing about the offset")).is_err());
        assert!(
            compose_body(Some("Prose.\n\n**Still unknown:** nothing new here."), None).is_err()
        );
        assert!(
            compose_body(
                Some("Prose."),
                Some("nothing. The test runs on Mutation only.")
            )
            .is_ok()
        );
    }

    #[test]
    fn a_body_takes_its_trailer_from_unknown() {
        assert_eq!(
            compose_body(Some("Prose.\n"), Some("why it hangs")).unwrap(),
            "Prose.\n\n**Still unknown:** why it hangs"
        );
    }

    #[test]
    fn a_body_that_ends_with_a_trailer_keeps_it() {
        let body =
            "Prose that mentions **Still unknown:** in passing.\n\n**Still unknown:** nothing";
        assert_eq!(compose_body(Some(body), None).unwrap(), body);
        // Two trailers is the confusion this exists to prevent.
        assert!(compose_body(Some(body), Some("nothing")).is_err());
    }

    #[test]
    fn a_body_with_no_trailer_at_all_is_refused_with_the_fix() {
        let error = compose_body(
            Some("Prose that mentions **Still unknown:** mid-line."),
            None,
        )
        .unwrap_err();
        assert!(error.contains("--unknown nothing"), "{error}");
        assert!(compose_body(Some("Prose."), Some("  ")).is_err());
    }

    #[test]
    fn a_body_that_repeats_the_heading_loses_it() {
        assert_eq!(
            compose_body(Some("# 4. The title\n\nProse."), Some("nothing")).unwrap(),
            "Prose.\n\n**Still unknown:** nothing"
        );
        assert!(compose_body(Some("# Only a heading"), Some("nothing")).is_err());
    }

    #[test]
    fn without_a_body_the_stub_is_unchanged() {
        assert_eq!(compose_body(None, None).unwrap(), "\n\n**Still unknown:** ");
        assert_eq!(
            compose_body(None, Some("x")).unwrap(),
            "\n\n**Still unknown:** x"
        );
    }

    #[test]
    fn every_spelling_of_a_multi_area_flag_agrees() {
        // The format accepts `a, b` and `a,b`; so must the command that writes
        // it. Worklog 5 is this bug.
        let want = vec!["decomp".to_string(), "engine".to_string()];
        assert_eq!(trimmed(&["decomp,engine".into()]), want);
        assert_eq!(trimmed(&["decomp, engine".into()]), want);
        assert_eq!(trimmed(&[" decomp , engine ".into()]), want);
        assert_eq!(trimmed(&["decomp".into(), "engine".into()]), want);
        assert_eq!(trimmed(&["decomp,,engine,".into()]), want);
    }
}
