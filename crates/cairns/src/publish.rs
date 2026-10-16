//! Getting the log out: `build` into a directory, `export` as `log.json`,
//! `publish` to a configured target.

use crate::project::{build_log, render_site, stamp, text_dirs};
use crate::{Result, license};
use cairns_core::config::TargetKind;
use cairns_core::{Config, Log};
use std::path::Path;

/// `cairns build`: the site, into `out`, saying where anything it had to work
/// out came from.
pub fn build(root: &Path, config: &Config, out: &Path) -> Result<()> {
    // Said, so an address that was guessed is never a surprise.
    match &config.site.base_url_from {
        Some(from) => eprintln!(
            "base_url: {} - from {from}; set site.base_url in cairns.toml to change it",
            config.site.base_url
        ),
        None if config.site.base_url.is_empty() => eprintln!(
            "base_url is not set and could not be worked out - the feed, ids.json and \
             every page's canonical link will have no address. Set site.base_url in \
             cairns.toml"
        ),
        None => {}
    }
    let found = license::detect(root, &text_dirs(config));
    if config.project.license.is_none() {
        match &found.code {
            Some(code) => eprintln!(
                "license: {} - from {}; set project.license in cairns.toml to change it",
                code.expression, code.from
            ),
            None => eprintln!(
                "no license declared or found - set project.license in cairns.toml to show \
                 one, and project.text_license if the log's text is licensed apart from the code"
            ),
        }
    }
    if config.project.text_license.is_none()
        && let Some(text) = &found.text
    {
        eprintln!(
            "text license: {} - from {}; set project.text_license in cairns.toml to change it",
            text.expression, text.from
        );
    }
    let built = build_log(root, config, stamp(false))?;
    let rendered = render_site(root, &built)?;
    for file in &rendered.files {
        let path = out.join(&file.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &file.bytes)?;
    }
    println!("{} entries -> {}", built.entries.len(), out.display());
    Ok(())
}

/// `cairns export`: `log.json`, to a file or stdout.
pub fn export(root: &Path, config: &Config, out: Option<&Path>, reproducible: bool) -> Result<()> {
    let built = build_log(root, config, stamp(reproducible))?;
    let json = serde_json::to_string_pretty(&built)?;
    match out {
        Some(path) => std::fs::write(path, json)?,
        None => println!("{json}"),
    }
    Ok(())
}

/// `cairns publish`: the payload, to the target named or the only one there is.
pub fn publish(root: &Path, config: &Config, target: Option<&str>, dry_run: bool) -> Result<()> {
    let target = pick_target(config, target)?;
    let built = build_log(root, config, stamp(false))?;
    let rendered = render_site(root, &built)?;

    match target.kind {
        TargetKind::Dir => {
            let out = root.join(target.path.as_deref().unwrap_or("site"));
            let changed = changes(&out, &built);
            report(&changed, &built);
            if dry_run {
                println!("dry run - nothing written to {}", out.display());
                return Ok(());
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
    Ok(())
}

/// Which target to publish to: the one named, or the only one there is.
fn pick_target<'a>(
    config: &'a Config,
    named: Option<&str>,
) -> Result<&'a cairns_core::config::Target> {
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
