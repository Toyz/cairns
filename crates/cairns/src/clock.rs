//! The clock behind `took:`.
//!
//! A model writing an entry has no sense of how long the work took - asked, it
//! will give a confident number that is wrong. So the tool keeps the time:
//! `cairns start` stamps it, `cairns new` reads it back and records the
//! difference. What that measures is wall-clock, start to entry, which is what
//! the field is documented as and all it claims to be.
//!
//! The stamp lives in `.cairns/clock` at the repository root. The directory
//! ignores itself, so a running clock is never committed by accident and no
//! project's own `.gitignore` has to learn about it.

use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// A clock that is running.
pub struct Clock {
    /// What the clock said at the start, local, with its offset - what goes in
    /// the entry's `started:`.
    pub started: String,
    /// The same moment as seconds since the epoch, for the arithmetic.
    pub seconds: i64,
    /// What the work was, if `start` was told.
    pub title: Option<String>,
}

impl Clock {
    /// Minutes since the clock started, at least one: an entry clocked at all
    /// took some time.
    pub fn minutes(&self) -> u32 {
        let elapsed = jiff::Timestamp::now().as_second() - self.seconds;
        u32::try_from((elapsed + 30) / 60).unwrap_or(0).max(1)
    }
}

fn path(root: &Path) -> PathBuf {
    root.join(".cairns/clock")
}

/// The running clock, if there is one.
pub fn read(root: &Path) -> Result<Option<Clock>> {
    let Ok(text) = std::fs::read_to_string(path(root)) else {
        return Ok(None);
    };
    let field = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key)?.strip_prefix(':'))
            .map(|value| value.trim().to_string())
    };
    let (Some(started), Some(seconds)) = (field("started"), field("seconds")) else {
        return Err(format!(
            "{} is not a clock this version wrote - `cairns start --cancel` clears it",
            path(root).display()
        )
        .into());
    };
    Ok(Some(Clock {
        started,
        seconds: seconds.parse()?,
        title: field("title").filter(|title| !title.is_empty()),
    }))
}

/// Start the clock, replacing one already running.
pub fn start(root: &Path, title: Option<&str>, quiet: bool) -> Result<()> {
    if let Some(old) = read(root).ok().flatten()
        && !quiet
    {
        println!(
            "replaced a clock started {} ago{}",
            cairns_core::entry::format_duration(old.minutes()),
            old.title.map(|t| format!(" for {t:?}")).unwrap_or_default()
        );
    }
    let now = jiff::Zoned::now();
    let dir = root.join(".cairns");
    std::fs::create_dir_all(&dir)?;
    // Ignores itself: nothing in here is ever meant to be committed.
    std::fs::write(dir.join(".gitignore"), "*\n")?;
    let title = title.map(str::trim).filter(|t| !t.is_empty());
    std::fs::write(
        path(root),
        format!(
            "started: {}\nseconds: {}\ntitle: {}\n",
            now.strftime("%Y-%m-%dT%H:%M:%S%:z"),
            now.timestamp().as_second(),
            title.unwrap_or("")
        ),
    )?;
    if quiet {
        return Ok(());
    }
    println!(
        "clock started at {}{} - `cairns new` records how long it ran",
        now.strftime("%H:%M"),
        title.map(|t| format!(" for {t:?}")).unwrap_or_default()
    );
    Ok(())
}

/// Stop the clock without recording anything.
pub fn cancel(root: &Path) -> Result<()> {
    match std::fs::remove_file(path(root)) {
        Ok(()) => println!("clock cleared"),
        Err(problem) if problem.kind() == std::io::ErrorKind::NotFound => {
            println!("no clock running")
        }
        Err(problem) => return Err(problem.into()),
    }
    Ok(())
}

/// What is running, for `cairns clock`.
pub fn show(root: &Path) -> Result<()> {
    match read(root)? {
        Some(clock) => println!(
            "running {}, since {}{}",
            cairns_core::entry::format_duration(clock.minutes()),
            clock.started,
            clock.title.map(|t| format!(" - {t}")).unwrap_or_default()
        ),
        None => println!("no clock running - `cairns start` starts one"),
    }
    Ok(())
}

/// The clock is done with once an entry has recorded it.
pub fn stop(root: &Path) -> Result<()> {
    match std::fs::remove_file(path(root)) {
        Err(problem) if problem.kind() != std::io::ErrorKind::NotFound => Err(problem.into()),
        _ => Ok(()),
    }
}
