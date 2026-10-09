//! The `cairns` command.
//!
//! The command surface is settled here even where the work behind it is not:
//! the names, the flags and the split between `build` and `publish` are what
//! everything else is designed against, so they are worth fixing early and
//! being honest about what is not built yet.

mod clock;
mod code;
mod doc;
mod git;
mod mcp;
mod refs;
mod serve;

use cairns_core::config::TargetKind;
use cairns_core::{Config, Entry, FsSource, Log, Source, log};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Subcommand)]
enum DocCommand {
    /// Write a new reference page and print its path.
    New {
        title: String,
        /// The section (directory under the docs root) it goes in.
        #[arg(long = "in")]
        section: Option<String>,
        /// How far it can be trusted: solid, partial or guess.
        #[arg(long)]
        status: Option<String>,
        /// The entries that established it.
        #[arg(long, value_delimiter = ',')]
        from: Vec<u32>,
        /// The page, as markdown, or `-` to read it from stdin.
        #[arg(long)]
        body: Option<String>,
    },
    /// Add entries to a page's evidence.
    Cite {
        /// The page: `formats/pod`, `formats/pod.md` or `docs/formats/pod.md`.
        page: String,
        /// Entry numbers.
        #[arg(required = true, value_delimiter = ',')]
        entries: Vec<u32>,
    },
    /// Every page with its status and evidence, and what needs looking at.
    List {
        /// Fail if any page needs looking at - for CI, the way `check` guards
        /// the log.
        #[arg(long)]
        strict: bool,
    },
}

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
        /// Earlier questions this one answers: an entry (`54`) or one question
        /// of its list (`54.2`).
        #[arg(long, value_delimiter = ',')]
        resolves: Vec<String>,
        /// Earlier questions this one takes over unanswered - a triage entry
        /// gathering what is open: `54` or `54.2`.
        #[arg(long, value_delimiter = ',')]
        carries: Vec<String>,
        /// The prose, as markdown, or `-` to read it from stdin. Without it the
        /// entry is a stub to fill in.
        #[arg(long)]
        body: Option<String>,
        /// What is still unknown, or `nothing`. Becomes the entry's
        /// `**Still unknown:**` line, unless the body already ends with one.
        #[arg(long)]
        unknown: Option<String>,
        /// How long the work took - `1h 23m`, `45m` - in place of what the
        /// clock from `cairns start` says.
        #[arg(long)]
        took: Option<String>,
        /// Files to keep with the entry - a screenshot, a capture - copied into
        /// its folder. Link them in the body by name: `![the hold](shot.png)`.
        #[arg(long, value_delimiter = ',')]
        attach: Vec<PathBuf>,
    },
    /// Start the clock on a piece of work; `cairns new` records how long it ran.
    Start {
        /// What the work is, to be reminded of.
        title: Option<String>,
        /// Stop the running clock without recording anything.
        #[arg(long)]
        cancel: bool,
        /// Leave a running clock alone rather than replace it - for a hook
        /// that runs on every prompt.
        #[arg(long)]
        keep: bool,
        /// Print nothing. A hook's output can land in the model's context.
        #[arg(long)]
        quiet: bool,
    },
    /// What the clock is doing.
    Clock,
    /// The reference pages: write one, cite an entry on one, list them all.
    Doc {
        #[command(subcommand)]
        action: DocCommand,
    },
    /// The number the next entry would take.
    Next,
    /// Regenerate the index from the entries.
    Index,
    /// Numbering sound, front matter complete, index current.
    Check {
        /// Regenerate a stale or missing index instead of reporting it.
        /// Everything else is still reported, and still fails.
        #[arg(long)]
        fix: bool,
    },
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
    Init {
        /// Also add a Claude Code hook that starts the clock when a prompt
        /// arrives, so timing does not depend on a model remembering to.
        #[arg(long)]
        hooks: bool,
    },
    /// List the entries a query matches - all of them with none. `:area`,
    /// `has word`, open, closed, superseded, clocked, documented; number, date
    /// and took with lt le gt ge eq ne; and, or, not, `[ ]` to group (quote
    /// the brackets in zsh).
    Ls {
        /// The query, as words: `:battle and open`, `took gt 2h`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        query: Vec<String>,
    },
    /// Where an entry is mentioned: other entries, reference pages, and the
    /// repository's own files - "worklog 50" in a comment.
    Refs {
        /// The entry number.
        number: u32,
    },
    /// Give an entry a new number - for two branches that each wrote the same
    /// one. Fixes the entry; lists every other mention of the old number to
    /// check by hand, since `[[34]]` elsewhere may mean either entry.
    Renumber {
        /// The entry: its path, or its number when only one entry has it.
        entry: String,
        /// The number to give it; the next free one if not given.
        #[arg(long)]
        to: Option<u32>,
    },
    /// Refresh what cairns generates in this repo - the skills, the clock hook
    /// if there is one, the index - after upgrading cairns or editing
    /// cairns.toml. Touches nothing else.
    Update,
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
    // `cairns ls | head` closes the pipe early; a command-line tool should end
    // quietly then, as Unix tools do, not panic. Rust ignores SIGPIPE by
    // default and turns it into a failed print instead.
    #[cfg(unix)]
    // SAFETY: restoring the default disposition of a signal, at start-up,
    // before any thread exists.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
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

        Command::Check { fix } => {
            let (root, config) = load()?;
            let entries = read_entries(&root, &config)?;
            let docs = read_docs(&root, &config)?;
            let mut problems = log::problems(&config, &entries);
            problems.extend(log::doc_problems(&entries, &docs));

            // An icon name that is not built in renders nothing. That is the
            // right behaviour at build time - a typo should not stop a site -
            // and the wrong behaviour to stay quiet about.
            for link in &config.links {
                let Some(icon) = &link.icon else { continue };
                if cairns_site::icon::is_path(icon) {
                    if let Err(problem) = resolve_icon(&root, icon) {
                        problems.push(format!(
                            "cairns.toml: link {:?} icon {icon:?} {problem}",
                            link.label
                        ));
                    }
                } else if !cairns_site::icon::is_url(icon) && cairns_site::icon::svg(icon).is_none()
                {
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
            let built = Log::build(&config, entries, None);
            let wanted = cairns_site::render_index(&built, config.index.header.as_deref());
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

            for problem in &problems {
                eprintln!("{}", located(&root, problem));
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
            let rendered = render_site(&root, &built)?;
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
            carries,
            body,
            unknown,
            took,
            attach,
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
                    // Saying how to add one is half the fix: an area that is a
                    // chore to add gets filed under the nearest wrong one.
                    return Err(format!(
                        "unknown area {name:?}; cairns.toml declares {}\n\
                         to add it, write this under [area] in cairns.toml:\n    \
                         {name} = \"what belongs here\"",
                        known.join(", ")
                    )
                    .into());
                }
            }

            // Read and settled before a number is taken, so a body that is
            // refused leaves nothing behind.
            let body = read_body(body)?;
            let prose = compose_body(body.as_deref(), unknown.as_deref())?;
            // The clock is read before anything is written, and stopped only
            // after the entry is, so a refused entry leaves it running.
            let clock = clock::read(&root)?;
            let took = match took.as_deref() {
                Some(given) => Some(cairns_core::entry::parse_duration(given)?),
                None => clock.as_ref().map(clock::Clock::minutes),
            };

            let dir = root.join(&config.paths.entries);
            let number = next_number(&dir)?;
            let slug = cairns_core::entry::slugify(&title);
            let path = dir.join(format!("{number:04}-{slug}.md"));
            // Attachments are checked before the number is used, and the
            // body's links to them by bare name pointed into the folder they
            // are about to be in - the number was not known when they were
            // written.
            let stem = format!("{number:04}-{slug}");
            for file in &attach {
                if !file.is_file() {
                    return Err(format!("--attach {}: no such file", file.display()).into());
                }
            }
            let mut prose = prose;
            for file in &attach {
                let name = file
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();
                prose = prose.replace(&format!("]({name})"), &format!("]({stem}/{name})"));
            }
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
            // Checked here, so a mistyped `54.x` is refused before a number is
            // taken rather than written into an entry that will not parse.
            let questions = |values: &[String], flag: &str| -> Result<Vec<String>, String> {
                values
                    .iter()
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .map(|value| {
                        value
                            .parse::<cairns_core::entry::QuestionRef>()
                            .map(|q| q.to_string())
                            .map_err(|_| {
                                format!("--{flag} {value:?}: give an entry (54) or one question of it (54.2)")
                            })
                    })
                    .collect()
            };
            let mut links = format!(
                "{}{}{}",
                line("files", &files),
                line("supersedes", &numbers(&supersedes)),
                line("resolves", &questions(&resolves, "resolves")?),
            );
            links.push_str(&line("carries", &questions(&carries, "carries")?));
            if let Some(clock) = &clock {
                links.push_str(&format!("started: {}\n", clock.started));
            }
            if let Some(minutes) = took {
                links.push_str(&format!(
                    "took: {}\n",
                    cairns_core::entry::format_duration(minutes)
                ));
            }
            std::fs::write(
                &path,
                format!(
                    "---\nnumber: {number}\ntitle: {title}\ndate: {today}\n\
                     area: {}\n{links}---\n\n# {number}. {title}\n\n{prose}\n",
                    area.join(", ")
                ),
            )?;

            if !attach.is_empty() {
                let folder = dir.join(&stem);
                std::fs::create_dir_all(&folder)?;
                for file in &attach {
                    let name = file.file_name().ok_or("an attachment needs a file name")?;
                    std::fs::copy(file, folder.join(name))?;
                }
            }
            // The index is refreshed here so it is never stale between creating
            // an entry and remembering to regenerate it.
            write_index(&root, &config)?;
            println!("{}", path.display());
            // A code reference that does not resolve is worth hearing about
            // while the entry can still be fixed. Not refused: code moves, and
            // a log is not wrong for having pointed at where it was.
            for code in cairns_core::entry::code_references(&prose) {
                if let Some(why) = code::resolve(&root, &code, false).missing {
                    eprintln!("cairns: {} - {why}", code.key());
                }
            }
            if let Some(minutes) = took {
                let took = cairns_core::entry::format_duration(minutes);
                match &clock {
                    Some(clock) => {
                        clock::stop(&root)?;
                        println!("took {took}, by the clock started {}", clock.started);
                        // Wall-clock includes every break. Past a working day
                        // it almost certainly does, and the number should not
                        // go into the log unremarked.
                        if minutes > 8 * 60 {
                            eprintln!(
                                "cairns: that is longer than a working day - if it spans a break, \
                                 correct `took:` in {}",
                                path.display()
                            );
                        }
                    }
                    None => println!("took {took}"),
                }
            }
        }

        Command::Ls { query } => {
            let (root, config) = load()?;
            let query = cairns_core::query::parse(&query)?;
            let built = Log::build_with(
                &config,
                read_entries(&root, &config)?,
                read_docs(&root, &config)?,
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
        }

        Command::Refs { number } => {
            let (root, config) = load()?;
            let built = build_log(&root, &config, None)?;
            let Some(entry) = built.entries.iter().find(|e| e.number == number) else {
                return Err(format!("no entry {number}").into());
            };
            println!("{number}. {}", entry.title);
            let mut any = false;
            let mut say = |line: String| {
                any = true;
                println!("{line}");
            };
            for other in &entry.referenced_by {
                if let Some(other) = built.entries.iter().find(|e| e.number == *other) {
                    say(format!("{}:1: links to it with [[{number}]]", other.path));
                }
            }
            for (key, list) in [
                ("corrects", &entry.superseded_by),
                ("answers", &entry.resolved_by),
                ("carries", &entry.carried_to),
            ] {
                for other in list {
                    if let Some(other) = built.entries.iter().find(|e| e.number == *other) {
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
        }

        Command::Renumber { entry, to } => {
            let (root, config) = load()?;
            renumber(&root, &config, &entry, to)?;
        }

        Command::Update => {
            let (root, config) = load()?;
            write_skills(&root, &config)?;
            refresh_clock_hook(&root)?;
            let count = write_index(&root, &config)?;
            println!("{count} entries -> {}", config.paths.index);
        }

        Command::Start {
            title,
            cancel,
            keep,
            quiet,
        } => {
            let (root, _) = load()?;
            if cancel {
                clock::cancel(&root)?;
            } else if keep && clock::read(&root)?.is_some() {
                // Already running: the work it times is not finished yet.
            } else {
                clock::start(&root, title.as_deref(), quiet)?;
            }
        }

        Command::Clock => {
            let (root, _) = load()?;
            clock::show(&root)?;
        }

        Command::Init { hooks } => {
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

            write_skills(&root, &config)?;
            if hooks {
                add_clock_hook(&root)?;
            }

            // Adoption preserves what is there, the same way it freezes slugs.
            // A log written before this convention existed should not fail
            // `check` on its history; entries written from here on will.
            let entries = read_entries(&root, &config)?;
            let silent = entries
                .iter()
                .filter(|entry| {
                    matches!(
                        entry.trailer(),
                        cairns_core::entry::Trailer::Missing | cairns_core::entry::Trailer::Blank
                    )
                })
                .count();
            let raw = std::fs::read_to_string(&config_path)?;
            if silent > 0 && !raw.contains("[check]") {
                std::fs::write(
                    &config_path,
                    format!(
                        "{}\n# {silent} entries predate the open-question line. Set this back to\n                         # \"required\" once they carry one - `cairns open` is only as complete as\n                         # the entries that answer it.\n[check]\nopen_questions = \"optional\"\n",
                        raw.trim_end()
                    ),
                )?;
                println!(
                    "{silent} entries have no `**Still unknown:**` line - \
                     check relaxed to optional in cairns.toml"
                );
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
            let rendered = render_site(&root, &built)?;

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

        Command::Doc { action } => {
            let (root, config) = load()?;
            match action {
                DocCommand::New {
                    title,
                    section,
                    status,
                    from,
                    body,
                } => {
                    let body = read_body(body)?;
                    doc::new(
                        &root,
                        &config,
                        &title,
                        section.as_deref(),
                        status.as_deref(),
                        &from,
                        body,
                    )?;
                }
                DocCommand::Cite { page, entries } => doc::cite(&root, &config, &page, &entries)?,
                DocCommand::List { strict } => {
                    let flagged = doc::list(&root, &config)?;
                    if strict && flagged > 0 {
                        return Err(format!(
                            "{flagged} {} to look at",
                            if flagged == 1 { "page" } else { "pages" }
                        )
                        .into());
                    }
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
                    .ok_or("no [area] in cairns.toml to file migrated entries under")?,
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
/// `cairns.toml` at a root already found, read again - for `serve`, which
/// watches it.
fn load_config(root: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    Ok(Config::parse(&std::fs::read_to_string(
        root.join("cairns.toml"),
    )?)?)
}

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
    // A link icon that names a file in the repository is read here, like the
    // README, and carried in the document as the markup or data it resolves
    // to - the renderer reads no files, and a hosted one has none to read.
    for link in &mut built.links {
        if let Some(icon) = link.icon.as_deref()
            && cairns_site::icon::is_path(icon)
        {
            link.icon = match resolve_icon(root, icon) {
                Ok(resolved) => Some(resolved),
                Err(problem) => {
                    eprintln!("cairns: link {:?} icon {icon:?} {problem}", link.label);
                    None
                }
            };
        }
    }
    // Code references, resolved against this checkout once for the whole log,
    // and the places the code mentions the log back.
    built.code = code::resolve_all(root, &built);
    built.mentions = refs::scan(root, config);
    for entry in &mut built.entries {
        entry.attachments = attachments(root, &entry.path);
    }
    // A stylesheet named and missing is an error, not a warning: the site would
    // build, look wrong, and say nothing about why.
    if let Some(path) = &config.site.stylesheet {
        built.stylesheet = Some(
            std::fs::read_to_string(root.join(path))
                .map_err(|problem| format!("{path}: {problem}"))?,
        );
    }
    Ok(built)
}

/// A repository image as an icon: an SVG's own markup, so it takes the page's
/// colours, or anything else as a `data:` URL, so the page still makes no
/// request for it.
fn resolve_icon(root: &Path, path: &str) -> Result<String, String> {
    let full = root.join(path.trim());
    let bytes = std::fs::read(&full).map_err(|problem| format!("cannot be read: {problem}"))?;
    let extension = full
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let mime = match extension.as_str() {
        "svg" => {
            let text = String::from_utf8(bytes).map_err(|_| "is not UTF-8 text".to_string())?;
            return cairns_site::icon::clean_svg(&text);
        }
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        other => {
            return Err(format!(
                "is a .{other} file; an icon is .svg, .png, .jpg, .gif, .webp or .ico"
            ));
        }
    };
    // Icons are small; one that is not is a mistake worth saying out loud,
    // since it is copied into every page.
    if bytes.len() > 64 * 1024 {
        return Err(format!(
            "is {} KB; an icon copied into every page should be under 64",
            bytes.len() / 1024
        ));
    }
    Ok(format!("data:{mime};base64,{}", base64(&bytes)))
}

/// Standard base64, for the one `data:` URL this tool makes. Ten lines, where
/// a crate would be a dependency for ten lines.
fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
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
[area]
design  = "a decision made, with the alternatives rejected"
build   = "how the thing is put together"
bug     = "a fault diagnosed, with the root cause"
perf    = "a measurement taken"
tooling = "the scripts and helpers around the project"
test    = "harnesses and fixtures"
"#
    )
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

    let mut body = body.trim();
    if body.starts_with("# ") {
        body = body
            .split_once('\n')
            .map(|(_, rest)| rest.trim())
            .unwrap_or("");
    }
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
fn trimmed(values: Vec<String>) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

/// Why an index differs from the one the log would generate now: its own table
/// of entries, or only what comes before it - the header and description,
/// which live in `cairns.toml`. An index stale from a config edit is expected
/// and harmless, and the message should not read like a problem with entries.
fn stale_because(found: &str, wanted: &str) -> String {
    let table = |text: &str| -> Vec<String> {
        text.lines()
            .skip_while(|line| !line.starts_with("| # |"))
            .map(str::to_string)
            .collect()
    };
    let (have, want) = (table(found), table(wanted));
    if have == want {
        return "its header changed in cairns.toml since it was generated".into();
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

/// The hook command: start a clock when a prompt arrives, unless one is
/// already running, silently, and never fail the prompt - a machine without
/// `cairns` on its path should lose the timing, not the conversation.
const CLOCK_HOOK: &str = "cairns start --keep --quiet 2>/dev/null || true";

/// Add the clock hook to `.claude/settings.json`, keeping everything else in
/// it. Running it twice adds it once.
///
/// A hook rather than an instruction, because an instruction depends on a
/// model following it and a hook does not. On every prompt it starts a clock
/// if none is running; `cairns new` stops it. So `took:` becomes the time from
/// the first prompt after the last entry to this one - wall-clock, as ever.
fn add_clock_hook(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = root.join(".claude/settings.json");
    let mut settings: serde_json::Value = match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|problem| format!("{}: not JSON ({problem}); left alone", path.display()))?,
        Err(_) => serde_json::json!({}),
    };
    let Some(object) = settings.as_object_mut() else {
        return Err(format!("{}: not a JSON object; left alone", path.display()).into());
    };
    let hooks = object
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| format!("{}: \"hooks\" is not an object; left alone", path.display()))?;
    let on_prompt = hooks
        .entry("UserPromptSubmit")
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()
        .ok_or_else(|| {
            format!(
                "{}: \"UserPromptSubmit\" is not a list; left alone",
                path.display()
            )
        })?;
    let present = on_prompt.iter().any(|group| {
        group["hooks"].as_array().is_some_and(|list| {
            list.iter().any(|hook| {
                hook["command"]
                    .as_str()
                    .is_some_and(|command| command.contains("cairns start"))
            })
        })
    });
    if present {
        println!("the clock hook is already in .claude/settings.json");
        return Ok(());
    }
    on_prompt.push(serde_json::json!({
        "hooks": [{ "type": "command", "command": CLOCK_HOOK }]
    }));
    std::fs::create_dir_all(path.parent().expect("settings has a directory"))?;
    std::fs::write(&path, serde_json::to_string_pretty(&settings)? + "\n")?;
    println!(
        "added the clock hook to .claude/settings.json - every prompt starts a clock if none is running"
    );
    Ok(())
}

/// The files in an entry's folder - `worklog/0050-the-table/` beside
/// `0050-the-table.md` - by name within it, sub-folders included.
fn attachments(root: &Path, entry_path: &str) -> Vec<String> {
    let Some(stem) = entry_path.strip_suffix(".md") else {
        return Vec::new();
    };
    fn walk(dir: &Path, prefix: &str, found: &mut Vec<String>) {
        let Ok(items) = std::fs::read_dir(dir) else {
            return;
        };
        for item in items.flatten() {
            let name = item.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let path = item.path();
            let inside = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            if path.is_dir() {
                walk(&path, &inside, found);
            } else {
                found.push(inside);
            }
        }
    }
    let mut found = Vec::new();
    walk(&root.join(stem), "", &mut found);
    found.sort();
    found
}

/// The site, with each entry's attachments copied in beside its page. The
/// renderer reads no files, so the bytes are added here.
fn render_site(
    root: &Path,
    built: &Log,
) -> Result<cairns_site::Rendered, Box<dyn std::error::Error>> {
    let mut rendered = cairns_site::render(built)?;
    for entry in &built.entries {
        let Some(stem) = entry.path.strip_suffix(".md") else {
            continue;
        };
        for name in &entry.attachments {
            rendered.files.push(cairns_site::OutputFile {
                path: format!("{}-{}/{name}", entry.number, entry.slug),
                bytes: std::fs::read(root.join(stem).join(name))?,
            });
        }
    }
    Ok(rendered)
}

/// Move one entry to a new number.
///
/// What belongs to the entry is changed: its `number:`, its `# N.` heading and
/// its file name - the slug, which is its URL, is kept. What does not belong
/// to it is only listed. After a merge, `[[34]]` in another entry may mean the
/// entry that kept 34 or the one that moved, and the text cannot say which;
/// guessing would quietly relink the past.
fn renumber(
    root: &Path,
    config: &Config,
    which: &str,
    to: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
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
            rewritten.push_str(&format!("number: {new}\n"));
            continue;
        } else if front_done && !heading_done && bare.starts_with(&format!("# {old}. ")) {
            heading_done = true;
            rewritten.push_str(&format!(
                "# {new}. {}\n",
                &bare[format!("# {old}. ").len()..]
            ));
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
            .any(|r| r.number == old)
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

/// A `check` problem as `path:line: message`, the form editors and terminals
/// jump to - the line the problem is about, as near as the message says: the
/// front matter key it names, the line holding a bad `[[N]]`, the trailer.
/// A problem with no file of its own is left as it is.
fn located(root: &Path, problem: &str) -> String {
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

/// A `--body` value: the text itself, or `-` for stdin.
fn read_body(body: Option<String>) -> Result<Option<String>, Box<dyn std::error::Error>> {
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

/// Write `.claude/skills/<name>/SKILL.md`, keeping its hand-written half.
///
/// A skill that predates cairns is entirely hand-written, and rewriting it
/// would throw away exactly the rules worth keeping, so one without the marker
/// is left alone. Adoption asks rather than assumes.
fn write_skill(
    root: &Path,
    name: &str,
    render: impl FnOnce(Option<&str>) -> (String, bool),
) -> Result<(), Box<dyn std::error::Error>> {
    let skill = root.join(format!(".claude/skills/{name}/SKILL.md"));
    std::fs::create_dir_all(skill.parent().unwrap())?;
    let existing = std::fs::read_to_string(&skill).ok();
    match existing.as_deref() {
        Some(text) if !text.contains(SKILL_MARKER) => {
            println!(
                "kept .claude/skills/{name}/SKILL.md - it has no {SKILL_MARKER} marker.\n\
                 Everything below that marker is what init preserves, so put it above \
                 the parts\nthis project wrote and run init again."
            );
        }
        // Said, rather than rewritten silently: after an upgrade the question
        // is what changed.
        _ => {
            let (rendered, preserved) = render(existing.as_deref());
            let state = match existing.as_deref() {
                Some(old) if old == rendered => "unchanged",
                Some(_) => "updated",
                None => "wrote",
            };
            if state != "unchanged" {
                std::fs::write(&skill, &rendered)?;
            }
            println!(
                "{state} .claude/skills/{name}/SKILL.md{}",
                if preserved && state != "unchanged" {
                    " (project section kept)"
                } else {
                    ""
                }
            );
        }
    }
    Ok(())
}

/// Every skill this project should have, written or refreshed.
fn write_skills(root: &Path, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    write_skill(root, "worklog", |existing| render_skill(config, existing))?;
    // The clock is a skill of its own because skills are picked by their
    // description, and the worklog skill's says "when work finishes" - matched
    // at the end, too late to start a clock. This one's says "when work
    // begins".
    write_skill(root, "clock", |existing| {
        keep_project_section(
            include_str!("../templates/CLOCK.md").replace("{{project}}", &config.project.name),
            existing,
        )
    })?;
    // The reference has a skill of its own, because writing a page is a
    // different job from writing an entry - and only where there is a
    // reference to write.
    if config.docs.is_some() {
        write_skill(root, "reference", |existing| {
            render_reference_skill(config, existing)
        })?;
    }
    Ok(())
}

/// Bring an existing clock hook up to the command this version writes. Adds
/// nothing: a project that never asked for the hook does not get one from an
/// update.
fn refresh_clock_hook(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = root.join(".claude/settings.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(());
    };
    let Ok(mut settings) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Ok(());
    };
    let mut changed = false;
    let mut found = false;
    if let Some(groups) = settings["hooks"]["UserPromptSubmit"].as_array_mut() {
        for group in groups {
            if let Some(hooks) = group["hooks"].as_array_mut() {
                for hook in hooks {
                    if hook["command"]
                        .as_str()
                        .is_some_and(|c| c.contains("cairns start"))
                    {
                        found = true;
                        if hook["command"] != CLOCK_HOOK {
                            hook["command"] = serde_json::Value::from(CLOCK_HOOK);
                            changed = true;
                        }
                    }
                }
            }
        }
    }
    if changed {
        std::fs::write(&path, serde_json::to_string_pretty(&settings)? + "\n")?;
        println!("updated the clock hook in .claude/settings.json");
    } else if found {
        println!("unchanged the clock hook in .claude/settings.json");
    }
    Ok(())
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
        .replace("{{areas}}", &areas)
        .replace("{{docs_section}}", &docs_section(config));

    keep_project_section(rendered, existing)
}

/// The worklog skill's part about the reference, for a project that keeps one.
/// A project without `[docs]` gets nothing, rather than instructions for pages
/// it does not have.
fn docs_section(config: &Config) -> String {
    let Some(docs) = &config.docs else {
        return String::new();
    };
    format!(
        "## The reference\n\n\
         This project also keeps reference pages under `{dir}/` - what is true now, for\n\
         someone who wants to use it rather than read how it was found. When an entry\n\
         establishes something a reader would look up - a layout, a table, a rule -\n\
         write or correct the page too (the `reference` skill says how), and cite the\n\
         entry on it:\n\n\
         ```sh\n\
         cairns doc cite formats/the-archive 31\n\
         ```\n\n\
         The entry is the evidence and the page is the result. Neither replaces the\n\
         other: an entry is never edited, a page always is.\n\n",
        dir = docs.dir
    )
}

/// The reference skill, for a project with `[docs]`.
fn render_reference_skill(config: &Config, existing: Option<&str>) -> (String, bool) {
    let dir = config
        .docs
        .as_ref()
        .map(|docs| docs.dir.as_str())
        .unwrap_or("docs");
    let rendered = include_str!("../templates/REFERENCE.md")
        .replace("{{project}}", &config.project.name)
        .replace("{{docs}}", dir);
    keep_project_section(rendered, existing)
}

/// A freshly rendered skill, with the hand-written half of an existing one -
/// everything below the marker - carried over.
fn keep_project_section(rendered: String, existing: Option<&str>) -> (String, bool) {
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
            "its header changed in cairns.toml since it was generated"
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

    #[test]
    fn the_clock_has_a_skill_that_triggers_at_the_start() {
        let skill = include_str!("../templates/CLOCK.md");
        let description = skill
            .lines()
            .find(|l| l.starts_with("description:"))
            .unwrap();
        // Picked by its description: it has to say "beginning", not "finished".
        assert!(description.contains("when beginning"), "{description}");
        assert!(skill.contains(SKILL_MARKER));
        let (kept, preserved) = keep_project_section(
            skill.to_string(),
            Some(&format!(
                "old\n{SKILL_MARKER}\n## This project\n\nTime the decoding only.\n"
            )),
        );
        assert!(preserved && kept.ends_with("Time the decoding only.\n"));
    }

    #[test]
    fn base64_matches_the_standard_vectors() {
        for (input, encoded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(input.as_bytes()), encoded);
        }
    }

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
