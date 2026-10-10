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

/// The day each of `paths` - relative to `root` - last changed: the newest
/// commit whose version of the file differs from its first parent's, or today
/// for a file changed and not yet committed. A path git has never seen is
/// left out.
///
/// One walk for all of them, newest first, stopping when every path is
/// placed. A commit that left `within` alone - the reference's directory - is
/// passed over without looking at the files in it, which is most commits.
pub fn last_changed(
    root: &Path,
    within: &str,
    paths: &[String],
) -> std::collections::BTreeMap<String, String> {
    let mut found = std::collections::BTreeMap::new();
    let Some((repo, inside)) = open(root) else {
        return found;
    };
    let prefix = slashed(&inside);
    let today = jiff::Zoned::now().strftime("%Y-%m-%d").to_string();
    let Ok(head) = repo.head_commit() else {
        return found;
    };
    let Ok(head_tree) = head.tree() else {
        return found;
    };

    // Changed and not committed: the working copy is not what HEAD has.
    let mut waiting: Vec<&String> = Vec::new();
    for path in paths {
        let full = format!("{prefix}{path}");
        let committed = head_tree
            .lookup_entry_by_path(&full)
            .ok()
            .flatten()
            .and_then(|entry| entry.object().ok())
            .map(|object| object.data.clone());
        let on_disk = std::fs::read(root.join(path)).ok();
        match committed {
            Some(committed) if Some(&committed) == on_disk.as_ref() => waiting.push(path),
            _ => {
                found.insert(path.clone(), today.clone());
            }
        }
    }
    if waiting.is_empty() {
        return found;
    }

    let id_at = |tree: &gix::Tree<'_>, path: &str| {
        tree.lookup_entry_by_path(path)
            .ok()
            .flatten()
            .map(|entry| entry.object_id())
    };
    let within = format!("{prefix}{}", within.trim_matches('/'));
    let Ok(walk) = repo
        .rev_walk([head.id])
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            Default::default(),
        ))
        .all()
    else {
        return found;
    };
    for info in walk {
        if waiting.is_empty() {
            break;
        }
        let Ok(info) = info else { break };
        let Ok(commit) = info.object() else { continue };
        let Ok(tree) = commit.tree() else { continue };
        let parent = info
            .parent_ids()
            .next()
            .and_then(|id| id.object().ok())
            .and_then(|object| object.try_into_commit().ok())
            .and_then(|parent| parent.tree().ok());
        if let Some(parent) = &parent
            && id_at(&tree, &within) == id_at(parent, &within)
        {
            continue;
        }
        let Ok(time) = commit.time() else { continue };
        let day = jiff::Timestamp::from_second(time.seconds)
            .ok()
            .zip(jiff::tz::Offset::from_seconds(time.offset).ok())
            .map(|(at, offset)| {
                at.to_zoned(jiff::tz::TimeZone::fixed(offset))
                    .strftime("%Y-%m-%d")
                    .to_string()
            });
        let Some(day) = day else { continue };
        waiting.retain(|path| {
            let full = format!("{prefix}{path}");
            let here = id_at(&tree, &full);
            let before = parent.as_ref().and_then(|parent| id_at(parent, &full));
            if here.is_some() && here != before {
                found.insert((*path).clone(), day.clone());
                false
            } else {
                true
            }
        });
    }
    found
}
