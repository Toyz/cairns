use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// A problem in one entry file, reported against its path so the message
    /// names something the reader can open.
    Entry {
        path: String,
        problem: String,
    },
    Config(String),
    Io(std::io::Error),
}

impl Error {
    pub fn entry(path: impl Into<String>, problem: impl Into<String>) -> Self {
        Error::Entry {
            path: path.into(),
            problem: problem.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Entry { path, problem } => write!(f, "{path}: {problem}"),
            Error::Config(m) => write!(f, "cairns.toml: {m}"),
            Error::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
