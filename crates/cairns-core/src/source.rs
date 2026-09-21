use crate::error::Result;

/// One entry file, unparsed.
#[derive(Debug, Clone)]
pub struct RawEntry {
    /// Repo-relative, with forward slashes, whatever the host.
    pub path: String,
    pub bytes: Vec<u8>,
}

/// Where entries come from.
///
/// The core never opens a file itself. A working tree is one source; a git tree,
/// an archive or an object store are others, and a hosted ingest would be
/// another still - none of which the parsing, validating or rendering code has
/// to know about.
pub trait Source {
    fn entries(&self) -> Result<Vec<RawEntry>>;
}

#[cfg(feature = "fs")]
mod fs {
    use super::{RawEntry, Source};
    use crate::error::Result;
    use std::path::{Path, PathBuf};

    /// Entries read from a directory in a working tree.
    pub struct FsSource {
        root: PathBuf,
        dir: String,
        /// Entries are flat; a docs tree is not.
        deep: bool,
    }

    impl FsSource {
        pub fn new(root: impl AsRef<Path>, dir: impl Into<String>) -> Self {
            FsSource {
                root: root.as_ref().to_path_buf(),
                dir: dir.into(),
                deep: false,
            }
        }

        /// Walk sub-directories too, for a tree of reference pages.
        pub fn recursive(root: impl AsRef<Path>, dir: impl Into<String>) -> Self {
            FsSource {
                deep: true,
                ..FsSource::new(root, dir)
            }
        }
    }

    impl FsSource {
        fn walk(&self, dir: &Path, prefix: &str, found: &mut Vec<RawEntry>) -> Result<()> {
            for item in std::fs::read_dir(dir)? {
                let path = item?.path();
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if path.is_dir() {
                    if self.deep && !name.starts_with('.') {
                        self.walk(&path, &format!("{prefix}/{name}"), found)?;
                    }
                    continue;
                }
                if path.extension().is_none_or(|ext| ext != "md") {
                    continue;
                }
                found.push(RawEntry {
                    path: format!("{prefix}/{name}"),
                    bytes: std::fs::read(&path)?,
                });
            }
            Ok(())
        }
    }

    impl Source for FsSource {
        fn entries(&self) -> Result<Vec<RawEntry>> {
            let mut found = Vec::new();
            self.walk(&self.root.join(&self.dir), &self.dir, &mut found)?;
            found.sort_by(|a, b| a.path.cmp(&b.path));
            Ok(found)
        }
    }
}

#[cfg(feature = "fs")]
pub use fs::FsSource;
