//! Syntax highlighting for fenced code, at build time.
//!
//! Done here rather than by a script in the page, because a worklog is read
//! offline, behind proxies and in feed readers, and a page that has to fetch a
//! highlighter to colour its code does not colour it in any of those places.
//!
//! The output is classes, not colours. `syntect` names each span by its scope -
//! `hl-keyword`, `hl-string`, `hl-comment` - and the stylesheet maps a handful
//! of those onto the `--code-*` tokens, so code follows light and dark and a
//! project's `[colors]` like everything else on the page.

use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag, TagEnd};
use std::sync::OnceLock;
use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::{SyntaxDefinition, SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(|| {
        let mut builder = SyntaxSet::load_defaults_newlines().into_builder();
        // The bundled set has no TOML, and this tool's own docs are full of it.
        match SyntaxDefinition::load_from_str(
            include_str!("assets/toml.sublime-syntax"),
            true,
            None,
        ) {
            Ok(toml) => builder.add(toml),
            Err(problem) => debug_assert!(false, "the TOML grammar does not load: {problem}"),
        }
        builder.build()
    })
}

/// The grammar a fence names, if there is one. `rust`, `rs`, `py`, `sh` and
/// the like are matched by `syntect` itself; a fence with no language, or one
/// it has no grammar for, is left as plain code.
fn syntax_for(info: &str) -> Option<&'static SyntaxReference> {
    let token = info.split([' ', ',', '{']).next()?.trim();
    if token.is_empty() {
        return None;
    }
    let set = syntaxes();
    let token = match token {
        // Shell sessions are written as `console` or `shell` as often as `sh`.
        "console" | "shell" | "zsh" => "sh",
        other => other,
    };
    set.find_syntax_by_token(token)
        .filter(|syntax| syntax.name != "Plain Text")
}

/// Replace each fenced block with a known language by its highlighted HTML.
/// Every other event passes through untouched.
pub fn code_blocks<'a>(events: impl IntoIterator<Item = Event<'a>>) -> Vec<Event<'a>> {
    let mut out = Vec::new();
    let mut open: Option<(&'static SyntaxReference, String, String)> = None;
    for event in events {
        match (&mut open, event) {
            (None, Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info)))) => {
                match syntax_for(&info) {
                    Some(syntax) => open = Some((syntax, info.to_string(), String::new())),
                    None => out.push(Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info)))),
                }
            }
            (Some((_, _, code)), Event::Text(text)) => code.push_str(&text),
            (Some(_), Event::End(TagEnd::CodeBlock)) => {
                let (syntax, info, code) = open.take().expect("inside a block");
                out.push(Event::Html(CowStr::from(render(syntax, &info, &code))));
            }
            (_, other) => out.push(other),
        }
    }
    out
}

fn render(syntax: &SyntaxReference, info: &str, code: &str) -> String {
    let mut generator = ClassedHTMLGenerator::new_with_class_style(
        syntax,
        syntaxes(),
        ClassStyle::SpacedPrefixed { prefix: "hl-" },
    );
    for line in LinesWithEndings::from(code) {
        // A grammar that cannot parse a line is no reason to lose the code;
        // the block is shown plain instead.
        if generator
            .parse_html_for_line_which_includes_newline(line)
            .is_err()
        {
            return format!("<pre><code>{}</code></pre>\n", crate::html::escape(code));
        }
    }
    let language = info.split([' ', ',', '{']).next().unwrap_or_default();
    format!(
        "<pre class=\"hl\"><code class=\"language-{}\">{}</code></pre>\n",
        crate::html::escape(language),
        generator.finalize()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{Parser, html};

    fn render(markdown: &str) -> String {
        let mut out = String::new();
        html::push_html(&mut out, code_blocks(Parser::new(markdown)).into_iter());
        out
    }

    #[test]
    fn a_fence_with_a_known_language_is_highlighted_by_class() {
        let out = render("```rust\nfn main() { let x = \"hi\"; }\n```\n");
        assert!(
            out.contains("<pre class=\"hl\"><code class=\"language-rust\">"),
            "{out}"
        );
        assert!(out.contains("hl-string"), "{out}");
        assert!(
            out.contains("hl-storage") || out.contains("hl-keyword"),
            "{out}"
        );
        // Classes only: the colours belong to the stylesheet.
        assert!(!out.contains("style="), "{out}");
    }

    #[test]
    fn plain_and_unknown_fences_are_left_alone() {
        let plain = render("```\n$ cairns check\n```\n");
        assert!(plain.contains("<pre><code>"), "{plain}");
        let unknown = render("```klingon\nqapla'\n```\n");
        assert!(
            unknown.contains("<pre><code class=\"language-klingon\">"),
            "{unknown}"
        );
    }

    #[test]
    fn toml_has_a_grammar_of_its_own() {
        let out = render("```toml\n# note\n[area]\nspec = \"the format\"\n```\n");
        assert!(out.contains("language-toml"), "{out}");
        for scope in ["hl-comment", "hl-section", "hl-tag", "hl-string"] {
            assert!(out.contains(scope), "{scope} missing: {out}");
        }
    }

    #[test]
    fn the_code_is_escaped() {
        let out = render("```python\nprint(\"<b>\")\n```\n");
        assert!(out.contains("&lt;b&gt;"), "{out}");
        assert!(!out.contains("<b>"), "{out}");
    }
}
