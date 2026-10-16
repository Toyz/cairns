//! What cairns writes into a repository besides the log: `cairns.toml`, the
//! skills under `.claude/skills/`, the clock hook - and `init` and `update`,
//! which write them.

use crate::Result;
use crate::project::{read_entries, write_index};
use cairns_core::Config;
use std::fmt::Write as _;
use std::path::Path;

/// `cairns init`: a config if there is none, the skills, and an index - and,
/// for a log that predates cairns, its history left passing `check`.
pub fn init(hooks: bool) -> Result<()> {
    let root = &std::env::current_dir()?;
    let config_path = root.join("cairns.toml");
    if !config_path.exists() {
        let name = root
            .file_name()
            .map_or_else(|| "project".into(), |n| n.to_string_lossy().to_string());
        std::fs::write(&config_path, starter_config(&name))?;
        println!("wrote cairns.toml - edit the areas before writing an entry");
    } else {
        println!("cairns.toml is already here, keeping it");
    }

    let config = &Config::parse(&std::fs::read_to_string(&config_path)?)?;
    std::fs::create_dir_all(root.join(&config.paths.entries))?;

    write_skills(root, config)?;
    if hooks {
        add_clock_hook(root)?;
    }

    // Adoption preserves what is there, the same way it freezes slugs.
    // A log written before this convention existed should not fail
    // `check` on its history; entries written from here on will.
    let entries = read_entries(root, config)?;
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
                "{}\n\n# {silent} entries predate the open-question line. Set this back to\n\
                 # \"required\" once they carry one - `cairns open` is only as complete as\n\
                 # the entries that answer it.\n[check]\nopen_questions = \"optional\"\n",
                raw.trim_end()
            ),
        )?;
        println!(
            "{silent} entries have no `**Still unknown:**` line - \
             check relaxed to optional in cairns.toml"
        );
    }

    let frozen = freeze_slugs(root, config)?;
    if frozen > 0 {
        println!(
            "froze {frozen} slug{} - these entries keep the names they were \
             published under",
            if frozen == 1 { "" } else { "s" }
        );
    }

    let count = write_index(root, config)?;
    println!("{count} entries -> {}", config.paths.index);
    Ok(())
}

/// `cairns update`: everything generated, refreshed; nothing else touched.
pub fn update(root: &Path, config: &Config) -> Result<()> {
    write_skills(root, config)?;
    refresh_clock_hook(root)?;
    let count = write_index(root, config)?;
    println!("{count} entries -> {}", config.paths.index);
    Ok(())
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

/// Everything below this line in a generated skill is the project's own.
const SKILL_MARKER: &str = "<!-- cairns:project -->";

/// Write `.claude/skills/<name>/SKILL.md`, keeping its hand-written half.
///
/// A skill that predates cairns is entirely hand-written, and rewriting it
/// would throw away exactly the rules worth keeping, so one without the marker
/// is left alone. Adoption asks rather than assumes.
fn write_skill(
    root: &Path,
    name: &str,
    render: impl FnOnce(Option<&str>) -> (String, bool),
) -> Result<()> {
    let skill = root.join(format!(".claude/skills/{name}/SKILL.md"));
    std::fs::create_dir_all(skill.parent().expect("a skill has a directory"))?;
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
fn write_skills(root: &Path, config: &Config) -> Result<()> {
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
    let first = config.areas.first().map_or("design", |a| a.name.as_str());
    // Every example carries two areas, because the question people ask first is
    // whether an entry may sit in more than one, and prose saying so under a
    // single-area example does not answer it.
    let second = config.areas.get(1).map_or(first, |a| a.name.as_str());

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
         write or correct the page too, as this project's skill for its reference\n\
         pages says, and cite the entry on it:\n\n\
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
        .map_or("docs", |docs| docs.dir.as_str());
    let mut rendered = include_str!("../templates/REFERENCE.md")
        .replace("{{project}}", &config.project.name)
        .replace("{{docs}}", dir);
    // What `cairns.toml` declares - the sections a page may go in, the front
    // matter it must carry - said where the model writing a page will read
    // it, generated so it cannot fall behind the config.
    let declared = reference_conventions(config);
    if !declared.is_empty() {
        rendered = rendered.replacen(SKILL_MARKER, &format!("{declared}{SKILL_MARKER}"), 1);
    }
    keep_project_section(rendered, existing)
}

/// The reference skill's part generated from `[docs]`: its sections and its
/// fields, or nothing when it declares neither.
fn reference_conventions(config: &Config) -> String {
    let Some(docs) = &config.docs else {
        return String::new();
    };
    if docs.sections.is_empty() && docs.fields.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "## This project's conventions

From `cairns.toml`; `check` holds pages to them.

",
    );
    if !docs.sections.is_empty() {
        out.push_str(
            "A page goes in one of these sections - `--in` takes the folder:

",
        );
        for section in &docs.sections {
            let _ = write!(out, "- `{}` - {}", section.dir, section.title());
            if !section.about.trim().is_empty() {
                let _ = write!(out, ": {}", section.about.trim());
            }
            out.push('\n');
        }
        out.push('\n');
    }
    if !docs.fields.is_empty() {
        out.push_str(
            "Front matter:

",
        );
        for field in &docs.fields {
            let mut line = format!("- `{}`", field.name);
            line.push_str(if field.required {
                " - required"
            } else {
                " - optional"
            });
            if !field.values.is_empty() {
                let _ = write!(
                    line,
                    "; a comma-separated list of {}",
                    field
                        .values
                        .iter()
                        .map(|value| format!("`{value}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            out.push_str(&line);
            out.push('\n');
        }
        out.push('\n');
    }
    out
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
fn add_clock_hook(root: &Path) -> Result<()> {
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

/// Bring an existing clock hook up to the command this version writes. Adds
/// nothing: a project that never asked for the hook does not get one from an
/// update.
fn refresh_clock_hook(root: &Path) -> Result<()> {
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

/// Pin the slug of every entry whose filename no longer matches its title.
///
/// Adoption has to freeze what exists rather than improve it. The slug is the
/// URL, and a log that has been published already handed those out - so an
/// entry whose name disagrees with its title keeps the name, in writing, and
/// only entries written from here on get the better derivation.
fn freeze_slugs(root: &Path, config: &Config) -> Result<usize> {
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
        let _ = write!(updated, "\nslug: {published}");
        updated.push_str(&text[anchor..]);
        std::fs::write(&path, updated)?;
        frozen += 1;
    }
    Ok(frozen)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config::parse(&starter_config("A Project")).expect("the starter config is valid")
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
    fn the_areas_reach_the_skill() {
        let (rendered, _) = render_skill(&config(), None);
        assert!(
            rendered.contains("| `design` | a decision made, with the alternatives rejected |")
        );
    }
}
