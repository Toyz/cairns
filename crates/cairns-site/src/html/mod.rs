//! The pages.
//!
//! Hand-built strings rather than a template engine: the whole site is five
//! page shapes, and a dependency that renders them would be larger than they
//! are. Every page is complete HTML with its own metadata, because the unit
//! that gets shared is one entry, not the site.
//!
//! One file per page, and three for what they share: [`markdown`] - the text
//! and where its links go - [`frame`], the head, rail and footer every page has,
//! and [`parts`], the pieces several pages show.

mod about;
mod docs;
mod entry;
mod frame;
mod home;
mod markdown;
mod open;
mod parts;

pub use about::*;
pub use docs::*;
pub use entry::*;
pub(crate) use frame::*;
pub use home::*;
pub use markdown::*;
pub use open::*;
pub(crate) use parts::*;

use cairns_core::log::{DocPage, Log, LogEntry};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd, html};
use std::collections::BTreeMap;
use std::fmt::Write as _;
