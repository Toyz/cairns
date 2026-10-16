//! The project's README, as the About page.

use super::*;

/// The project's README, so a reader who arrives at a worklog can find out
/// what the project is - and how to install it.
pub fn about(log: &Log) -> String {
    let base = log.project.base_url.trim_end_matches('/');
    let readme = log.readme.as_deref().unwrap_or_default();

    // Its own `# Title` is the project name, which the masthead already says.
    let body_md = cairns_core::entry::without_title(readme);

    let body = format!(
        "<article>\n<h1>About</h1>\n{}</article>\n",
        markdown_linked(
            body_md,
            &Links {
                log,
                from_dir: "",
                rel: "../",
                base: None
            }
        )
    );
    shell(
        log,
        Page {
            title: &format!("About - {}", log.project.name),
            description: &if log.project.description.is_empty() {
                format!("What {} is.", log.project.name)
            } else {
                plain_md(&log.project.description)
            },
            url: &format!("{base}/about/"),
            rel: "../",
            nav: Nav::About,
            aside: "",
            contents: "",
            body: &body,
        },
    )
}
