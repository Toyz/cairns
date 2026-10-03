//! A small set of inline icons, by name.
//!
//! Inline rather than a font or a sprite sheet, for the same reason the page
//! uses no webfonts: an icon that needs a request is an icon that is missing
//! for the first second, or forever behind a proxy. Ten of them cost less than
//! one request.
//!
//! An unknown name yields nothing at all. A link with no glyph still works,
//! and a typo in a config should not stop a site from building.

/// The inner markup of a 16x16 icon, or `None` if the name is not built in.
pub fn svg(name: &str) -> Option<&'static str> {
    Some(match name.trim().to_ascii_lowercase().as_str() {
        // Octicons `mark-github`, MIT licensed by GitHub.
        "github" => {
            r#"<path fill="currentColor" d="M8 0c4.42 0 8 3.58 8 8a8.013 8.013 0 0 1-5.45 7.59c-.4.08-.55-.17-.55-.38 0-.27.01-1.13.01-2.2 0-.75-.25-1.23-.54-1.48 1.78-.2 3.65-.88 3.65-3.95 0-.88-.31-1.59-.82-2.15.08-.2.36-1.02-.08-2.12 0 0-.66-.21-2.2.82-.6-.17-1.23-.25-1.87-.25-.64 0-1.27.08-1.87.25C4.7 3.9 4.04 4.11 4.04 4.11c-.44 1.1-.16 1.92-.08 2.12-.51.56-.82 1.27-.82 2.15 0 3.06 1.86 3.75 3.64 3.95-.23.2-.44.55-.51 1.07-.46.21-1.61.55-2.33-.66-.15-.24-.6-.83-1.23-.82-.67.01-.27.38.01.53.34.19.73.9.82 1.13.16.45.68 1.31 2.69.94 0 .67.01 1.3.01 1.49 0 .21-.15.45-.55.38A7.995 7.995 0 0 1 0 8c0-4.42 3.58-8 8-8Z"/>"#
        }
        "globe" | "site" | "web" => {
            r#"<circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" stroke-width="1.4"/><path d="M1.5 8h13M8 1.5c1.8 1.9 2.7 4 2.7 6.5S9.8 12.6 8 14.5C6.2 12.6 5.3 10.5 5.3 8S6.2 3.4 8 1.5Z" fill="none" stroke="currentColor" stroke-width="1.4"/>"#
        }
        "book" | "docs" => {
            r#"<path d="M2.5 2.5h4.2c.7 0 1.3.6 1.3 1.3v9.7c0-.7-.6-1.3-1.3-1.3H2.5Zm11 0H9.3c-.7 0-1.3.6-1.3 1.3v9.7c0-.7.6-1.3 1.3-1.3h4.2Z" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/>"#
        }
        "code" => {
            r#"<path d="M5.5 4 1.8 8l3.7 4m5-8L14.2 8l-3.7 4" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#
        }
        "download" => {
            r#"<path d="M8 1.8v8.4M4.6 7l3.4 3.4L11.4 7M2 13.4h12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#
        }
        "rss" | "feed" => {
            r#"<circle cx="3.4" cy="12.6" r="1.6" fill="currentColor"/><path d="M2.4 7.2A6.4 6.4 0 0 1 8.8 13.6M2.4 2.6A11 11 0 0 1 13.4 13.6" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>"#
        }
        "chat" | "discord" | "forum" => {
            r#"<path d="M14 9.2c0 1.2-1 2.2-2.2 2.2H6.6L3 14V4.2C3 3 4 2 5.2 2h6.6C13 2 14 3 14 4.2Z" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/>"#
        }
        "mail" => {
            r#"<rect x="1.6" y="3.4" width="12.8" height="9.2" rx="1.2" fill="none" stroke="currentColor" stroke-width="1.4"/><path d="m2.4 4.6 5.6 4 5.6-4" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/>"#
        }
        "star" => {
            r#"<path d="m8 1.7 2 4.1 4.5.6-3.3 3.2.8 4.5L8 11.9l-4 2.2.8-4.5L1.5 6.4 6 5.8Z" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/>"#
        }
        "link" => {
            r#"<path d="M6.5 9.5a2.6 2.6 0 0 0 3.7 0l2.4-2.4a2.6 2.6 0 0 0-3.7-3.7l-.9.9m-1.5 5.2a2.6 2.6 0 0 0-3.7 0L.4 11.9a2.6 2.6 0 0 0 3.7 3.7l.9-.9" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" transform="translate(0.8 -0.8)"/>"#
        }
        _ => return None,
    })
}

/// Every name that resolves to an icon, for documentation and for `check`.
pub const NAMES: &[&str] = &[
    "github", "globe", "book", "code", "download", "rss", "chat", "mail", "star", "link",
];

#[cfg(test)]
mod tests {
    use super::{NAMES, svg};

    #[test]
    fn every_documented_name_has_an_icon() {
        for name in NAMES {
            assert!(svg(name).is_some(), "{name} is documented but not built in");
        }
    }

    #[test]
    fn an_unknown_name_is_not_an_error() {
        assert!(svg("octopus").is_none());
        assert!(svg("").is_none());
    }

    #[test]
    fn names_are_matched_loosely() {
        assert_eq!(svg("GitHub"), svg("github"));
        assert_eq!(svg("  rss "), svg("rss"));
        // Aliases, so a project writing the obvious word gets something.
        assert!(svg("docs").is_some());
        assert!(svg("discord").is_some());
    }
}

/// Whether an `icon` value is a URL rather than a name or a path.
pub fn is_url(icon: &str) -> bool {
    let icon = icon.trim();
    icon.starts_with("https://")
        || icon.starts_with("http://")
        || icon.starts_with("//")
        || icon.starts_with("data:image/")
}

/// Whether an `icon` value names a file in the repository: anything with a
/// slash or a dot in it that is not a URL. A built-in name has neither.
pub fn is_path(icon: &str) -> bool {
    let icon = icon.trim();
    !is_url(icon) && !icon.starts_with('<') && (icon.contains('/') || icon.contains('.'))
}

/// An SVG file's markup, made safe to put in a page as it is.
///
/// Inline rather than an `<img>` so that an icon drawn in `currentColor`
/// follows the page's colours, light and dark, like the built-in ones. That
/// puts the file's markup in every page, so anything that could run is
/// refused rather than cleaned: an icon has no business carrying a script.
pub fn clean_svg(text: &str) -> Result<String, String> {
    let mut rest = text.trim_start_matches('\u{feff}').trim();
    // The prolog, a doctype and leading comments are not part of the drawing.
    loop {
        if let Some(after) = rest.strip_prefix("<?") {
            rest = after
                .split_once("?>")
                .map(|(_, r)| r)
                .unwrap_or("")
                .trim_start();
        } else if let Some(after) = rest.strip_prefix("<!--") {
            rest = after
                .split_once("-->")
                .map(|(_, r)| r)
                .unwrap_or("")
                .trim_start();
        } else if rest.starts_with("<!") {
            rest = rest
                .split_once('>')
                .map(|(_, r)| r)
                .unwrap_or("")
                .trim_start();
        } else {
            break;
        }
    }
    if !rest.starts_with("<svg") {
        return Err("is not an SVG - it does not start with <svg".into());
    }
    let lower = rest.to_ascii_lowercase();
    for refused in [
        "<script",
        "javascript:",
        "<foreignobject",
        "<iframe",
        "<object",
        "<embed",
    ] {
        if lower.contains(refused) {
            return Err(format!("contains {refused}, which an icon cannot carry"));
        }
    }
    // Event handler attributes: ` onload=`, ` onclick=` and the rest.
    let bytes = lower.as_bytes();
    for (at, _) in lower.match_indices("on") {
        let before = at.checked_sub(1).map(|i| bytes[i]);
        if !matches!(before, Some(b' ' | b'\t' | b'\n' | b'\r')) {
            continue;
        }
        let name: String = lower[at..]
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        let after = lower[at + name.len()..].trim_start();
        if name.len() > 2 && after.starts_with('=') {
            return Err(format!(
                "has an {name} attribute, which an icon cannot carry"
            ));
        }
    }
    Ok(rest.trim_end().to_string())
}

/// The markup for an icon in the rail, or `None` for no icon: a built-in name,
/// a URL, or markup a repository path was already resolved to.
pub fn html(icon: &str) -> Option<String> {
    let icon = icon.trim();
    if let Some(inner) = svg(icon) {
        return Some(format!(
            "<svg viewBox=\"0 0 16 16\" aria-hidden=\"true\" focusable=\"false\">{inner}</svg>"
        ));
    }
    if let Some(rest) = icon.strip_prefix("<svg") {
        return Some(format!(
            "<svg aria-hidden=\"true\" focusable=\"false\"{rest}"
        ));
    }
    if is_url(icon) {
        return Some(format!(
            "<img src=\"{}\" alt=\"\" aria-hidden=\"true\" loading=\"lazy\">",
            crate::html::escape(icon)
        ));
    }
    None
}

#[cfg(test)]
mod source_tests {
    use super::*;

    #[test]
    fn an_icon_is_a_name_a_url_or_a_path() {
        assert!(html("github").is_some_and(|h| h.contains("viewBox=\"0 0 16 16\"")));
        assert!(html("https://example.com/i.svg").is_some_and(|h| h.starts_with("<img")));
        assert!(is_path("assets/logo.svg") && is_path("logo.png"));
        assert!(!is_path("github") && !is_path("https://example.com/i.svg"));
        // An unresolved path, or a misspelt name, draws nothing.
        assert!(html("assets/logo.svg").is_none() && html("githb").is_none());
    }

    #[test]
    fn an_svg_file_is_inlined_without_its_prolog() {
        let clean = clean_svg(
            "<?xml version=\"1.0\"?>\n<!-- drawn by hand -->\n<svg viewBox=\"0 0 24 24\"><path d=\"M0 0h24\"/></svg>\n",
        )
        .unwrap();
        assert!(clean.starts_with("<svg viewBox"));
        assert!(
            html(&clean)
                .unwrap()
                .starts_with("<svg aria-hidden=\"true\" focusable=\"false\" viewBox")
        );
    }

    #[test]
    fn an_svg_that_could_run_anything_is_refused() {
        assert!(clean_svg("<svg><script>alert(1)</script></svg>").is_err());
        assert!(clean_svg("<svg onload=\"alert(1)\"></svg>").is_err());
        assert!(clean_svg("<svg><a href=\"javascript:x\"/></svg>").is_err());
        assert!(clean_svg("<html></html>").is_err());
        // `stroke-linejoin` and `fill-opacity` are not handlers.
        assert!(clean_svg("<svg><path stroke-linejoin=\"round\" font=\"x\"/></svg>").is_ok());
    }
}
