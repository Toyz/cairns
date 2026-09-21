//! A stdio MCP server, so a worklog is readable by a model with no shell.
//!
//! The protocol is newline-delimited JSON-RPC 2.0 on stdin and stdout. It is
//! hand-rolled rather than taken from an SDK: the surface used here is small,
//! synchronous, and wants no async runtime, and the cost of that choice is that
//! protocol drift is ours to track.
//!
//! **Nothing may be written to stdout except protocol messages.** Diagnostics
//! go to stderr; a stray `println!` corrupts the session.
//!
//! In a host that already has a shell this adds little - the skill and the CLI
//! cover it. It earns its place where there is no shell, and in `resources`,
//! which let a model pull one entry into context instead of reading the log.

use crate::{read_entries, write_index};
use cairns_core::{Config, Log, log};
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

/// The protocol version answered when a client offers none. A client that names
/// one gets it echoed back: this server's surface - tools and resources - has
/// been stable across versions, and refusing an unrecognised newer version
/// would break hosts for no benefit.
const PROTOCOL: &str = "2025-06-18";

const READ_ONLY_NOTE: &str =
    "This server is read-only. Start it with `cairns mcp --write` to allow writing entries.";

pub fn serve(root: PathBuf, config: Config, writable: bool) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };

        // A notification has no id and takes no response.
        let Some(id) = message.get("id").cloned() else {
            continue;
        };
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));

        let response = match dispatch(&root, &config, writable, method, &params) {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err(failure) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": failure.code, "message": failure.message }
            }),
        };
        writeln!(
            stdout,
            "{}",
            serde_json::to_string(&response).unwrap_or_default()
        )?;
        stdout.flush()?;
    }
    Ok(())
}

#[derive(Debug)]
pub struct Failure {
    pub code: i32,
    pub message: String,
}

fn invalid(message: impl Into<String>) -> Failure {
    Failure {
        code: -32602,
        message: message.into(),
    }
}

fn internal(message: impl Into<String>) -> Failure {
    Failure {
        code: -32603,
        message: message.into(),
    }
}

pub fn dispatch(
    root: &Path,
    config: &Config,
    writable: bool,
    method: &str,
    params: &Value,
) -> Result<Value, Failure> {
    match method {
        "initialize" => {
            let version = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(PROTOCOL);
            Ok(json!({
                "protocolVersion": version,
                "capabilities": { "tools": {}, "resources": {} },
                "serverInfo": { "name": "cairns", "version": env!("CARGO_PKG_VERSION") },
                "instructions": format!(
                    "The worklog for {}. Entries are numbered and append-only: an entry that \
                     turns out to be wrong is corrected by a later one that names it in \
                     `supersedes`, never by editing it. {}",
                    config.project.name,
                    if writable { "Writing entries is allowed." } else { READ_ONLY_NOTE }
                ),
            }))
        }

        "ping" => Ok(json!({})),

        "tools/list" => Ok(json!({ "tools": tools(config, writable) })),

        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match call(root, config, writable, name, &arguments) {
                Ok(text) => Ok(json!({ "content": [{ "type": "text", "text": text }] })),
                // A tool that fails reports through the result, not as a
                // protocol error: the model is meant to read it and adjust.
                Err(failure) => Ok(json!({
                    "content": [{ "type": "text", "text": failure.message }],
                    "isError": true
                })),
            }
        }

        "resources/list" => {
            let built = build(root, config)?;
            let mut resources = vec![
                json!({ "uri": "worklog://index", "name": "Index",
                        "description": "Every entry, newest last.", "mimeType": "text/markdown" }),
                json!({ "uri": "worklog://open", "name": "Open questions",
                        "description": "Everything the log has not closed out.",
                        "mimeType": "text/markdown" }),
                json!({ "uri": "worklog://log.json", "name": "The whole log",
                        "description": "Every entry with its metadata, backlinks and hashes.",
                        "mimeType": "application/json" }),
            ];
            for entry in &built.entries {
                resources.push(json!({
                    "uri": format!("worklog://entry/{}", entry.number),
                    "name": format!("{}. {}", entry.number, entry.title),
                    "description": entry.summary.clone().unwrap_or_default(),
                    "mimeType": "text/markdown",
                }));
            }
            Ok(json!({ "resources": resources }))
        }

        "resources/read" => {
            let uri = params
                .get("uri")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let (mime, text) = resource(root, config, uri)?;
            Ok(json!({ "contents": [{ "uri": uri, "mimeType": mime, "text": text }] }))
        }

        "prompts/list" => Ok(json!({ "prompts": [] })),

        _ => Err(Failure {
            code: -32601,
            message: format!("no method {method:?}"),
        }),
    }
}

fn build(root: &Path, config: &Config) -> Result<Log, Failure> {
    let entries = read_entries(root, config).map_err(|e| internal(e.to_string()))?;
    Ok(Log::build(config, entries, None))
}

fn tools(config: &Config, writable: bool) -> Vec<Value> {
    let areas: Vec<&str> = config.areas.iter().map(|area| area.name.as_str()).collect();
    let mut tools = vec![
        json!({
            "name": "worklog_list",
            "description": "List every entry: number, title, date, areas and summary.",
            "inputSchema": { "type": "object", "properties": {} },
        }),
        json!({
            "name": "worklog_read",
            "description": "Read one entry in full, with the entries that correct it or that it corrects.",
            "inputSchema": {
                "type": "object",
                "properties": { "number": { "type": "integer", "description": "The entry number." } },
                "required": ["number"],
            },
        }),
        json!({
            "name": "worklog_search",
            "description": "Find entries whose title, summary, areas or prose contain some text.",
            "inputSchema": {
                "type": "object",
                "properties": { "query": { "type": "string" } },
                "required": ["query"],
            },
        }),
        json!({
            "name": "worklog_open",
            "description": "Every unresolved question in the log, with the entry that raised it.",
            "inputSchema": { "type": "object", "properties": {} },
        }),
        json!({
            "name": "worklog_check",
            "description": "Validate the log: numbering, front matter, filenames, index freshness.",
            "inputSchema": { "type": "object", "properties": {} },
        }),
    ];

    if writable {
        tools.push(json!({
            "name": "worklog_new",
            "description": "Write a new entry and refresh the index. Lead with the finding, \
                            keep numbers and names exact, and record dead ends - they are the \
                            most valuable entries. End with what is still unknown.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Short, in plain words." },
                    "area": {
                        "type": "array", "items": { "type": "string", "enum": areas },
                        "description": "One or more. Say so when the work genuinely sits in two.",
                    },
                    "body": { "type": "string", "description": "The prose, as markdown. No heading - the title becomes it." },
                    "still_unknown": { "type": "string", "description": "What remains open, or \"nothing\"." },
                    "files": { "type": "array", "items": { "type": "string" } },
                    "supersedes": {
                        "type": "array", "items": { "type": "integer" },
                        "description": "Entries this one corrects. Set it whenever you overturn a claim.",
                    },
                },
                "required": ["title", "area", "body"],
            },
        }));
    }
    tools
}

fn call(
    root: &Path,
    config: &Config,
    writable: bool,
    name: &str,
    arguments: &Value,
) -> Result<String, Failure> {
    match name {
        "worklog_list" => {
            let built = build(root, config)?;
            let mut out = String::new();
            for entry in &built.entries {
                out.push_str(&format!(
                    "{}. {} ({}, {})\n    {}\n",
                    entry.number,
                    entry.title,
                    entry.date,
                    entry.areas.join(", "),
                    entry.summary.clone().unwrap_or_default()
                ));
            }
            Ok(out)
        }

        "worklog_read" => {
            let number = arguments
                .get("number")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid("`number` is required"))? as u32;
            let built = build(root, config)?;
            let entry = built
                .entries
                .iter()
                .find(|entry| entry.number == number)
                .ok_or_else(|| invalid(format!("no entry {number}")))?;

            let mut out = String::new();
            if !entry.superseded_by.is_empty() {
                out.push_str(&format!(
                    "NOTE: something claimed here was corrected by entry {}.\n\n",
                    entry
                        .superseded_by
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !entry.supersedes.is_empty() {
                out.push_str(&format!(
                    "This entry corrects entry {}.\n\n",
                    entry
                        .supersedes
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            out.push_str(&entry.body);
            Ok(out)
        }

        "worklog_search" => {
            let query = arguments
                .get("query")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("`query` is required"))?
                .to_lowercase();
            let built = build(root, config)?;
            let mut out = String::new();
            for entry in &built.entries {
                let haystack = format!(
                    "{} {} {} {}",
                    entry.title,
                    entry.summary.clone().unwrap_or_default(),
                    entry.areas.join(" "),
                    entry.body
                )
                .to_lowercase();
                if haystack.contains(&query) {
                    out.push_str(&format!("{}. {}\n", entry.number, entry.title));
                }
            }
            if out.is_empty() {
                out.push_str("nothing matched");
            }
            Ok(out)
        }

        "worklog_open" => {
            let built = build(root, config)?;
            if built.open_questions.is_empty() {
                return Ok("nothing open".into());
            }
            Ok(built
                .open_questions
                .iter()
                .map(|question| format!("{}. {}", question.entry, question.text))
                .collect::<Vec<_>>()
                .join("\n"))
        }

        "worklog_check" => {
            let entries = read_entries(root, config).map_err(|e| internal(e.to_string()))?;
            let problems = log::problems(config, &entries);
            if problems.is_empty() {
                Ok("ok".into())
            } else {
                Ok(problems.join("\n"))
            }
        }

        "worklog_new" if writable => new_entry(root, config, arguments),
        "worklog_new" => Err(invalid(READ_ONLY_NOTE)),

        _ => Err(invalid(format!("no tool {name:?}"))),
    }
}

fn new_entry(root: &Path, config: &Config, arguments: &Value) -> Result<String, Failure> {
    let strings = |key: &str| -> Vec<String> {
        arguments
            .get(key)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    };

    let title = arguments
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| invalid("`title` is required"))?;
    let areas = strings("area");
    if areas.is_empty() {
        return Err(invalid("`area` needs at least one area"));
    }
    for area in &areas {
        if !config.knows_area(area) {
            let known: Vec<&str> = config.areas.iter().map(|a| a.name.as_str()).collect();
            return Err(invalid(format!(
                "unknown area {area:?}; cairns.toml declares {}",
                known.join(", ")
            )));
        }
    }
    let body = arguments
        .get("body")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|body| !body.is_empty())
        .ok_or_else(|| invalid("`body` is required"))?;

    let dir = root.join(&config.paths.entries);
    let number = crate::next_number(&dir).map_err(|e| internal(e.to_string()))?;
    let path = dir.join(format!(
        "{number:04}-{}.md",
        cairns_core::entry::slugify(title)
    ));
    if path.exists() {
        return Err(internal(format!("{} already exists", path.display())));
    }

    let mut front = format!(
        "---\nnumber: {number}\ntitle: {title}\ndate: {}\narea: {}\n",
        jiff::Zoned::now().date(),
        areas.join(", ")
    );
    let files = strings("files");
    if !files.is_empty() {
        front.push_str(&format!("files: {}\n", files.join(", ")));
    }
    let supersedes: Vec<String> = arguments
        .get("supersedes")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_u64)
                .map(|n| n.to_string())
                .collect()
        })
        .unwrap_or_default();
    if !supersedes.is_empty() {
        front.push_str(&format!("supersedes: {}\n", supersedes.join(", ")));
    }

    let unknown = arguments
        .get("still_unknown")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or("nothing");

    std::fs::write(
        &path,
        format!("{front}---\n\n# {number}. {title}\n\n{body}\n\n**Still unknown:** {unknown}\n"),
    )
    .map_err(|e| internal(e.to_string()))?;

    write_index(root, config).map_err(|e| internal(e.to_string()))?;
    Ok(format!("wrote entry {number}: {}", path.display()))
}

fn resource(root: &Path, config: &Config, uri: &str) -> Result<(&'static str, String), Failure> {
    let built = build(root, config)?;
    match uri {
        "worklog://index" => Ok((
            "text/markdown",
            cairns_site::render_index(&built, config.index.header.as_deref()),
        )),
        "worklog://open" => Ok((
            "text/markdown",
            built
                .open_questions
                .iter()
                .map(|question| format!("- **{}**: {}", question.entry, question.text))
                .collect::<Vec<_>>()
                .join("\n"),
        )),
        "worklog://log.json" => Ok((
            "application/json",
            serde_json::to_string_pretty(&built).map_err(|e| internal(e.to_string()))?,
        )),
        _ => {
            let number: u32 = uri
                .strip_prefix("worklog://entry/")
                .and_then(|rest| rest.parse().ok())
                .ok_or_else(|| invalid(format!("no resource {uri:?}")))?;
            built
                .entries
                .iter()
                .find(|entry| entry.number == number)
                .map(|entry| ("text/markdown", entry.body.clone()))
                .ok_or_else(|| invalid(format!("no entry {number}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config::parse(
            "spec_version = 1\n[project]\nname = \"P\"\nslug = \"p\"\n\
             [[area]]\nname = \"spec\"\n[[area]]\nname = \"core\"\n",
        )
        .unwrap()
    }

    fn call(method: &str, params: Value, writable: bool) -> Result<Value, Failure> {
        dispatch(
            Path::new("/nonexistent"),
            &config(),
            writable,
            method,
            &params,
        )
    }

    #[test]
    fn initialize_echoes_the_version_the_client_asked_for() {
        // A client naming a version this build has never heard of still gets a
        // working session; the surface used here does not change between them.
        let result = call(
            "initialize",
            json!({ "protocolVersion": "2099-01-01" }),
            false,
        )
        .unwrap();
        assert_eq!(result["protocolVersion"], "2099-01-01");
        assert_eq!(result["serverInfo"]["name"], "cairns");

        let bare = call("initialize", json!({}), false).unwrap();
        assert_eq!(bare["protocolVersion"], PROTOCOL);
    }

    #[test]
    fn writing_is_absent_from_the_tool_list_until_it_is_allowed() {
        let names = |writable| {
            let result = call("tools/list", json!({}), writable).unwrap();
            result["tools"]
                .as_array()
                .unwrap()
                .iter()
                .map(|tool| tool["name"].as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        };
        assert!(!names(false).contains(&"worklog_new".to_string()));
        assert!(names(true).contains(&"worklog_new".to_string()));
        assert!(names(false).contains(&"worklog_read".to_string()));
    }

    #[test]
    fn a_read_only_server_refuses_to_write_and_says_how_to_change_that() {
        let result = call(
            "tools/call",
            json!({ "name": "worklog_new", "arguments": { "title": "t", "area": ["spec"], "body": "b" } }),
            false,
        )
        .unwrap();
        assert_eq!(result["isError"], true);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("--write"), "{text}");
    }

    #[test]
    fn the_new_entry_tool_offers_only_the_areas_this_project_declares() {
        let result = call("tools/list", json!({}), true).unwrap();
        let new = result["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "worklog_new")
            .unwrap();
        let areas = &new["inputSchema"]["properties"]["area"]["items"]["enum"];
        assert_eq!(areas, &json!(["spec", "core"]));
    }

    #[test]
    fn an_unknown_method_is_a_protocol_error_but_an_unknown_tool_is_not() {
        // The distinction matters: a model should read a bad tool name and try
        // again, where a bad method means the client and server disagree.
        let failure = call("bogus/method", json!({}), false).unwrap_err();
        assert_eq!(failure.code, -32601);

        let result = call("tools/call", json!({ "name": "nope" }), false).unwrap();
        assert_eq!(result["isError"], true);
    }
}
