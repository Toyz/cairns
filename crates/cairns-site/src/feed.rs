//! The Atom feed.
//!
//! It carries each entry's full rendered content rather than an excerpt, so it
//! is a usable way to *read* the log. It is not the complete log: it is capped
//! at the newest [`FEED_ENTRIES`], the way feeds are, and points at `log.json`
//! for anything that wants all of it.

use crate::html::escape;
use cairns_core::log::Log;
use std::fmt::Write as _;

/// How many entries the feed carries. A reader wants the recent ones; an
/// ingester wants `log.json`, which the feed links to.
pub const FEED_ENTRIES: usize = 25;

pub fn atom(log: &Log) -> String {
    let base = log.project.base_url.trim_end_matches('/');
    let updated = log
        .entries
        .last()
        .map(|entry| format!("{}T00:00:00Z", entry.date))
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".into());

    let mut out = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    let _ = write!(
        out,
        r#"<feed xmlns="http://www.w3.org/2005/Atom">
<title>{title}</title>
<subtitle>{subtitle}</subtitle>
<id>{base}/</id>
<link rel="self" href="{base}/feed.xml"/>
<link rel="alternate" href="{base}/"/>
<link rel="alternate" type="application/json" href="{base}/log.json"/>
<updated>{updated}</updated>
<generator>{generator}</generator>
"#,
        title = escape(&log.project.name),
        subtitle = escape(&log.project.description),
        base = escape(base),
        generator = escape(&log.generator),
    );

    // Newest first: a feed reader takes the order it is given.
    for entry in log.entries.iter().rev().take(FEED_ENTRIES) {
        let _ = write!(
            out,
            r#"<entry>
<title>{title}</title>
<id>{url}</id>
<link rel="alternate" href="{url}"/>
<updated>{date}T00:00:00Z</updated>
{categories}<summary>{summary}</summary>
<content type="html">{content}</content>
</entry>
"#,
            title = escape(&entry.title),
            url = escape(&entry.url),
            date = entry.date,
            categories = entry
                .areas
                .iter()
                .map(|area| format!("<category term=\"{}\"/>\n", escape(area)))
                .collect::<String>(),
            summary = escape(&crate::html::plain_md(
                entry.summary.as_deref().unwrap_or(&entry.title)
            )),
            content = escape(&crate::html::body_html(log, entry)),
        );
    }

    out.push_str("</feed>\n");
    out
}
