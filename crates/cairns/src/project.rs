//! The project on disk: finding it, reading its entries and pages, and
//! building the log from them - everything a command needs before it does its
//! own part.

use crate::{Result, code, doc, git, license, pages, refs, workspace};
use cairns_core::{Config, Entry, FsSource, Log, Source};
use std::path::{Path, PathBuf};

/// Find `cairns.toml` by walking up from the working directory, so the command
/// works from anywhere in the repo.
pub fn load() -> Result<(PathBuf, Config)> {
    let mut dir = std::env::current_dir()?;
    loop {
        let candidate = dir.join("cairns.toml");
        if candidate.is_file() {
            let mut config = Config::parse(&std::fs::read_to_string(&candidate)?)?;
            infer_base_url(&dir, &mut config);
            return Ok((dir, config));
        }
        if !dir.pop() {
            return Err("no cairns.toml here or above - run `cairns init`".into());
        }
    }
}

/// `cairns.toml` at a root already found, read again - for `serve`, which
/// watches it.
pub fn load_config(root: &Path) -> Result<Config> {
    let mut config = Config::parse(&std::fs::read_to_string(root.join("cairns.toml"))?)?;
    infer_base_url(root, &mut config);
    Ok(config)
}

/// Fill an unset `site.base_url` with the Pages address the repository
/// implies, noting where it came from.
pub fn infer_base_url(root: &Path, config: &mut Config) {
    // The repository too, when not given: a README's relative links and the
    // license files are linked into it.
    if config.project.repository.is_none() {
        config.project.repository = pages::infer_repository(root);
    }
    if !config.site.base_url.trim().is_empty() {
        return;
    }
    if let Some((url, from)) = pages::infer(root, config.project.repository.as_deref()) {
        config.site.base_url = url;
        config.site.base_url_from = Some(from);
    }
}

pub fn read_entries(root: &Path, config: &Config) -> Result<Vec<Entry>> {
    let source = FsSource::new(root, &config.paths.entries);
    let mut entries = Vec::new();
    for raw in source.entries()? {
        entries.push(Entry::parse(&raw)?);
    }
    Ok(entries)
}

/// The reference pages, if the project keeps any.
pub fn read_docs(root: &Path, config: &Config) -> Result<Vec<cairns_core::Doc>> {
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

/// The next number, from the filenames alone - never a read of the entries, so
/// it costs the same on entry 5 and entry 5,000.
pub fn next_number(dir: &Path) -> Result<u32> {
    let mut highest = 0;
    let items = match std::fs::read_dir(dir) {
        Ok(items) => items,
        // No directory yet: no entries yet.
        Err(problem) if problem.kind() == std::io::ErrorKind::NotFound => return Ok(1),
        Err(problem) => return Err(problem.into()),
    };
    for item in items {
        let name = item?.file_name().to_string_lossy().to_string();
        let digits: String = name.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(number) = digits.parse::<u32>() {
            highest = highest.max(number);
        }
    }
    Ok(highest + 1)
}

/// Regenerate the index, and the reference's index pages that carry the
/// marker, returning how many entries went into it.
pub fn write_index(root: &Path, config: &Config) -> Result<usize> {
    let built = index_log(root, config)?;
    std::fs::write(
        root.join(&config.paths.index),
        cairns_site::render_index(&built, config.index.header.as_deref(), published(config)),
    )?;
    doc::write_indexes(root, config, &built, false)?;
    Ok(built.entries.len())
}

/// The log as the generated indexes need it: entries and pages, and nothing
/// that needs the network or the history.
pub fn index_log(root: &Path, config: &Config) -> Result<Log> {
    Ok(Log::build_with(
        config,
        read_entries(root, config)?,
        &read_docs(root, config)?,
        None,
    ))
}

/// Where the log is published, for the index to say - only when `cairns.toml`
/// says so: an address worked out from CI or a remote can differ between a
/// fork's CI and a checkout, and the index would be stale in one of them.
pub fn published(config: &Config) -> Option<&str> {
    let url = config.site.base_url.trim();
    (config.site.base_url_from.is_none() && !url.is_empty()).then_some(url)
}

/// Where the text lives, for a license of its own: the worklog's directory and
/// the reference's.
pub fn text_dirs(config: &Config) -> Vec<&str> {
    let mut dirs = vec![config.paths.entries.as_str()];
    if let Some(docs) = &config.docs {
        dirs.push(docs.dir.as_str());
    }
    dirs
}

/// Build the canonical document, README and all.
///
/// The readme is read here rather than in core, which owns no filesystem, and
/// lands in `log.json` so the renderer still consumes one thing.
pub fn build_log(root: &Path, config: &Config, generated: Option<String>) -> Result<Log> {
    let mut built = Log::build_with(
        config,
        read_entries(root, config)?,
        &read_docs(root, config)?,
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
    built.workspaces = workspace::load(root, config);
    // The licenses: as declared, else as found; the files they are in, either
    // way, for the footer to link to.
    let found = license::detect(root, &text_dirs(config));
    if built.project.license.is_none() {
        built.project.license = found.code.map(|code| code.expression);
    }
    if built.project.text_license.is_none() {
        built.project.text_license = found.text.map(|text| text.expression);
    }
    built.project.license_files = found.files;
    if built.project.text_license.as_deref() == built.project.license.as_deref() {
        built.project.text_license = None;
    }
    for entry in &mut built.entries {
        entry.attachments = attachments(root, &entry.path);
    }
    // When each reference page last changed, from the history.
    if let Some(docs) = &config.docs
        && !built.docs.is_empty()
    {
        let paths: Vec<String> = built.docs.iter().map(|doc| doc.path.clone()).collect();
        let changed = git::last_changed(root, &docs.dir, &paths);
        for doc in &mut built.docs {
            doc.changed = changed.get(&doc.path).cloned();
        }
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

pub fn stamp(reproducible: bool) -> Option<String> {
    if reproducible {
        return None;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default();
    Some(cairns_core::rfc3339(now))
}

/// The files in an entry's folder - `worklog/0050-the-table/` beside
/// `0050-the-table.md` - by name within it, sub-folders included.
pub fn attachments(root: &Path, entry_path: &str) -> Vec<String> {
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
pub fn render_site(root: &Path, built: &Log) -> Result<cairns_site::Rendered> {
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

/// A repository image as an icon: an SVG's own markup, so it takes the page's
/// colours, or anything else as a `data:` URL, so the page still makes no
/// request for it.
pub fn resolve_icon(root: &Path, path: &str) -> Result<String, String> {
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
        other => crate::serve::image_type(other).ok_or_else(|| {
            format!("is a .{other} file; an icon is .svg, .png, .jpg, .gif, .webp or .ico")
        })?,
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
pub fn base64(bytes: &[u8]) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
