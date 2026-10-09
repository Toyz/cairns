//! The little of git that cairns reads, in-process through `gix`.
//!
//! It used to run the `git` binary - `git ls-files`, `git show rev:path` -
//! which needed one on the path, cost a process each time, and meant reading
//! errors out of its output. Both are a few lines against the repository here.
//!
//! The project's root (where `cairns.toml` is) may sit anywhere inside the
//! repository, so paths are translated between the two.

use std::path::{Path, PathBuf};

/// The repository containing `root`, and where `root` is inside its work tree.
fn open(root: &Path) -> Option<(gix::Repository, PathBuf)> {
    let repo = gix::discover(root).ok()?;
    let workdir = repo.workdir()?.canonicalize().ok()?;
    let inside = root
        .canonicalize()
        .ok()?
        .strip_prefix(&workdir)
        .ok()?
        .to_path_buf();
    Some((repo, inside))
}

/// The files git tracks under `root`, relative to it, or `None` when `root`
/// is not in a git repository.
pub fn tracked(root: &Path) -> Option<Vec<String>> {
    let (repo, inside) = open(root)?;
    let index = repo.index_or_empty().ok()?;
    let prefix = slashed(&inside);
    let mut found = Vec::new();
    for entry in index.entries() {
        let path = entry.path(&index).to_string();
        match path.strip_prefix(&prefix) {
            Some(rest) => found.push(rest.to_string()),
            None if prefix.is_empty() => found.push(path),
            None => {}
        }
    }
    Some(found)
}

/// A file as it was at `rev`, by its path relative to `root`.
pub fn show(root: &Path, rev: &str, path: &str) -> Result<String, String> {
    let (repo, inside) = open(root).ok_or("not in a git repository, so no revision can be read")?;
    let full = format!("{}{path}", slashed(&inside));
    let id = repo
        .rev_parse_single(format!("{rev}:{full}").as_str())
        .map_err(|_| format!("not in the repository at {rev}"))?;
    let object = id
        .object()
        .map_err(|_| format!("not in the repository at {rev}"))?;
    String::from_utf8(object.data.clone()).map_err(|_| "not text".to_string())
}

/// A path inside the work tree as a prefix: `""`, or `sub/dir/`.
fn slashed(inside: &Path) -> String {
    let text = inside.to_string_lossy().replace('\\', "/");
    if text.is_empty() {
        text
    } else {
        format!("{text}/")
    }
}
