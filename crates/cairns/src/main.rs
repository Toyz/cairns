//! The `cairns` command.
//!
//! The command surface is settled here even where the work behind it is not:
//! the names, the flags and the split between `build` and `publish` are what
//! everything else is designed against, so they are worth fixing early and
//! being honest about what is not built yet.

mod check;
mod clock;
mod code;
mod doc;
mod git;
mod inspect;
mod license;
mod mcp;
mod migrate;
mod pages;
mod project;
mod publish;
mod refs;
mod renumber;
mod serve;
mod skills;
mod workspace;
mod write;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

/// What every command returns: its errors are said to the user and end it.
pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

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
        action: doc::Command,
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

fn run() -> Result<ExitCode> {
    let command = Cli::parse().command;
    // The one command that makes a project rather than finding one.
    if let Command::Init { hooks } = command {
        skills::init(hooks)?;
        return Ok(ExitCode::SUCCESS);
    }
    let (root, config) = project::load()?;
    match command {
        Command::Init { .. } => unreachable!("handled above"),
        Command::Next => {
            println!(
                "{}",
                project::next_number(&root.join(&config.paths.entries))?
            );
        }
        Command::Check { fix } => {
            if !check::run(&root, &config, fix)? {
                return Ok(ExitCode::FAILURE);
            }
        }
        Command::Index => {
            let count = project::write_index(&root, &config)?;
            println!("{count} entries -> {}", config.paths.index);
        }
        Command::Open => inspect::open(&root, &config)?,
        Command::Ls { query } => inspect::ls(&root, &config, &query)?,
        Command::Refs { number } => inspect::refs(&root, &config, number)?,
        Command::Export { out, reproducible } => {
            publish::export(&root, &config, out.as_deref(), reproducible)?
        }
        Command::Build { out } => publish::build(&root, &config, &out)?,
        Command::Publish { target, dry_run } => {
            publish::publish(&root, &config, target.as_deref(), dry_run)?
        }
        Command::Serve { port, open } => serve::serve(&root, config, port, open)?,
        Command::Mcp { write } => mcp::serve(&root, &config, write)?,
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
            let entry = write::NewEntry {
                title,
                areas: area,
                files,
                supersedes,
                resolves,
                carries,
                body: write::read_body(body)?,
                unknown,
                took,
                attach,
            };
            write::report(&write::write(&root, &config, &entry)?);
        }
        Command::Doc { action } => doc::run(&root, &config, action)?,
        Command::Renumber { entry, to } => renumber::renumber(&root, &config, &entry, to)?,
        Command::Update => skills::update(&root, &config)?,
        Command::Start {
            title,
            cancel,
            keep,
            quiet,
        } => {
            if cancel {
                clock::cancel(&root)?;
            } else if keep && clock::read(&root)?.is_some() {
                // Already running: the work it times is not finished yet.
            } else {
                clock::start(&root, title.as_deref(), quiet)?;
            }
        }
        Command::Clock => clock::show(&root)?,
        Command::Migrate { from, date, area } => {
            migrate::migrate(&root, &config, &from, date, area)?
        }
    }
    Ok(ExitCode::SUCCESS)
}
