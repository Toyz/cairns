//! The `cairns` command.
//!
//! The command surface is settled here even where the work behind it is not:
//! the names, the flags and the split between `build` and `publish` are what
//! everything else is designed against, so they are worth fixing early and
//! being honest about what is not built yet.

mod mcp;
mod serve;

use cairns_core::config::TargetKind;
use cairns_core::{Config, Entry, FsSource, Log, Source, log};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "cairns",
    version,
    about = "A worklog kept as numbered markdown entries"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start a new entry and print its path.
    New {
        title: String,
        #[arg(long, required = true, value_delimiter = ',')]
        area: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        files: Vec<String>,
        /// Earlier entries this one corrects.
        #[arg(long, value_delimiter = ',')]
        supersedes: Vec<u32>,
        /// Earlier entries whose open question this one answers.
        #[arg(long, value_delimiter = ',')]
        resolves: Vec<u32>,
    },
    /// The number the next entry would take.
    Next,
    /// Regenerate the index from the entries.
    Index,
    /// Numbering sound, front matter complete, index current.
    Check,
    /// What the log still does not know, collected across every entry.
    Open,
    /// Render the payload into a local directory.
    Build {
        #[arg(long, default_value = "site")]
        out: PathBuf,
    },
    /// Write log.json to stdout, or to a file.
    Export {
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Omit the timestamp, so two runs over unchanged entries are identical.
        #[arg(long)]
        reproducible: bool,
    },
    /// Build the site and serve it locally, rebuilding as entries change.
    Serve {
        #[arg(long, default_value_t = 8787)]
        port: u16,
        /// Open it in a browser.
        #[arg(long)]
        open: bool,
    },
    /// Deliver the payload to a configured target.
    Publish {
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Write cairns.toml and the worklog skill into this repo.
    Init,
    /// Serve the worklog over MCP on stdin and stdout, for a host with no shell.
    Mcp {
        /// Allow writing entries. Reading is all that is offered otherwise.
        #[arg(long)]
        write: bool,
    },
    /// Convert a single-file WORKLOG.md into numbered entries.
    Migrate {
        from: PathBuf,
        /// The date every migrated entry gets. The old format carried none, so
        /// there is nothing truer available.
        #[arg(long)]
        date: Option<String>,
        /// The area every migrated entry gets, to be corrected by hand.
        #[arg(long)]
        area: Option<String>,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(problem) => {
            eprintln!("cairns: {problem}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Next => {
            let (root, config) = load()?;
            println!("{}", next_number(&root.join(&config.paths.entries))?);
        }

        Command::Check => {
            let (root, config) = load()?;
            let entries = read_entries(&root, &config)?;
            let docs = read_docs(&root, &config)?;
            let mut problems = log::problems(&config, &entries);
            problems.extend(log::doc_problems(&entries, &docs));

            // A stale index is the most common way a worklog starts lying, and
            // the cheapest to catch: render it again and compare.
            let built = Log::build(&config, entries, None);
            let wanted = cairns_site::render_index(&built, config.index.header.as_deref());
            let index = root.join(&config.paths.index);
            match std::fs::read_to_string(&index) {
                Ok(found) if found == wanted => {}
                Ok(_) => problems.push(format!(
                    "{} is stale - run `cairns index`",
                    config.paths.index
                )),
                Err(_) => problems.push(format!(
                    "{} is missing - run `cairns index`",
                    config.paths.index
                )),
            }

            for problem in &problems {
                eprintln!("{problem}");
            }
            if !problems.is_empty() {
                return Ok(ExitCode::FAILURE);
            }
            println!("ok");
        }

        Command::Index => {
            let (root, config) = load()?;
            let count = write_index(&root, &config)?;
            println!("{count} entries -> {}", config.paths.index);
        }

        Command::Open => {
            let (root, config) = load()?;
            let built = build_log(&root, &config, None)?;
            if built.open_questions.is_empty() {
                println!("nothing open");
            }
            for question in &built.open_questions {
                println!("{:>4}  {}", question.entry, question.text);
            }
        }

        Command::Export { out, reproducible } => {
            let (root, config) = load()?;
            let built = build_log(&root, &config, stamp(reproducible))?;
            let json = serde_json::to_string_pretty(&built)?;
            match out {
                Some(path) => std::fs::write(path, json)?,
                None => println!("{json}"),
            }
        }

        Command::Build { out } => {
            let (root, config) = load()?;
            let built = build_log(&root, &config, stamp(false))?;
            let rendered = cairns_site::render(&built)?;
            for file in &rendered.files {
                let path = out.join(&file.path);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, &file.bytes)?;
            }
            println!("{} entries -> {}", built.entries.len(), out.display());
        }

        Command::New {
            title,
            area,
            files,
            supersedes,
            resolves,
        } => {
            let (root, config) = load()?;
            // `--area "a, b"` and `--area a,b` are the same thing. The front
            // matter parser accepts both spellings because the spec says it
            // must; the command that writes the file has no business being
            // stricter than the one that reads it.
            let area = trimmed(area);
            let files = trimmed(files);
            for name in &area {
                if !config.knows_area(name) {
                    let known: Vec<&str> = config.areas.iter().map(|a| a.name.as_str()).collect();
                    return Err(format!(
                        "unknown area {name:?}; cairns.toml declares {}",
                        known.join(", ")
                    )
                    .into());
                }
            }

            let dir = root.join(&config.paths.entries);
            let number = next_number(&dir)?;
            let slug = cairns_core::entry::slugify(&title);
            let path = dir.join(format!("{number:04}-{slug}.md"));
            if path.exists() {
                return Err(format!("{} already exists", path.display()).into());
            }

            let today = jiff::Zoned::now().date();
            let line = |key: &str, values: &[String]| {
                if values.is_empty() {
                    String::new()
                } else {
                    format!("{key}: {}\n", values.join(", "))
                }
            };
            let numbers = |values: &[u32]| values.iter().map(u32::to_string).collect::<Vec<_>>();
            let links = format!(
                "{}{}{}",
                line("files", &files),
                line("supersedes", &numbers(&supersedes)),
                line("resolves", &numbers(&resolves)),
            );
            std::fs::write(
                &path,
                format!(
                    "---\nnumber: {number}\ntitle: {title}\ndate: {today}\n\
                     area: {}\n{links}---\n\n# {number}. {title}\n\n\n\n\
                     **Still unknown:** \n",
                    area.join(", ")
                ),
            )?;

            // The index is refreshed here so it is never stale between creating
            // an entry and remembering to regenerate it.
            write_index(&root, &config)?;
            println!("{}", path.display());
        }

        Command::Init => {
            let root = std::env::current_dir()?;
            let config_path = root.join("cairns.toml");
            if !config_path.exists() {
                let name = root
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "project".into());
                std::fs::write(&config_path, starter_config(&name))?;
                println!("wrote cairns.toml - edit the areas before writing an entry");
            } else {
                println!("cairns.toml is already here, keeping it");
            }

            let config = Config::parse(&std::fs::read_to_string(&config_path)?)?;
            std::fs::create_dir_all(root.join(&config.paths.entries))?;

            let skill = root.join(".claude/skills/worklog/SKILL.md");
            std::fs::create_dir_all(skill.parent().unwrap())?;
            let existing = std::fs::read_to_string(&skill).ok();
            match existing.as_deref() {
                // A skill that predates cairns is entirely hand-written, and
                // rewriting it would throw away exactly the rules worth keeping.
                // Adoption asks rather than assumes.
                Some(text) if !text.contains(SKILL_MARKER) => {
                    println!(
                        "kept .claude/skills/worklog/SKILL.md - it has no {SKILL_MARKER} marker.\n\
                         Everything below that marker is what init preserves, so put it above \
                         the parts\nthis project wrote and run init again."
                    );
                }
                _ => {
                    let (rendered, preserved) = render_skill(&config, existing.as_deref());
                    std::fs::write(&skill, rendered)?;
                    println!(
                        "wrote .claude/skills/worklog/SKILL.md{}",
                        if preserved {
                            " (project section kept)"
                        } else {
                            ""
                        }
                    );
                }
            }

            let frozen = freeze_slugs(&root, &config)?;
            if frozen > 0 {
                println!(
                    "froze {frozen} slug{} - these entries keep the names they were \
                     published under",
                    if frozen == 1 { "" } else { "s" }
                );
            }

            let count = write_index(&root, &config)?;
            println!("{count} entries -> {}", config.paths.index);
        }

        Command::Serve { port, open } => {
            let (root, config) = load()?;
            serve::serve(root, config, port, open)?;
        }

        Command::Publish { target, dry_run } => {
            let (root, config) = load()?;
            let target = pick_target(&config, target.as_deref())?;
            let built = build_log(&root, &config, stamp(false))?;
            let rendered = cairns_site::render(&built)?;

            match target.kind {
                TargetKind::Dir => {
                    let out = root.join(target.path.as_deref().unwrap_or("site"));
                    let changed = changes(&out, &built);
                    report(&changed, &built);
                    if dry_run {
                        println!("dry run - nothing written to {}", out.display());
                        return Ok(ExitCode::SUCCESS);
                    }
                    for file in &rendered.files {
                        let path = out.join(&file.path);
                        if let Some(parent) = path.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        std::fs::write(&path, &file.bytes)?;
                    }
                    println!("{} files -> {}", rendered.files.len(), out.display());
                }

                TargetKind::Http => {
                    let url = target.url.as_deref().unwrap_or("<no url>");
                    if !dry_run {
                        return Err("the http target is reserved, not implemented - \
                             run it with --dry-run to see the request it specifies"
                            .to_string()
                            .into());
                    }
                    // The whole point of specifying this before building it:
                    // the ingest contract can be read off a real log today.
                    let body = serde_json::to_string_pretty(&built)?;
                    println!("POST {url}");
                    println!("content-type: application/json");
                    println!(
                        "authorization: Bearer $({})",
                        target
                            .token
                            .as_deref()
                            .unwrap_or("$CAIRNS_TOKEN")
                            .trim_start_matches('$')
                    );
                    println!("content-length: {}", body.len());
                    println!();
                    println!("{body}");
                }

                TargetKind::GitBranch => {
                    return Err("the git-branch target is not built yet - \
                                `cairns build` plus the Pages workflow in \
                                docs/spec/publish.md does the same job today"
                        .into());
                }
            }
        }

        Command::Mcp { write } => {
            let (root, config) = load()?;
            mcp::serve(root, config, write)?;
        }

        Command::Migrate { from, date, area } => {
            let (root, config) = load()?;
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
                    .ok_or("no [[area]] in cairns.toml to file migrated entries under")?,
            };
            if !config.knows_area(&area) {
                return Err(format!("unknown area {area:?}").into());
            }

            // Resolve before anything is written. `from` arrives as the user
            // typed it, usually relative, and the index path is absolute - a
            // comparison between the two is always false, which is how the
            // first version of this silently overwrote a 9,727-line log.
            let source =
                std::fs::canonicalize(&from).map_err(|e| format!("{}: {e}", from.display()))?;
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
                    return Err(format!(
                        "{} already exists - move it aside first",
                        backup.display()
                    )
                    .into());
                }
                std::fs::copy(&source, &backup)
                    .map_err(|e| format!("could not keep the original: {e}"))?;
                println!("kept the original as {}", backup.display());
            }

            let count = write_index(&root, &config)?;
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
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// Find `cairns.toml` by walking up from the working directory, so the command
/// works from anywhere in the repo.
fn load() -> Result<(PathBuf, Config), Box<dyn std::error::Error>> {
    let mut dir = std::env::current_dir()?;
    loop {
        let candidate = dir.join("cairns.toml");
        if candidate.is_file() {
            let config = Config::parse(&std::fs::read_to_string(&candidate)?)?;
            return Ok((dir, config));
        }
        if !dir.pop() {
            return Err("no cairns.toml here or above - run `cairns init`".into());
        }
    }
}

/// Build the canonical document, README and all.
///
/// The readme is read here rather than in core, which owns no filesystem, and
/// lands in `log.json` so the renderer still consumes one thing.
fn build_log(
    root: &Path,
    config: &Config,
    generated: Option<String>,
) -> Result<Log, Box<dyn std::error::Error>> {
    let mut built = Log::build_with(
        config,
        read_entries(root, config)?,
        read_docs(root, config)?,
        generated,
    );
    if let Some(path) = &config.site.readme {
        match std::fs::read_to_string(root.join(path)) {
            Ok(text) => built.readme = Some(text),
            // Named but missing is worth saying out loud; the site is still
            // worth building without it.
            Err(problem) => eprintln!("cairns: {path}: {problem}"),
        }
    }
    Ok(built)
}

/// The reference pages, if the project keeps any.
fn read_docs(
    root: &Path,
    config: &Config,
) -> Result<Vec<cairns_core::Doc>, Box<dyn std::error::Error>> {
    let Some(docs) = &config.docs else {
        return Ok(Vec::new());
    };
    if !root.join(&docs.dir).is_dir() {
        eprintln!("cairns: {} is not a directory", docs.dir);
        return Ok(Vec::new());
    }
    let mut pages = Vec::new();
    for raw in FsSource::recursive(root, &docs.dir).entries()? {
        pages.push(cairns_core::Doc::parse(&raw)?);
    }
    Ok(pages)
}

fn read_entries(root: &Path, config: &Config) -> Result<Vec<Entry>, Box<dyn std::error::Error>> {
    let source = FsSource::new(root, &config.paths.entries);
    let mut entries = Vec::new();
    for raw in source.entries()? {
        entries.push(Entry::parse(&raw)?);
    }
    Ok(entries)
}

/// The next number, from the filenames alone - never a read of the entries, so
/// it costs the same on entry 5 and entry 5,000.
fn next_number(dir: &Path) -> Result<u32, Box<dyn std::error::Error>> {
    let mut highest = 0;
    for item in std::fs::read_dir(dir)? {
        let name = item?.file_name().to_string_lossy().to_string();
        let digits: String = name.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(number) = digits.parse::<u32>() {
            highest = highest.max(number);
        }
    }
    Ok(highest + 1)
}

/// Regenerate the index, returning how many entries went into it.
fn write_index(root: &Path, config: &Config) -> Result<usize, Box<dyn std::error::Error>> {
    let built = Log::build(config, read_entries(root, config)?, None);
    std::fs::write(
        root.join(&config.paths.index),
        cairns_site::render_index(&built, config.index.header.as_deref()),
    )?;
    Ok(built.entries.len())
}

fn stamp(reproducible: bool) -> Option<String> {
    if reproducible {
        return None;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default();
    Some(cairns_core::rfc3339(now))
}

/// A starter `cairns.toml`. The areas are a guess and the file says so: a
/// taxonomy nobody edited is a taxonomy nobody believes.
fn starter_config(name: &str) -> String {
    let slug = cairns_core::entry::slugify(name);
    format!(
        r#"spec_version = 1

[project]
name        = "{name}"
slug        = "{slug}"
description = ""

# Edit these. They are the one part of a worklog that has to fit the project,
# and an area nobody chose gets used for everything or for nothing.
[[area]]
name  = "design"
about = "a decision made, with the alternatives rejected"
[[area]]
name  = "build"
about = "how the thing is put together"
[[area]]
name  = "bug"
about = "a fault diagnosed, with the root cause"
[[area]]
name  = "perf"
about = "a measurement taken"
[[area]]
name  = "tooling"
about = "the scripts and helpers around the project"
[[area]]
name  = "test"
about = "harnesses and fixtures"
"#
    )
}

/// Split values, trimmed, with the empties dropped.
fn trimmed(values: Vec<String>) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

/// Everything below this line in a generated skill is the project's own.
const SKILL_MARKER: &str = "<!-- cairns:project -->";

/// The skill, with this project's areas written into it.
///
/// Everything below the project marker in an existing file is kept: that half
/// is hand-written, and regenerating the generic half should never cost it.
fn render_skill(config: &Config, existing: Option<&str>) -> (String, bool) {
    let areas = config
        .areas
        .iter()
        .map(|area| format!("| `{}` | {} |", area.name, area.about))
        .collect::<Vec<_>>()
        .join("\n");
    let first = config
        .areas
        .first()
        .map(|a| a.name.as_str())
        .unwrap_or("design");
    // Every example carries two areas, because the question people ask first is
    // whether an entry may sit in more than one, and prose saying so under a
    // single-area example does not answer it.
    let second = config
        .areas
        .get(1)
        .map(|a| a.name.as_str())
        .unwrap_or(first);

    let rendered = include_str!("../templates/SKILL.md")
        .replace("{{project}}", &config.project.name)
        .replace("{{entries}}", &config.paths.entries)
        .replace("{{index}}", &config.paths.index)
        .replace("{{first_area}}", first)
        .replace("{{areas_typed}}", &format!("{first},{second}"))
        .replace("{{areas_written}}", &format!("{first}, {second}"))
        .replace("{{areas}}", &areas);

    match existing.and_then(|text| text.split_once(SKILL_MARKER)) {
        Some((_, kept)) => {
            let (generic, _) = rendered.split_once(SKILL_MARKER).unwrap_or((&rendered, ""));
            (format!("{generic}{SKILL_MARKER}{kept}"), true)
        }
        None => (rendered, false),
    }
}

/// Pin the slug of every entry whose filename no longer matches its title.
///
/// Adoption has to freeze what exists rather than improve it. The slug is the
/// URL, and a log that has been published already handed those out - so an
/// entry whose name disagrees with its title keeps the name, in writing, and
/// only entries written from here on get the better derivation.
fn freeze_slugs(root: &Path, config: &Config) -> Result<usize, Box<dyn std::error::Error>> {
    let mut frozen = 0;
    for entry in read_entries(root, config)? {
        if entry.front.slug.is_some() || entry.path.ends_with(&entry.filename()) {
            continue;
        }
        let name = entry.path.rsplit('/').next().unwrap_or(&entry.path);
        let Some(published) = name
            .strip_suffix(".md")
            .and_then(|stem| stem.split_once('-'))
            .map(|(_, slug)| slug)
        else {
            continue;
        };

        let path = root.join(&entry.path);
        let text = std::fs::read_to_string(&path)?;
        let Some(anchor) = text.match_indices('\n').map(|(at, _)| at).find(|at| {
            let line = text[..*at].rsplit('\n').next().unwrap_or("");
            line.starts_with("files:") || (line.starts_with("area:") && !text.contains("\nfiles:"))
        }) else {
            continue;
        };

        let mut updated = String::with_capacity(text.len() + 32);
        updated.push_str(&text[..anchor]);
        updated.push_str(&format!("\nslug: {published}"));
        updated.push_str(&text[anchor..]);
        std::fs::write(&path, updated)?;
        frozen += 1;
    }
    Ok(frozen)
}

/// Which target to publish to: the one named, or the only one there is.
fn pick_target<'a>(
    config: &'a Config,
    named: Option<&str>,
) -> Result<&'a cairns_core::config::Target, Box<dyn std::error::Error>> {
    match named {
        Some(name) => config
            .targets
            .iter()
            .find(|target| target.name == name)
            .ok_or_else(|| {
                let known: Vec<&str> = config.targets.iter().map(|t| t.name.as_str()).collect();
                format!(
                    "no publish target {name:?}; cairns.toml declares {}",
                    known.join(", ")
                )
                .into()
            }),
        None => match config.targets.as_slice() {
            [only] => Ok(only),
            [] => Err("no [[publish]] target in cairns.toml".into()),
            many => {
                let known: Vec<&str> = many.iter().map(|t| t.name.as_str()).collect();
                Err(format!("--target is one of {}", known.join(", ")).into())
            }
        },
    }
}

/// What this publish would change at the destination.
///
/// The manifest is whatever `log.json` is already there, so there is no state
/// file in the working tree to go stale and a fresh clone publishes the same as
/// a dirty one. The same comparison serves every target.
fn changes(out: &Path, built: &Log) -> (Vec<u32>, Vec<u32>) {
    let Ok(text) = std::fs::read_to_string(out.join("log.json")) else {
        return (
            built.entries.iter().map(|entry| entry.number).collect(),
            Vec::new(),
        );
    };
    let Ok(there) = serde_json::from_str::<Log>(&text) else {
        return (
            built.entries.iter().map(|entry| entry.number).collect(),
            Vec::new(),
        );
    };

    let known: std::collections::BTreeMap<u32, &str> = there
        .entries
        .iter()
        .map(|entry| (entry.number, entry.content_hash.as_str()))
        .collect();

    let mut added = Vec::new();
    let mut changed = Vec::new();
    for entry in &built.entries {
        match known.get(&entry.number) {
            None => added.push(entry.number),
            Some(hash) if *hash != entry.content_hash => changed.push(entry.number),
            Some(_) => {}
        }
    }
    (added, changed)
}

fn report((added, changed): &(Vec<u32>, Vec<u32>), built: &Log) {
    let same = built.entries.len() - added.len() - changed.len();
    let list = |numbers: &[u32]| {
        numbers
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    };
    if !added.is_empty() {
        println!("new: {}", list(added));
    }
    if !changed.is_empty() {
        println!("changed: {}", list(changed));
    }
    if added.is_empty() && changed.is_empty() {
        println!("nothing changed ({same} entries)");
    } else {
        println!("{same} unchanged");
    }
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
            let (number, title) = match heading.split_once(". ") {
                Some((digits, rest)) if digits.chars().all(|c| c.is_ascii_digit()) => (
                    digits.parse().unwrap_or(entries.len() as u32 + 1),
                    rest.to_string(),
                ),
                _ => (entries.len() as u32 + 1, heading.to_string()),
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

    fn config() -> Config {
        Config::parse(&starter_config("A Project")).expect("the starter config is valid")
    }

    #[test]
    fn the_starter_config_parses_and_names_the_directory() {
        let config = config();
        assert_eq!(config.project.slug, "a-project");
        assert!(!config.areas.is_empty());
    }

    #[test]
    fn a_skill_with_the_marker_keeps_its_project_section() {
        let (first, preserved) = render_skill(&config(), None);
        assert!(!preserved);

        let hand_written = format!("{first}\nEvery claim carries a locator.\n");
        let (again, preserved) = render_skill(&config(), Some(&hand_written));
        assert!(preserved);
        assert!(
            again.ends_with("Every claim carries a locator.\n"),
            "{again}"
        );
    }

    #[test]
    fn the_generic_half_is_regenerated_not_kept() {
        // The project section survives; the half above the marker does not, or
        // a project could never receive a fix to the generic instructions.
        let stale = format!("stale instructions\n{SKILL_MARKER}\nproject rules\n");
        let (rendered, preserved) = render_skill(&config(), Some(&stale));
        assert!(preserved);
        assert!(!rendered.contains("stale instructions"));
        assert!(rendered.contains("project rules"));
        assert!(rendered.contains("name: worklog"));
    }

    #[test]
    fn every_spelling_of_a_multi_area_flag_agrees() {
        // The format accepts `a, b` and `a,b`; so must the command that writes
        // it. Worklog 5 is this bug.
        let want = vec!["decomp".to_string(), "engine".to_string()];
        assert_eq!(trimmed(vec!["decomp,engine".into()]), want);
        assert_eq!(trimmed(vec!["decomp, engine".into()]), want);
        assert_eq!(trimmed(vec![" decomp , engine ".into()]), want);
        assert_eq!(trimmed(vec!["decomp".into(), "engine".into()]), want);
        assert_eq!(trimmed(vec!["decomp,,engine,".into()]), want);
    }

    #[test]
    fn the_examples_in_the_skill_carry_two_areas() {
        let (rendered, _) = render_skill(&config(), None);
        assert!(
            rendered.contains("--area design,build"),
            "typed form missing"
        );
        assert!(
            rendered.contains("area: design, build"),
            "written form missing"
        );
    }

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

    #[test]
    fn the_areas_reach_the_skill() {
        let (rendered, _) = render_skill(&config(), None);
        assert!(
            rendered.contains("| `design` | a decision made, with the alternatives rejected |")
        );
    }
}
