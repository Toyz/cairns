//! The cairns worklog format.
//!
//! The markdown entries are the source of truth; everything here is a reader of
//! them and a writer of things generated from them. See `docs/spec/` for the
//! format itself - this crate is one implementation of that spec, and the spec
//! is the part that has to outlive it.
//!
//! Nothing in the core API touches the filesystem. Entries arrive through
//! [`Source`], so the same code serves a working tree today and a git tree or an
//! object store later, and the crate builds for `wasm32` with `--no-default-features`.

pub mod config;
pub mod date;
pub mod doc;
pub mod entry;
pub mod error;
pub mod log;
pub mod source;

/// The spec version this crate reads and writes. See `docs/spec/README.md`.
pub const SPEC_VERSION: u32 = 1;

pub use config::Config;
pub use date::{Date, rfc3339};
pub use doc::{Doc, Status};
pub use entry::{Entry, FrontMatter};
pub use error::{Error, Result};
pub use log::Log;
#[cfg(feature = "fs")]
pub use source::FsSource;
pub use source::{RawEntry, Source};
