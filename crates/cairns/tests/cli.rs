//! The commands, run as a user runs them: the built binary, in a directory of
//! its own, with nothing but its arguments and stdin.
//!
//! The unit tests cover the parts; these cover what the parts were for. A
//! refactor that keeps every unit test passing and changes what `cairns new`
//! writes is caught here and nowhere else.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A project in a directory of its own, removed when the test is done.
struct Project {
    root: PathBuf,
}

const CONFIG: &str = r#"spec_version = 1

[project]
name        = "Test"
slug        = "test"
description = "A log for the tests."

[site]
base_url = "https://example.com/test/"

[area]
spec = "the format"
core = "the library"
"#;

impl Project {
    fn new() -> Project {
        Project::with(CONFIG)
    }

    fn with(config: &str) -> Project {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "cairns-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        if !config.is_empty() {
            std::fs::write(root.join("cairns.toml"), config).unwrap();
        }
        Project { root }
    }

    fn run(&self, args: &[&str]) -> Output {
        self.run_with(args, "")
    }

    fn run_with(&self, args: &[&str], stdin: &str) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_cairns"))
            .args(args)
            .current_dir(&self.root)
            // What CI would set, unset, so the tests mean the same anywhere.
            .env_remove("GITHUB_REPOSITORY")
            .env_remove("GITHUB_SERVER_URL")
            .env_remove("CI_PAGES_URL")
            .env_remove("CI_PROJECT_PATH")
            .env_remove("CI_PROJECT_URL")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }

    /// Run, and insist it succeeded; its stdout.
    fn ok(&self, args: &[&str]) -> String {
        self.ok_with(args, "")
    }

    fn ok_with(&self, args: &[&str], stdin: &str) -> String {
        let out = self.run_with(args, stdin);
        assert!(
            out.status.success(),
            "cairns {args:?} failed:\n{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }

    /// Run, and insist it failed; its stderr.
    fn fails(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(
            !out.status.success(),
            "cairns {args:?} should have failed:\n{}",
            String::from_utf8_lossy(&out.stdout)
        );
        String::from_utf8(out.stderr).unwrap()
    }

    fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.root.join(path))
            .unwrap_or_else(|_| panic!("{path} is not there"))
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn exists(&self, path: &str) -> bool {
        self.root.join(path).exists()
    }

    fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn an_entry_is_written_whole_and_indexed() {
    let log = Project::new();
    let path = log.ok_with(
        &[
            "new",
            "The first thing",
            "--area",
            "spec, core",
            "--files",
            "src/a.rs",
            "--body",
            "-",
            "--unknown",
            "whether it holds",
        ],
        "What was found, with `code` and $dollars.\n",
    );
    assert!(
        path.trim().ends_with("worklog/0001-the-first-thing.md"),
        "{path}"
    );
    let entry = log.read("worklog/0001-the-first-thing.md");
    assert!(
        entry.starts_with("---\nnumber: 1\ntitle: The first thing\ndate: "),
        "{entry}"
    );
    assert!(entry.contains("\narea: spec, core\nfiles: src/a.rs\n---\n\n# 1. The first thing\n\n"));
    assert!(entry.contains(
        "What was found, with `code` and $dollars.\n\n**Still unknown:** whether it holds\n"
    ));

    let index = log.read("WORKLOG.md");
    assert!(index.contains("| 1 | [The first thing](worklog/0001-the-first-thing.md) |"));
    assert!(index.contains("## Still open"));
    assert_eq!(log.ok(&["check"]).trim(), "ok");

    // The second corrects and answers the first.
    log.ok_with(
        &[
            "new",
            "The second",
            "--area",
            "core",
            "--supersedes",
            "1",
            "--resolves",
            "1",
            "--body",
            "-",
            "--unknown",
            "nothing",
        ],
        "It was otherwise.",
    );
    let second = log.read("worklog/0002-the-second.md");
    assert!(
        second.contains("\nsupersedes: 1\nresolves: 1\n"),
        "{second}"
    );
    let index = log.read("WORKLOG.md");
    assert!(
        index.contains("- *corrected by [2](worklog/0002-the-second.md)*"),
        "{index}"
    );
    assert!(!index.contains("## Still open"), "{index}");
    assert_eq!(log.ok(&["next"]).trim(), "3");
}

#[test]
fn a_refused_entry_leaves_nothing_behind() {
    let log = Project::new();
    let problem = log.fails(&["new", "Nope", "--area", "nowhere", "--body", "x"]);
    assert!(problem.contains("unknown area \"nowhere\""), "{problem}");
    assert!(
        problem.contains("nowhere = \"what belongs here\""),
        "{problem}"
    );
    let problem = log.fails(&[
        "new",
        "Nope",
        "--area",
        "spec",
        "--resolves",
        "1.x",
        "--body",
        "x",
        "--unknown",
        "nothing",
    ]);
    assert!(problem.contains("resolves \"1.x\""), "{problem}");
    assert!(!log.exists("worklog/0001-nope.md"));
    assert_eq!(log.ok(&["next"]).trim(), "1");
}

#[test]
fn a_stale_index_is_reported_and_fixed() {
    let log = Project::new();
    log.ok(&[
        "new",
        "One",
        "--area",
        "spec",
        "--body",
        "Prose.",
        "--unknown",
        "nothing",
    ]);
    log.write("WORKLOG.md", "# Hand edited\n");
    let problem = log.fails(&["check"]);
    assert!(problem.contains("WORKLOG.md is stale"), "{problem}");
    assert!(
        log.ok(&["check", "--fix"])
            .contains("regenerated WORKLOG.md")
    );
    assert_eq!(log.ok(&["check"]).trim(), "ok");
}

#[test]
fn check_reports_what_is_wrong_with_an_entry() {
    let log = Project::new();
    log.write(
        "worklog/0001-a.md",
        "---\nnumber: 1\ntitle: A\ndate: 2026-09-20\narea: spec\n---\n\n# 1. A\n\nNo trailer.\n",
    );
    log.write(
        "worklog/0001-b.md",
        "---\nnumber: 1\ntitle: B\ndate: 2026-09-20\narea: spec\n---\n\n# 1. B\n\nX.\n\n**Still unknown:** nothing.\n",
    );
    log.ok(&["index"]);
    let problem = log.fails(&["check"]);
    assert!(
        problem.contains("has no `**Still unknown:**` line"),
        "{problem}"
    );
    assert!(problem.contains("is already used by"), "{problem}");

    let moved = log.ok(&["renumber", "worklog/0001-b.md"]);
    assert!(moved.contains('2'), "{moved}");
    assert!(log.exists("worklog/0002-b.md"));
    assert!(log.read("worklog/0002-b.md").contains("number: 2\n"));
    assert!(log.read("worklog/0002-b.md").contains("# 2. B"));
}

#[test]
fn the_reference_is_written_indexed_and_checked() {
    let log = Project::with(&format!(
        "{CONFIG}\n[docs]\ndir = \"docs\"\n\n[[docs.section]]\ndir = \"formats\"\ntitle = \"File formats\"\n\n\
         [[docs.field]]\nname = \"volumes\"\nvalues = [\"one\", \"all\"]\nrequired = true\n"
    ));
    log.ok(&[
        "new",
        "Found it",
        "--area",
        "spec",
        "--body",
        "Prose.",
        "--unknown",
        "nothing",
    ]);

    let refused = log.fails(&[
        "doc",
        "new",
        "The table",
        "--in",
        "formats",
        "--set",
        "volumes=two",
        "--body",
        "x",
    ]);
    assert!(refused.contains("volumes cannot be \"two\""), "{refused}");

    log.ok_with(
        &[
            "doc",
            "new",
            "The table",
            "--in",
            "formats",
            "--status",
            "partial",
            "--from",
            "1",
            "--set",
            "volumes=all",
            "--body",
            "-",
        ],
        "The table is a table.\n\n## Layout\n\nRows.\n",
    );
    let page = log.read("docs/formats/the-table.md");
    assert!(
        page.starts_with(
            "---\ntitle: The table\nstatus: partial\nworklog: 1\nvolumes: all\n---\n\n# The table\n"
        ),
        "{page}"
    );

    log.ok(&["doc", "index"]);
    let index = log.read("docs/README.md");
    assert!(index.contains("<!-- cairns:index -->"));
    assert!(index.contains("## File formats"));
    assert!(index.contains("| [The table](formats/the-table.md) | partial | all | [1](../worklog/0001-found-it.md) |"), "{index}");
    assert_eq!(log.ok(&["check"]).trim(), "ok");

    // Citing keeps the index current; a section citation goes under its heading.
    log.ok(&[
        "new",
        "More",
        "--area",
        "spec",
        "--body",
        "Prose.",
        "--unknown",
        "nothing",
    ]);
    log.ok(&[
        "doc",
        "cite",
        "formats/the-table",
        "2",
        "--section",
        "Layout",
    ]);
    assert!(
        log.read("docs/formats/the-table.md")
            .contains("## Layout\n<!-- worklog: 2 -->")
    );
    assert_eq!(log.ok(&["check"]).trim(), "ok");

    // A dead link, an undeclared folder and a bad value are each reported.
    log.write(
        "docs/extras/odd.md",
        "---\ntitle: Odd\nstatus: guess\nvolumes: none\n---\n\n# Odd\n\nSee [gone](gone.md).\n",
    );
    let problem = log.fails(&["check"]);
    assert!(
        problem.contains("links to gone.md, which does not exist"),
        "{problem}"
    );
    assert!(problem.contains("not a [[docs.section]]"), "{problem}");
    assert!(problem.contains("volumes has \"none\""), "{problem}");
    assert!(problem.contains("docs/README.md is stale"), "{problem}");
}

#[test]
fn a_build_writes_the_site_and_the_document() {
    let log = Project::new();
    log.ok(&[
        "new",
        "One",
        "--area",
        "spec",
        "--body",
        "Prose [[1]].",
        "--unknown",
        "whether",
    ]);
    log.ok(&["build", "--out", "site"]);
    for path in [
        "site/index.html",
        "site/log.json",
        "site/feed.xml",
        "site/ids.json",
        "site/open/index.html",
        "site/1-one/index.html",
        "site/1/index.html",
    ] {
        assert!(log.exists(path), "{path} was not written");
    }
    let json: serde_json::Value = serde_json::from_str(&log.read("site/log.json")).unwrap();
    assert_eq!(json["entries"][0]["title"], "One");
    assert_eq!(json["open_questions"][0]["entry"], 1);
}

#[test]
fn an_attached_image_is_built_beside_its_page_and_a_missing_one_is_reported() {
    let log = Project::new();
    log.write("shot.png", "png");
    let out = log.run(&[
        "new",
        "One",
        "--area",
        "spec",
        "--attach",
        "shot.png",
        "--body",
        "![kept](shot.png) ![typo](0001-one/shto.png)",
        "--unknown",
        "nothing",
    ]);
    assert!(out.status.success());
    let warned = String::from_utf8_lossy(&out.stderr);
    assert!(
        warned.contains("worklog/0001-one.md: shows 0001-one/shto.png, which does not exist"),
        "{warned}"
    );
    assert!(!warned.contains("shot.png,"), "{warned}");

    log.ok(&["build", "--out", "site"]);
    assert_eq!(log.read("site/1-one/shot.png"), "png");
    assert!(
        log.read("site/1-one/index.html")
            .contains("src=\"../1-one/shot.png\""),
        "the image does not point at its copy"
    );

    let problem = log.fails(&["check"]);
    assert!(problem.contains("shows 0001-one/shto.png"), "{problem}");
    log.write("worklog/0001-one/shto.png", "png");
    log.ok(&["check"]);
}

#[test]
fn init_writes_a_project_and_update_keeps_what_is_hand_written() {
    let log = Project::with("");
    log.ok(&["init"]);
    assert!(log.exists("cairns.toml"));
    let skill = ".claude/skills/worklog/SKILL.md";
    assert!(log.exists(skill));
    let written = log.read(skill);
    let marker = "<!-- cairns:project -->";
    assert!(written.contains(marker));
    let ours = format!(
        "{}{marker}\n\nOur own conventions.\n",
        written.split(marker).next().unwrap()
    );
    log.write(skill, &ours);
    log.ok(&["update"]);
    assert!(log.read(skill).ends_with("Our own conventions.\n"));
}

/// Speak MCP to `cairns mcp --write`: initialise, call one tool, read the reply.
fn mcp_call(log: &Project, tool: &str, arguments: &serde_json::Value) -> serde_json::Value {
    let messages = [
        serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
        serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": { "name": tool, "arguments": arguments }
        }),
    ];
    let input = messages
        .iter()
        .map(|m| m.to_string() + "\n")
        .collect::<String>();
    let out = log.ok_with(&["mcp", "--write"], &input);
    let reply = out
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|reply| reply["id"] == 2)
        .expect("a reply to the call");
    reply["result"].clone()
}

#[test]
fn an_entry_written_over_mcp_is_the_entry_cairns_new_writes() {
    let cli = Project::new();
    let mcp = Project::new();
    for log in [&cli, &mcp] {
        log.ok(&[
            "new",
            "First",
            "--area",
            "spec",
            "--body",
            "One.",
            "--unknown",
            "whether",
        ]);
    }
    cli.ok(&[
        "new",
        "Second",
        "--area",
        "spec, core",
        "--files",
        "a.rs, b.rs",
        "--supersedes",
        "1",
        "--resolves",
        "1",
        "--body",
        "Two.",
        "--unknown",
        "nothing",
    ]);
    let result = mcp_call(
        &mcp,
        "worklog_new",
        &serde_json::json!({
            "title": "Second",
            "area": ["spec", "core"],
            "files": ["a.rs", "b.rs"],
            "supersedes": [1],
            "resolves": [1],
            "body": "Two.",
            "still_unknown": "nothing",
        }),
    );
    assert!(
        result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("wrote entry 2"),
        "{result}"
    );
    let path = "worklog/0002-second.md";
    assert_eq!(cli.read(path), mcp.read(path));
    assert_eq!(cli.read("WORKLOG.md"), mcp.read("WORKLOG.md"));

    // A refusal over MCP is the refusal the command gives.
    let refused = mcp_call(
        &mcp,
        "worklog_new",
        &serde_json::json!({ "title": "X", "area": ["nowhere"], "body": "x" }),
    );
    assert_eq!(refused["isError"], true, "{refused}");
    assert!(
        refused["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("unknown area"),
        "{refused}"
    );
    assert!(!mcp.path().join("worklog/0003-x.md").exists());
}
