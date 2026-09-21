use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// `cairns.toml`. See `docs/spec/config.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "spec_version")]
    pub spec_version: u32,
    pub project: Project,
    #[serde(default)]
    pub paths: Paths,
    #[serde(default)]
    pub site: Site,
    #[serde(default)]
    pub index: Index,
    /// Ordered, because the order is how areas are presented everywhere.
    #[serde(default, rename = "area")]
    pub areas: Vec<Area>,
    #[serde(default, rename = "publish")]
    pub targets: Vec<Target>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    /// Permanent. Half of an entry's canonical id, `{slug}/{number}`.
    pub slug: String,
    #[serde(default)]
    pub description: String,
    /// The repository, if there is one. A README's relative links point at
    /// files in it, and on a site they have to point somewhere real.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paths {
    #[serde(default = "entries_dir")]
    pub entries: String,
    #[serde(default = "index_file")]
    pub index: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Site {
    /// Every internal link renders against this, so one build serves from a
    /// Pages sub-path and from a domain root alike.
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub theme: Option<String>,
    /// A markdown file shown on the site, so a reader arriving at a worklog
    /// can find out what the project is. Usually `README.md`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readme: Option<String>,
}

/// The generated index. Only its opening prose is a project's to write; the
/// counts and the table are derived, which is why it is never hand-edited.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Index {
    /// Replaces the default opening paragraph. A project explaining what its
    /// log is for says it better than a generated sentence can.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    pub name: String,
    /// One phrase, copied verbatim into the generated skill.
    #[serde(default)]
    pub about: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: TargetKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// An environment variable reference, always - see [`Config::validate`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TargetKind {
    Dir,
    GitBranch,
    /// Specified, reserved, and not implemented. `--dry-run` against it prints
    /// the request body a server would receive, which is the point of it
    /// existing before the server does.
    Http,
}

impl Config {
    pub fn parse(text: &str) -> Result<Self> {
        let config: Config = toml::from_str(text).map_err(|e| Error::Config(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    /// The checks that are cheaper to fail at parse time than to discover at
    /// publish time.
    pub fn validate(&self) -> Result<()> {
        if self.spec_version > crate::SPEC_VERSION {
            return Err(Error::Config(format!(
                "spec_version {} is newer than this build reads ({})",
                self.spec_version,
                crate::SPEC_VERSION
            )));
        }
        if self.project.slug.is_empty() {
            return Err(Error::Config(
                "project.slug must be set and never change".into(),
            ));
        }
        if self.areas.is_empty() {
            return Err(Error::Config(
                "no [[area]] declared - an entry must be filable".into(),
            ));
        }
        for target in &self.targets {
            // The file is committed, so a literal here is a leaked credential
            // in the next push. Catching it now costs nothing.
            if let Some(token) = &target.token
                && !token.starts_with('$')
            {
                return Err(Error::Config(format!(
                    "publish target {:?} has a literal token; use an environment \
                     variable reference such as \"$CAIRNS_TOKEN\"",
                    target.name
                )));
            }
        }
        Ok(())
    }

    pub fn knows_area(&self, name: &str) -> bool {
        self.areas.iter().any(|area| area.name == name)
    }
}

impl Default for Paths {
    fn default() -> Self {
        Paths {
            entries: entries_dir(),
            index: index_file(),
        }
    }
}

fn spec_version() -> u32 {
    crate::SPEC_VERSION
}

/// The layout the format grew up in, so an existing project adopts cairns
/// without moving a file.
fn entries_dir() -> String {
    "worklog".into()
}

fn index_file() -> String {
    "WORKLOG.md".into()
}
