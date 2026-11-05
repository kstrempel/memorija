//! The local, read-only web viewer (spec: `web-ui`).
//!
//! An axum app serving one embedded page at `/` plus read-only JSON
//! endpoints over the shared structured store scan (design D5) and the
//! memory files (via `paths::resolve`). Nothing here can modify memory
//! files or version records.

use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tokio_stream::StreamExt;

use crate::cli;
use crate::inspect::{self, Timespan};

mod markdown;

/// How often the version store is checked for movement (design D4).
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// The embedded main page (design D3): one file with inline CSS/JS, no
/// front-end build step.
const PAGE: &str = include_str!("page.html");

/// State shared by all handlers: the memory root the viewer reads from and
/// the latest feed snapshot pushed by the store poller (design D4).
#[derive(Clone)]
struct WebState {
    root: PathBuf,
    events: watch::Receiver<String>,
}

/// The axum app of the viewer.
pub fn app(root: PathBuf) -> Router {
    // The poller's baseline is taken before the channel is created, so no
    // store movement between the two can be missed.
    let initial_signal = store_signal(&crate::versions::store_dir(&root));
    let (events, receiver) = watch::channel(changes_json(&root));
    tokio::spawn(poll_store(root.clone(), initial_signal, events));
    Router::new()
        .route("/", get(index))
        .route("/api/changes", get(api_changes))
        .route("/api/file", get(api_file))
        .route("/api/events", get(api_events))
        .with_state(WebState {
            root,
            events: receiver,
        })
}

/// A movement signal of the version store (design D4): the newest
/// modification time and the total size across its JSONL files. A cheap
/// metadata-only read; an append always grows a file, so any recorded
/// change moves the pair.
fn store_signal(store: &Path) -> (u128, u64) {
    let mut newest_mtime = 0u128;
    let mut total_size = 0u64;
    if let Ok(entries) = std::fs::read_dir(store) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.extension().is_some_and(|ext| ext == "jsonl") {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            total_size += meta.len();
            if let Some(nanos) = meta
                .modified()
                .ok()
                .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|since_epoch| since_epoch.as_nanos())
            {
                newest_mtime = newest_mtime.max(nanos);
            }
        }
    }
    (newest_mtime, total_size)
}

/// The current feed as the SSE event payload: the same JSON shape
/// `GET /api/changes` returns, or an error object the page can show.
fn changes_json(root: &Path) -> String {
    match inspect::scan_changes(root, Some(&Timespan::All)) {
        Ok(changes) => serde_json::to_string(&ChangesResponse { changes })
            .expect("ChangeRecord serializes as JSON"),
        Err(message) => serde_json::to_string(&ErrorResponse { error: message })
            .expect("ErrorResponse serializes as JSON"),
    }
}

/// The ~1s store poll (design D4): emit the latest feed whenever the store
/// moves. Runs for the server's lifetime; a scan error still updates the
/// watchers so the page can show it.
async fn poll_store(root: PathBuf, mut signal: (u128, u64), events: watch::Sender<String>) {
    let store = crate::versions::store_dir(&root);
    loop {
        tokio::time::sleep(POLL_INTERVAL).await;
        let next = store_signal(&store);
        if next == signal {
            continue;
        }
        signal = next;
        let _ = events.send(changes_json(&root));
    }
}

/// Run the `web` subcommand: resolve the memory root (never creating it — a
/// fresh machine shows the empty state), bind, and serve until interrupted.
pub async fn run_web(args: &cli::WebArgs) -> Result<(), Box<dyn std::error::Error>> {
    let root = crate::root::resolve()?;
    eprintln!("memorija: memory root: {}", root.display());
    if !args.host.is_loopback() {
        eprintln!(
            "Warning: the web viewer has no authentication and is listening on a non-loopback address ({}); anyone who can reach this address can read your memory files",
            args.host
        );
    }
    let address = SocketAddr::new(args.host, args.port);
    let listener = match tokio::net::TcpListener::bind(address).await {
        Ok(listener) => listener,
        Err(error) => return Err(format!("cannot bind {address}: {error}").into()),
    };
    let url = format!("http://{}/", display_host(args.host, args.port));
    eprintln!("memorija: web viewer at {url}");
    if !args.no_open {
        open_browser(&url);
    }
    let router = app(root);
    axum::serve(listener, router).await?;
    eprintln!("memorija: web viewer stopped");
    Ok(())
}

/// Host as it appears in a URL: bracketed for IPv6.
fn display_host(host: IpAddr, port: u16) -> String {
    match host {
        IpAddr::V4(ip) => format!("{ip}:{port}"),
        IpAddr::V6(ip) => format!("[{ip}]:{port}"),
    }
}

/// Open `url` in the system's default browser (design D7): spawn the
/// platform opener. Failure is non-fatal — the URL is printed regardless.
fn open_browser(url: &str) {
    let command = if cfg!(target_os = "macos") {
        std::process::Command::new("/usr/bin/open")
    } else if cfg!(target_os = "windows") {
        let mut command = std::process::Command::new("cmd");
        command.args(["/c", "start", ""]);
        command
    } else {
        std::process::Command::new("xdg-open")
    };
    open_with(command, url);
}

/// Spawn `command` pointed at `url`, reporting a failed spawn without
/// stopping the server (the URL is printed regardless).
fn open_with(mut command: std::process::Command, url: &str) {
    command.arg(url);
    if let Err(error) = command.spawn() {
        eprintln!("memorija: could not open the browser: {error}");
    }
}

/// `GET /api/events`: the live-update stream (design D4). Yields the current
/// feed immediately on connect, then the latest feed each time the store
/// poller detects movement. `EventSource` in the page reconnects on its own
/// if the stream drops.
async fn api_events(
    State(state): State<WebState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let updates = tokio_stream::wrappers::WatchStream::new(state.events.clone());
    let events = updates.map(|feed| Ok(Event::default().event("changes").data(feed)));
    Sse::new(events).keep_alive(KeepAlive::default())
}

async fn index() -> axum::response::Response {
    // Defense in depth (design D2): the page runs only inline style/script
    // and talks to this same origin — no remote origin can load.
    let mut response = Html(PAGE).into_response();
    response.headers_mut().insert(
        "Content-Security-Policy",
        axum::http::HeaderValue::from_static(
            "default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src 'self'",
        ),
    );
    response
}

#[derive(Serialize)]
struct ChangesResponse {
    changes: Vec<inspect::ChangeRecord>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

type ApiResult<T> = Result<(StatusCode, Json<T>), (StatusCode, Json<ErrorResponse>)>;

fn api_error(status: StatusCode, message: String) -> (StatusCode, Json<ErrorResponse>) {
    (status, Json(ErrorResponse { error: message }))
}

/// `GET /api/changes`: the version store's change log, newest first (design
/// D5). An empty or fresh machine returns an empty list, never an error.
async fn api_changes(State(state): State<WebState>) -> ApiResult<ChangesResponse> {
    let changes = match inspect::scan_changes(&state.root, Some(&Timespan::All)) {
        Ok(changes) => changes,
        Err(message) => return Err(api_error(StatusCode::INTERNAL_SERVER_ERROR, message)),
    };
    Ok((StatusCode::OK, Json(ChangesResponse { changes })))
}

#[derive(Deserialize)]
struct FileQuery {
    path: String,
}

#[derive(Serialize)]
struct FileResponse {
    /// The path as the page should keep addressing it (`/?path=…`).
    path: String,
    /// The file's content rendered from markdown into HTML (design D2).
    html: String,
    /// The latest recorded version of the file, if it has one.
    latest_version: Option<u64>,
}

/// `GET /api/file?path=…`: one memory file rendered as HTML, with its latest
/// recorded version. The path goes through the same `paths::resolve` sandbox
/// as every other surface (design D6), so traversal and symlink escapes are
/// refused, and a missing file is a clear 404.
async fn api_file(
    State(state): State<WebState>,
    Query(query): Query<FileQuery>,
) -> ApiResult<FileResponse> {
    let target = match crate::paths::resolve(&state.root, &query.path) {
        Ok(target) => target,
        Err(message) => return Err(api_error(StatusCode::BAD_REQUEST, message)),
    };
    if target.is_root {
        return Err(api_error(
            StatusCode::NOT_FOUND,
            format!(
                "The path {} is the memory root itself; pick a memory file",
                query.path
            ),
        ));
    }
    let meta = match std::fs::symlink_metadata(&target.path) {
        Ok(meta) => meta,
        Err(_) => {
            return Err(api_error(
                StatusCode::NOT_FOUND,
                format!("The path {} does not exist", query.path),
            ));
        }
    };
    if meta.is_dir() {
        return Err(api_error(
            StatusCode::NOT_FOUND,
            format!("The path {} is a directory; pick a memory file", query.path),
        ));
    }
    let content = match std::fs::read_to_string(&target.path) {
        Ok(content) => content,
        Err(error) => {
            return Err(api_error(
                StatusCode::NOT_FOUND,
                format!(
                    "The path {} is not a readable text file: {error}",
                    query.path
                ),
            ));
        }
    };
    let rel = crate::ops::rel_key(&state.root, &target)
        .map_err(|message| api_error(StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let records = crate::versions::read(&crate::versions::store_dir(&state.root), &rel)
        .map_err(|message| api_error(StatusCode::INTERNAL_SERVER_ERROR, message))?;
    Ok((
        StatusCode::OK,
        Json(FileResponse {
            path: rel,
            html: markdown::render(&content),
            latest_version: records.last().map(|record| record.seq),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::time::Duration;

    use crate::root;
    use crate::testutil::temp_dir;

    /// Serve the app on a random loopback port; returns the port.
    async fn spawn_app(root: PathBuf) -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            axum::serve(listener, app(root)).await.unwrap();
        });
        port
    }

    /// A blocking HTTP GET over a raw TCP connection (no client dependency).
    /// The server closes the connection (`Connection: close`), which ends
    /// the read.
    fn http_get(port: u16, path: &str) -> (u16, String) {
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut raw = String::new();
        stream.read_to_string(&mut raw).unwrap();
        let status: u16 = raw
            .split_whitespace()
            .nth(1)
            .expect("a status line")
            .parse()
            .expect("a numeric status");
        let body = raw
            .split_once("\r\n\r\n")
            .expect("headers and body")
            .1
            .to_string();
        (status, body)
    }

    /// Seed the store with records at fixed, distinct timestamps (written
    /// directly, like the `inspect` tests, for deterministic ordering).
    fn seed_store(root: &std::path::Path, records: &[(&str, u64, &str, &str, &str)]) {
        let store = crate::versions::store_dir(root);
        std::fs::create_dir_all(&store).unwrap();
        let mut by_path: std::collections::BTreeMap<&str, Vec<String>> = Default::default();
        for (path, seq, content, cause, at) in records {
            by_path.entry(path).or_default().push(
                serde_json::json!({
                    "path": path, "seq": seq, "content": content,
                    "cause": cause, "recorded_at": at,
                })
                .to_string(),
            );
        }
        for (path, lines) in by_path {
            std::fs::write(
                store.join(format!("{}.jsonl", crate::versions::encode_key(path))),
                lines.join("\n") + "\n",
            )
            .unwrap();
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn root_serves_the_embedded_page() {
        let root = root::root_in(&temp_dir("web-page"));
        let port = spawn_app(root).await;

        let (status, body) = http_get(port, "/");
        assert_eq!(status, 200);
        assert!(body.contains("<!DOCTYPE html"), "an HTML document: {body}");
        // The page's JS references all three API endpoints.
        for endpoint in ["/api/changes", "/api/file", "/api/events"] {
            assert!(body.contains(endpoint), "references {endpoint}: {body}");
        }
        assert!(
            body.contains("EventSource"),
            "live updates via EventSource: {body}"
        );
        // Defense in depth: the document ships a restrictive CSP.
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(
            stream,
            "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut raw = String::new();
        stream.read_to_string(&mut raw).unwrap();
        assert!(
            raw.to_ascii_lowercase().contains("content-security-policy"),
            "CSP header present: {raw}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn api_changes_returns_newest_first_json_for_a_seeded_store() {
        let root = root::root_in(&temp_dir("web-changes"));
        seed_store(
            &root,
            &[
                ("a.txt", 1, "a1\n", "create", "2027-01-01T10:00:00Z"),
                ("a.txt", 2, "a1\na2\n", "insert", "2027-01-05T12:30:00Z"),
                ("b.txt", 1, "b1\n", "create", "2027-01-03T08:00:00Z"),
            ],
        );
        let port = spawn_app(root).await;

        let (status, body) = http_get(port, "/api/changes");
        assert_eq!(status, 200, "{body}");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let changes = json["changes"].as_array().expect("a changes array");
        let paths: Vec<&str> = changes
            .iter()
            .map(|c| c["path"].as_str().unwrap())
            .collect();
        assert_eq!(paths, ["a.txt", "b.txt", "a.txt"], "newest first: {json}");
        assert_eq!(changes[0]["seq"], 2, "the a.txt edit: {json}");
        assert_eq!(changes[0]["cause"], "insert");
        assert_eq!(changes[0]["added"], 1, "+1 -0 against the previous version");
        assert_eq!(changes[0]["removed"], 0);
        assert_eq!(changes[0]["recorded_at"], "2027-01-05T12:30:00Z");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn api_changes_on_a_fresh_machine_is_an_empty_list() {
        let root = root::root_in(&temp_dir("web-changes-empty"));
        assert!(!root.exists(), "the viewer must not need the root to exist");
        let port = spawn_app(root).await;

        let (status, body) = http_get(port, "/api/changes");
        assert_eq!(status, 200, "{body}");
        assert_eq!(body, r#"{"changes":[]}"#, "an empty feed, not an error");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn api_file_renders_markdown_with_version_metadata() {
        let root = root::root_in(&temp_dir("web-file"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("notes.md"),
            "# Heading\n\n- item\n\n```rust\ncode\n```\n",
        )
        .unwrap();
        let port = spawn_app(root).await;

        let (status, body) = http_get(port, "/api/file?path=notes.md");
        assert_eq!(status, 200, "{body}");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["path"], "notes.md");
        let html = json["html"].as_str().unwrap();
        assert!(html.contains("<h1>Heading</h1>"), "heading: {html}");
        assert!(html.contains("<li>item</li>"), "list: {html}");
        assert!(html.contains("language-rust"), "code: {html}");
        assert_eq!(
            json["latest_version"],
            serde_json::Value::Null,
            "no records yet"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn api_file_reports_the_latest_recorded_version() {
        let home = temp_dir("web-file-version");
        let root = root::root_in(&home);
        root::ensure(&root).unwrap();
        crate::ops::run(&root, &memory_args("create", "/memories/notes.md", "one\n")).unwrap();
        crate::ops::run(&root, &memory_args("create", "/memories/notes.md", "two\n")).unwrap();
        let port = spawn_app(root).await;

        let (status, body) = http_get(port, "/api/file?path=notes.md");
        assert_eq!(status, 200, "{body}");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["latest_version"], 2, "{json}");
    }

    #[test]
    fn a_failed_browser_spawn_does_not_stop_the_server() {
        // The opener may be missing (headless machine, unusual platform);
        // `open_with` reports and returns instead of panicking.
        open_with(
            std::process::Command::new("memorija-nonexistent-opener-for-tests"),
            "http://127.0.0.1:8081/",
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn api_events_pushes_an_event_when_the_store_moves() {
        let home = temp_dir("web-events");
        let root = root::root_in(&home);
        root::ensure(&root).unwrap();
        crate::ops::run(&root, &memory_args("create", "/memories/first.md", "one\n")).unwrap();
        let port = spawn_app(root.clone()).await;

        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        write!(
            stream,
            "GET /api/events HTTP/1.1\r\nHost: localhost\r\nAccept: text/event-stream\r\n\r\n"
        )
        .unwrap();

        // The current feed arrives immediately on connect.
        read_until_contains(
            &mut stream,
            "first.md",
            Duration::from_secs(10),
            "initial feed",
        );

        // An agent records a change while the page is open; the event must
        // arrive within a few seconds, without a reconnect.
        crate::ops::run(
            &root,
            &memory_args("create", "/memories/second.md", "two\n"),
        )
        .unwrap();
        read_until_contains(
            &mut stream,
            "second.md",
            Duration::from_secs(10),
            "store movement",
        );
    }

    /// Read from the SSE stream until a chunk containing `needle` arrives or
    /// the deadline passes (the read timeout only paces the polling).
    fn read_until_contains(
        stream: &mut std::net::TcpStream,
        needle: &str,
        deadline: Duration,
        what: &str,
    ) {
        let start = std::time::Instant::now();
        let mut seen = String::new();
        loop {
            assert!(
                start.elapsed() < deadline,
                "no {what} event containing {needle:?} within {deadline:?}; got: {seen}"
            );
            let mut chunk = [0u8; 4096];
            match stream.read(&mut chunk) {
                Ok(0) => panic!("connection closed while waiting for {what}: {seen}"),
                Ok(n) => seen.push_str(&String::from_utf8_lossy(&chunk[..n])),
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        || error.kind() == std::io::ErrorKind::TimedOut =>
                {
                    continue;
                }
                Err(error) => panic!("read error while waiting for {what}: {error}"),
            }
            if seen.contains(needle) {
                return;
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn api_file_renders_script_in_a_memory_file_as_escaped_text() {
        let root = root::root_in(&temp_dir("web-file-xss"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("evil.md"),
            "# note\n\n<script>alert('xss')</script>\n",
        )
        .unwrap();
        let port = spawn_app(root).await;

        let (status, body) = http_get(port, "/api/file?path=evil.md");
        assert_eq!(status, 200, "{body}");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let html = json["html"].as_str().unwrap();
        assert!(
            !html.contains("<script>"),
            "no live tag in the rendered output: {html}"
        );
        assert!(
            html.contains("&lt;script&gt;"),
            "the tag shows as text: {html}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn api_file_refuses_paths_that_escape_the_root() {
        let home = temp_dir("web-file-escape");
        let root = root::root_in(&home);
        std::fs::create_dir_all(&root).unwrap();
        // A file outside the root and a symlink inside it pointing there.
        let outside = home.join("secrets.txt");
        std::fs::write(&outside, "classified\n").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("escape.md")).unwrap();
        let port = spawn_app(root).await;

        for path in ["/memories/../secrets.txt", "../secrets.txt", "escape.md"] {
            let requested = path.replace(' ', "%20");
            let (status, body) = http_get(port, &format!("/api/file?path={requested}"));
            assert_ne!(status, 200, "{path} must be refused: {body}");
            let json: serde_json::Value = serde_json::from_str(&body).unwrap();
            let message = json["error"].as_str().unwrap();
            assert!(
                message.contains("outside the memory root"),
                "{path}: {message}"
            );
            assert!(!body.contains("classified"), "no outside content: {body}");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn api_file_names_missing_files_clearly() {
        let root = root::root_in(&temp_dir("web-file-missing"));
        std::fs::create_dir_all(&root).unwrap();
        let port = spawn_app(root).await;

        let (status, body) = http_get(port, "/api/file?path=gone.md");
        assert_eq!(status, 404, "{body}");
        assert!(body.contains("gone.md"), "names the path: {body}");
        assert!(
            body.contains("does not exist"),
            "a clear not-found message: {body}"
        );
    }

    fn memory_args(command: &str, path: &str, file_text: &str) -> crate::args::MemoryArgs {
        crate::args::MemoryArgs {
            command: serde_json::from_str(&format!("\"{command}\"")).unwrap(),
            path: path.to_string(),
            file_text: Some(file_text.to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
            insert_text: None,
            new_path: None,
            view_range: None,
            version: None,
        }
    }
}
