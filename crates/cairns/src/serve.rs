//! `cairns serve` - the site, locally, rebuilt when the entries change.
//!
//! A hand-rolled HTTP/1.1 server over `TcpListener`. It serves one person on
//! one machine looking at their own worklog, so it is single-threaded, closes
//! every connection, and speaks the smallest subset of HTTP that a browser
//! will accept. Anything more would be a dependency bought for a preview.
//!
//! Pages served from here reload themselves when an entry changes. The signal
//! is a counter polled at [`RELOAD_PATH`] rather than an event stream: a
//! single-threaded server holding a stream open would be a server answering
//! nothing else. The script is injected on the way out and is never in what
//! `cairns build` writes.

use cairns_core::{Config, Log};
use cairns_site::Rendered;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Polled by the injected script; its value changes when the site is rebuilt.
const RELOAD_PATH: &str = "/_cairns/reload";

/// Injected into every page this server sends, and only this server.
const RELOAD_SCRIPT: &str = "<script>(function(){var seen=null;setInterval(function(){\
fetch('/_cairns/reload',{cache:'no-store'}).then(function(r){return r.text()}).then(function(n){\
if(seen===null){seen=n;return}if(n!==seen){location.reload()}}).catch(function(){})},600)})();\
</script>";

pub fn serve(
    root: PathBuf,
    config: Config,
    port: u16,
    open: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let address = format!("http://127.0.0.1:{}", listener.local_addr()?.port());

    let mut site = build(&root, &config)?;
    let mut built_at = newest(&root, &config);
    let mut generation: u64 = 0;

    // The version, because a server started before an upgrade keeps running
    // the old binary and renders the old site - which looks exactly like a
    // config change that did not take.
    println!(
        "cairns {} - {} files on {address}",
        env!("CARGO_PKG_VERSION"),
        site.files.len()
    );
    println!("pages reload themselves when an entry changes; ctrl-c to stop");
    if open {
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        let _ = std::process::Command::new(opener).arg(&address).status();
    }

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(_) => continue,
        };

        // Stat before serving rather than watching: one pass over the entries
        // costs less than a filesystem watcher costs to depend on, and a
        // preview that is ever stale is worse than one that is slow.
        let now = newest(&root, &config);
        if now != built_at {
            match build(&root, &config) {
                Ok(fresh) => {
                    site = fresh;
                    built_at = now;
                    generation += 1;
                    println!("rebuilt: {} files", site.files.len());
                }
                Err(problem) => eprintln!("cairns: {problem}"),
            }
        }

        if let Err(problem) = respond(&mut stream, &site, generation) {
            // A browser closing a connection early is not worth a line.
            if problem.kind() != std::io::ErrorKind::BrokenPipe {
                eprintln!("cairns: {problem}");
            }
        }
    }
    Ok(())
}

fn build(root: &Path, config: &Config) -> Result<Rendered, Box<dyn std::error::Error>> {
    let built: Log = crate::build_log(root, config, None)?;
    Ok(cairns_site::render(&built)?)
}

/// The newest modification time across everything the site is built from.
fn newest(root: &Path, config: &Config) -> Option<SystemTime> {
    let mut latest = None;
    let mut consider = |path: PathBuf| {
        if let Ok(stamp) = std::fs::metadata(&path).and_then(|meta| meta.modified()) {
            latest = latest.max(Some(stamp));
        }
    };
    consider(root.join("cairns.toml"));
    if let Some(readme) = &config.site.readme {
        consider(root.join(readme));
    }
    if let Ok(entries) = std::fs::read_dir(root.join(&config.paths.entries)) {
        for entry in entries.flatten() {
            consider(entry.path());
        }
    }
    latest
}

fn respond(stream: &mut TcpStream, site: &Rendered, generation: u64) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request = String::new();
    reader.read_line(&mut request)?;

    // Drain the headers so the client is not left writing into a closed socket.
    let mut header = String::new();
    while reader.read_line(&mut header)? > 2 {
        header.clear();
    }

    let mut parts = request.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or("/");
    let path = target.split(['?', '#']).next().unwrap_or("/");

    if method != "GET" && method != "HEAD" {
        return write(
            stream,
            405,
            "text/plain; charset=utf-8",
            b"method not allowed",
            false,
        );
    }

    if path == RELOAD_PATH {
        let body = generation.to_string();
        return write(
            stream,
            200,
            "text/plain; charset=utf-8",
            body.as_bytes(),
            method == "HEAD",
        );
    }

    let wanted = resolve(path);
    match site.files.iter().find(|file| file.path == wanted) {
        Some(file) if file.path.ends_with(".html") => {
            let page = with_reload(&file.bytes);
            write(stream, 200, mime(&file.path), &page, method == "HEAD")
        }
        Some(file) => write(stream, 200, mime(&file.path), &file.bytes, method == "HEAD"),
        None => {
            let body = format!("404: no {wanted}\n");
            write(
                stream,
                404,
                "text/plain; charset=utf-8",
                body.as_bytes(),
                method == "HEAD",
            )
        }
    }
}

/// A URL path to a file in the payload: `/` and `/open/` are directories with
/// an `index.html` in them, which is what the clean URLs depend on.
fn resolve(path: &str) -> String {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return "index.html".into();
    }
    if trimmed.ends_with('/') {
        return format!("{trimmed}index.html");
    }
    // A path with no extension is a clean URL that arrived without its slash.
    match Path::new(trimmed).extension() {
        Some(_) => trimmed.to_string(),
        None => format!("{trimmed}/index.html"),
    }
}

/// Put the reload script in just before the page ends.
fn with_reload(page: &[u8]) -> Vec<u8> {
    let Ok(text) = std::str::from_utf8(page) else {
        return page.to_vec();
    };
    match text.rfind("</body>") {
        Some(at) => format!("{}{RELOAD_SCRIPT}{}", &text[..at], &text[at..]).into_bytes(),
        None => page.to_vec(),
    }
}

fn mime(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("xml") => "application/atom+xml; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn write(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        _ => "Method Not Allowed",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\n\
         content-type: {content_type}\r\n\
         content-length: {}\r\n\
         cache-control: no-store\r\n\
         connection: close\r\n\r\n",
        body.len()
    )?;
    if !head_only {
        stream.write_all(body)?;
    }
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_url_path_finds_the_file_behind_it() {
        assert_eq!(resolve("/"), "index.html");
        assert_eq!(resolve("/open/"), "open/index.html");
        assert_eq!(resolve("/style.css"), "style.css");
        assert_eq!(resolve("/feed.xml"), "feed.xml");
        // A clean URL that arrived without its trailing slash.
        assert_eq!(resolve("/50-kreash-mix"), "50-kreash-mix/index.html");
    }

    #[test]
    fn the_reload_script_goes_in_the_page_but_never_in_the_build() {
        let page = b"<html><body><p>hi</p></body></html>";
        let served = String::from_utf8(with_reload(page)).unwrap();
        assert!(served.contains(RELOAD_PATH));
        assert!(served.ends_with("</body></html>"), "{served}");
        // Only this server injects it; what `build` writes has none of it.
        assert!(!String::from_utf8_lossy(page).contains("_cairns"));
    }

    #[test]
    fn a_page_without_a_body_close_is_left_alone() {
        let odd = b"not html at all";
        assert_eq!(with_reload(odd), odd.to_vec());
    }

    #[test]
    fn content_types_are_the_ones_a_browser_needs() {
        assert_eq!(mime("index.html"), "text/html; charset=utf-8");
        assert_eq!(mime("search.json"), "application/json; charset=utf-8");
        assert_eq!(mime("feed.xml"), "application/atom+xml; charset=utf-8");
        assert_eq!(mime("cairns"), "application/octet-stream");
    }
}
