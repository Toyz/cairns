//! Code references, resolved against the repository.
//!
//! `[[src/a.rs#parse_header]]` names code by its definition rather than by line
//! numbers, because lines move and names mostly do not. Finding it means
//! reading the file - from the working tree, or from git for an `@rev` - and
//! that is the binary's job: the renderer reads no files, and a renderer
//! elsewhere has no repository to read.

use cairns_core::entry::CodeRef;
use cairns_core::log::ResolvedCode;
use std::collections::BTreeMap;
use std::path::Path;

/// Every code reference in the log - entry prose, reference pages, entries'
/// `files:` - resolved once each, keyed by `CodeRef::key`.
pub fn resolve_all(root: &Path, log: &cairns_core::Log) -> BTreeMap<String, ResolvedCode> {
    let mut wanted: BTreeMap<String, (CodeRef, bool)> = BTreeMap::new();
    let mut add = |code: CodeRef| {
        let embed = code.embed;
        wanted
            .entry(code.key())
            .and_modify(|(_, text)| *text |= embed)
            .or_insert((code, embed));
    };
    for entry in &log.entries {
        cairns_core::entry::code_references(&entry.body)
            .into_iter()
            .for_each(&mut add);
        // Only a file named with its lines or a definition needs finding; a
        // plain path is linked as it is, as it always was.
        for file in &entry.files {
            if let Some(code) = CodeRef::parse(file)
                && (code.lines.is_some() || code.symbol.is_some())
            {
                add(code);
            }
        }
    }
    for doc in &log.docs {
        cairns_core::entry::code_references(&doc.body)
            .into_iter()
            .for_each(&mut add);
    }
    wanted
        .into_iter()
        .map(|(key, (code, text))| (key, resolve(root, &code, text)))
        .collect()
}

/// One reference: where it is, and the code when `with_text`.
pub fn resolve(root: &Path, code: &CodeRef, with_text: bool) -> ResolvedCode {
    let mut resolved = ResolvedCode {
        path: code.path.clone(),
        lines: code.lines,
        rev: code.rev.clone(),
        ..ResolvedCode::default()
    };
    // A directory is found by being there; it has no lines to find.
    if code.rev.is_none()
        && code.symbol.is_none()
        && code.lines.is_none()
        && root.join(&code.path).is_dir()
    {
        return resolved;
    }
    let source = match read(root, code) {
        Ok(source) => source,
        Err(why) => {
            resolved.missing = Some(why);
            return resolved;
        }
    };
    let lines: Vec<&str> = source.lines().collect();
    let span = match (&code.symbol, code.lines) {
        (Some(symbol), _) => match find_definition(&lines, symbol) {
            Some(span) => Some(span),
            None => {
                resolved.missing = Some(format!("nothing named {symbol} is defined in it"));
                return resolved;
            }
        },
        (None, Some((start, end))) => {
            if start as usize > lines.len() {
                resolved.missing = Some(format!(
                    "line {start} is past its end ({} lines)",
                    lines.len()
                ));
                return resolved;
            }
            Some((start, end.min(lines.len() as u32)))
        }
        (None, None) => None,
    };
    resolved.lines = span;
    if with_text {
        let (start, end) = span.unwrap_or((1, lines.len() as u32));
        // A whole file embedded is a mistake more often than a wish; it is
        // cut at a screenful and says so.
        let end = end.min(start + 199);
        resolved.text = Some(lines[(start - 1) as usize..end as usize].join("\n"));
    }
    resolved
}

fn read(root: &Path, code: &CodeRef) -> Result<String, String> {
    match &code.rev {
        None => std::fs::read_to_string(root.join(&code.path))
            .map_err(|_| "not in the repository".to_string()),
        Some(rev) => crate::git::show(root, rev, &code.path),
    }
}

/// The words that introduce a definition, across the languages a log is most
/// likely to point into.
const DEFINES: &[&str] = &[
    "fn",
    "struct",
    "enum",
    "trait",
    "impl",
    "mod",
    "type",
    "const",
    "static",
    "union",
    "macro_rules!",
    "class",
    "def",
    "function",
    "func",
    "interface",
    "let",
    "var",
    "typedef",
    "namespace",
    "for",
];

/// The line a name is defined on, and the end of its block: the first line
/// where it follows one of `DEFINES`, else the first where it appears at all.
fn find_definition(lines: &[&str], symbol: &str) -> Option<(u32, u32)> {
    let defines = |line: &str| {
        occurrences(line, symbol).any(|at| {
            let before = line[..at].trim_end();
            let word = before
                .rsplit(|c: char| c.is_whitespace() || c == '(' || c == '<' || c == ',')
                .next()
                .unwrap_or("");
            DEFINES.contains(&word)
        })
    };
    let start = lines.iter().position(|line| defines(line)).or_else(|| {
        lines
            .iter()
            .position(|line| occurrences(line, symbol).next().is_some())
    })?;
    Some((start as u32 + 1, block_end(lines, start) as u32 + 1))
}

/// Where `symbol` occurs in `line` as a whole identifier.
fn occurrences<'a>(line: &'a str, symbol: &'a str) -> impl Iterator<Item = usize> + 'a {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    line.match_indices(symbol)
        .map(|(at, _)| at)
        .filter(move |&at| {
            let before = line[..at].chars().next_back();
            let after = line[at + symbol.len()..].chars().next();
            !before.is_some_and(ident) && !after.is_some_and(ident)
        })
}

/// The last line of the block a definition opens: matching braces when it
/// opens one within a few lines, indentation when it ends with `:`, else the
/// line itself.
fn block_end(lines: &[&str], start: usize) -> usize {
    let mut depth = 0i32;
    let mut opened = false;
    for (at, line) in lines.iter().enumerate().skip(start) {
        let code = strip_strings_and_comments(line);
        for c in code.chars() {
            match c {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
        if opened && depth <= 0 {
            return at;
        }
        if !opened {
            let trimmed = code.trim_end();
            if trimmed.ends_with(';') && at == start {
                return at;
            }
            // Python and the like: a block by indentation.
            if trimmed.ends_with(':') {
                let indent = indentation(line);
                let mut end = at;
                for (next, later) in lines.iter().enumerate().skip(at + 1) {
                    if later.trim().is_empty() {
                        continue;
                    }
                    if indentation(later) <= indent {
                        break;
                    }
                    end = next;
                }
                return end;
            }
            if at >= start + 4 {
                return start;
            }
        }
    }
    if opened { lines.len() - 1 } else { start }
}

fn indentation(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// A line with its string literals and `//` comment blanked, so a brace in
/// either is not counted. Rough, and enough for finding where a block ends.
fn strip_strings_and_comments(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                if c == '\\' {
                    chars.next();
                } else if c == q {
                    quote = None;
                }
                out.push(' ');
            }
            None => {
                if c == '"'
                    || c == '\''
                        && chars
                            .peek()
                            .is_some_and(|n| *n != '_' && !n.is_alphabetic())
                {
                    quote = Some(c);
                    out.push(' ');
                } else if (c == '/' && chars.peek() == Some(&'/'))
                    || (c == '#' && out.trim().is_empty() && !line.trim_start().starts_with("#["))
                {
                    // A comment: `//`, or `#` at the start of a line that is
                    // not a Rust attribute.
                    break;
                } else {
                    out.push(c);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUST: &str = "use x;\n\n/// Doc.\npub fn parse_header(input: &str) -> Header {\n    let s = \"{\";\n    if input.is_empty() {\n        return Header::default();\n    }\n    Header { len: 1 }\n}\n\nfn other() {}\n";

    #[test]
    fn a_rust_function_is_found_by_name_and_its_block_by_braces() {
        let lines: Vec<&str> = RUST.lines().collect();
        assert_eq!(find_definition(&lines, "parse_header"), Some((4, 10)));
        assert_eq!(find_definition(&lines, "other"), Some((12, 12)));
        assert_eq!(find_definition(&lines, "missing"), None);
    }

    #[test]
    fn a_python_function_ends_where_its_indentation_does() {
        let lines: Vec<&str> = "import os\n\ndef walk(root):\n    for x in root:\n\n        yield x\n\ndef next_one():\n    pass\n"
            .lines()
            .collect();
        assert_eq!(find_definition(&lines, "walk"), Some((3, 6)));
    }

    #[test]
    fn a_definition_is_preferred_over_a_mention() {
        let lines: Vec<&str> =
            "// see Header below\nlet h = Header::new();\nstruct Header {\n    len: u32,\n}\n"
                .lines()
                .collect();
        assert_eq!(find_definition(&lines, "Header"), Some((3, 5)));
    }
}
