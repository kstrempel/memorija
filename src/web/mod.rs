//! The local, read-only web viewer (spec: `web-ui`).
//!
//! An axum app serving one embedded shell (Changes, Files, Reader at their
//! own addresses) plus its stylesheet and client script, and read-only JSON
//! endpoints over the shared structured store scan (design D5) and the
//! memory files (via `paths::resolve`). Nothing here can modify memory files
//! or version records.

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

/// The embedded shell (design D2): one document, one stylesheet, two client
/// scripts, and the vendored diagram renderer — no front-end build step.
/// `prefs.js` is the preference bootstrap loaded from `<head>`, `app.js` the
/// client router and screen renderers, and the Mermaid bundle is fetched only
/// for a file that contains a diagram (design D5).
const INDEX_HTML: &str = include_str!("index.html");
const APP_CSS: &str = include_str!("app.css");
const APP_JS: &str = include_str!("app.js");
const PREFS_JS: &str = include_str!("prefs.js");
const MERMAID_JS: &str = include_str!("vendor/mermaid.min.js");

/// The viewer's Content-Security-Policy (design D2 of the web viewer and D2 of
/// `add-mermaid-diagram-rendering`): assets and connections from this origin
/// only, and no inline script or event handlers.
///
/// The one exception is `style-src`'s `'unsafe-inline'`: Mermaid writes a
/// `<style>` element and style attributes into the SVG it draws, and the
/// policy would otherwise blank the diagram (spec: `web-ui`, "Diagram content
/// cannot run code or reach the network", which keeps script inline-free). Do
/// not remove it without taking the diagrams with it, and do not widen
/// `script-src` to compensate for anything.
const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; base-uri 'none'; form-action 'none'";

/// State shared by all handlers: the memory root the viewer reads from and
/// the latest feed + index snapshot pushed by the poller (design D4).
#[derive(Clone)]
struct WebState {
    root: PathBuf,
    events: watch::Receiver<String>,
}

/// The axum app of the viewer.
pub fn app(root: PathBuf) -> Router {
    // The poller's baseline is taken before the channel is created, so no
    // movement between the two can be missed.
    let initial_signal = MovementSignal::of(&root);
    let (events, receiver) = watch::channel(updates_json(&root));
    tokio::spawn(poll_store(root.clone(), initial_signal, events));
    Router::new()
        .route("/", get(shell))
        .route("/changes", get(shell))
        .route("/files", get(shell))
        .route("/file", get(shell))
        .route("/app.css", get(app_css))
        .route("/app.js", get(app_js))
        .route("/prefs.js", get(prefs_js))
        .route("/mermaid.js", get(mermaid_js))
        .route("/api/changes", get(api_changes))
        .route("/api/files", get(api_files))
        .route("/api/file", get(api_file))
        .route("/api/events", get(api_events))
        .fallback(fallback)
        .with_state(WebState {
            root,
            events: receiver,
        })
}

/// A movement signal of the whole viewer surface (design D4): the version
/// store's newest modification time and total size across its JSONL files,
/// plus the memory root's top-level shape (entry count + newest mtime).
/// Cheap metadata-only reads; a recorded change always moves the store pair,
/// and a file created or removed without a record still moves the root pair.
#[derive(Clone, Copy, PartialEq, Eq)]
struct MovementSignal {
    store_mtime: u128,
    store_size: u64,
    root_entries: u64,
    root_mtime: u128,
}

impl MovementSignal {
    fn of(root: &Path) -> Self {
        let (store_mtime, store_size) = store_signal(&crate::versions::store_dir(root));
        let (root_entries, root_mtime) = root_signal(root);
        Self {
            store_mtime,
            store_size,
            root_entries,
            root_mtime,
        }
    }
}

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
            if let Some(nanos) = mtime_nanos(&meta) {
                newest_mtime = newest_mtime.max(nanos);
            }
        }
    }
    (newest_mtime, total_size)
}

/// The memory root's top-level shape (design D3): the number of entries and
/// the newest modification time among them. Metadata only; creating or
/// removing a top-level entry moves at least one of the two.
fn root_signal(root: &Path) -> (u64, u128) {
    let mut entries = 0u64;
    let mut newest_mtime = 0u128;
    if let Ok(dir) = std::fs::read_dir(root) {
        for entry in dir.flatten() {
            entries += 1;
            if let Ok(meta) = entry.metadata()
                && let Some(nanos) = mtime_nanos(&meta)
            {
                newest_mtime = newest_mtime.max(nanos);
            }
        }
    }
    (entries, newest_mtime)
}

fn mtime_nanos(meta: &std::fs::Metadata) -> Option<u128> {
    meta.modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|since_epoch| since_epoch.as_nanos())
}

/// The current feed plus the all-files index as the SSE event payload: the
/// same JSON shapes `GET /api/changes` and `GET /api/files` return, or an
/// error object the page can show.
fn updates_json(root: &Path) -> String {
    match inspect::scan_changes(root, Some(&Timespan::All)) {
        Ok(changes) => serde_json::to_string(&UpdatesResponse {
            files: file_entries(root, &changes),
            changes,
        })
        .expect("UpdatesResponse serializes as JSON"),
        Err(message) => serde_json::to_string(&ErrorResponse { error: message })
            .expect("ErrorResponse serializes as JSON"),
    }
}

/// The ~1s poll (design D4): emit the latest feed and index whenever the
/// store or the root's shape moves. Runs for the server's lifetime; a scan
/// error still updates the watchers so the page can show it.
async fn poll_store(root: PathBuf, mut signal: MovementSignal, events: watch::Sender<String>) {
    loop {
        tokio::time::sleep(POLL_INTERVAL).await;
        let next = MovementSignal::of(&root);
        if next == signal {
            continue;
        }
        signal = next;
        let _ = events.send(updates_json(&root));
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
/// feed plus the all-files index immediately on connect, then a fresh pair
/// each time the poller detects movement (in the store or the root's shape).
/// `EventSource` in the page reconnects on its own if the stream drops.
async fn api_events(
    State(state): State<WebState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let updates = tokio_stream::wrappers::WatchStream::new(state.events.clone());
    let events = updates.map(|feed| Ok(Event::default().event("changes").data(feed)));
    Sse::new(events).keep_alive(KeepAlive::default())
}

/// The shell document for every screen address (design D1): the client reads
/// `location` and renders the matching screen.
async fn shell() -> axum::response::Response {
    // Defense in depth (design D2): assets come from this origin only, and
    // no inline script or event handler can run; inline style is allowed
    // solely because Mermaid writes its SVG's styling that way (see `CSP`).
    // `no-cache` keeps a rebuilt binary from serving a stale shell whose
    // markup predates its assets.
    let mut response = Html(INDEX_HTML).into_response();
    response.headers_mut().insert(
        "Content-Security-Policy",
        axum::http::HeaderValue::from_static(CSP),
    );
    response.headers_mut().insert(
        "Cache-Control",
        axum::http::HeaderValue::from_static("no-cache"),
    );
    response
}

/// `GET /app.css`: the embedded stylesheet, served from the same origin.
async fn app_css() -> axum::response::Response {
    asset_response(APP_CSS, "text/css; charset=utf-8")
}

/// `GET /app.js`: the embedded client script, served from the same origin.
async fn app_js() -> axum::response::Response {
    asset_response(APP_JS, "text/javascript; charset=utf-8")
}

/// `GET /prefs.js`: the embedded preference bootstrap, served from the same
/// origin so the strict CSP keeps working (design D3, D6).
async fn prefs_js() -> axum::response::Response {
    asset_response(PREFS_JS, "text/javascript; charset=utf-8")
}

/// `GET /mermaid.js`: the vendored Mermaid bundle that draws diagrams in the
/// Reader (design D1, D5). Same origin, so `script-src 'self'` covers it; the
/// client asks for it only when a rendered file actually contains a diagram.
async fn mermaid_js() -> axum::response::Response {
    asset_response(MERMAID_JS, "text/javascript; charset=utf-8")
}

/// An embedded asset with its content type, and `no-cache` so a rebuilt
/// binary never serves a stale asset from the browser cache (design D2).
fn asset_response(body: &'static str, content_type: &'static str) -> axum::response::Response {
    let mut response = axum::response::Response::new(axum::body::Body::from(body));
    let headers = response.headers_mut();
    headers.insert(
        "Content-Type",
        axum::http::HeaderValue::from_static(content_type),
    );
    headers.insert(
        "Cache-Control",
        axum::http::HeaderValue::from_static("no-cache"),
    );
    response
}

/// Every other path (design D1, D10): the shell for a screen address the
/// client routes itself (including its not-found screen), and a plain JSON
/// 404 for unknown API paths so a typo never masquerades as a page.
async fn fallback(uri: axum::http::Uri) -> axum::response::Response {
    let path = uri.path();
    if path == "/api" || path.starts_with("/api/") {
        return api_error(StatusCode::NOT_FOUND, format!("No API endpoint at {path}"))
            .into_response();
    }
    shell().await
}

#[derive(Serialize)]
struct ChangesResponse {
    changes: Vec<inspect::ChangeRecord>,
}

#[derive(Serialize)]
struct UpdatesResponse {
    changes: Vec<inspect::ChangeRecord>,
    files: Vec<FileEntry>,
}

#[derive(Serialize)]
struct FilesResponse {
    files: Vec<FileEntry>,
}

/// One memory file in the all-files index (design D4): its path, line count,
/// latest recorded version, and the time of that version's record. `lines`
/// is `null` for an unreadable or non-text file; version and time are `null`
/// when the file has no recorded history.
#[derive(Serialize)]
struct FileEntry {
    path: String,
    lines: Option<usize>,
    latest_version: Option<u64>,
    last_changed: Option<String>,
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
    /// Optional recorded version; absent means the file's current content.
    version: Option<String>,
}

/// Every memory file in the root with its metadata (design D1, D4), sorted by
/// path. Includes files with no records in the version store; a missing root
/// yields an empty list, never an error.
///
/// The newest change per path comes from the same newest-first scan the feed
/// uses. Line counts read each file once; symlinks are counted as unreadable
/// so no content beyond the root is ever read.
fn file_entries(root: &Path, changes: &[inspect::ChangeRecord]) -> Vec<FileEntry> {
    // The scan is newest first, so the first record per path is the latest.
    let mut newest: std::collections::HashMap<&str, (u64, &str)> = std::collections::HashMap::new();
    for change in changes {
        newest
            .entry(change.path.as_str())
            .or_insert((change.seq, change.recorded_at.as_str()));
    }

    // A nonexistent root (fresh machine) is the empty-index case, not an
    // error; everything the walker sees is already inside the canonical root.
    let Ok(canon) = root.canonicalize() else {
        return Vec::new();
    };
    let mut files = Vec::new();
    walk_files(&canon, &canon, &mut files);
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let latest = newest.get(path.as_str());
            FileEntry {
                lines: line_count(&canon.join(&path)),
                latest_version: latest.map(|(seq, _)| *seq),
                last_changed: latest.map(|(_, at)| (*at).to_string()),
                path,
            }
        })
        .collect()
}

/// The line count of a memory file, or `None` when it is not a readable text
/// file. A symlink is `None` too: its target may live outside the root.
fn line_count(path: &Path) -> Option<usize> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return None;
    }
    std::fs::read_to_string(path)
        .ok()
        .map(|content| content.lines().count())
}

/// Recursive read of `dir`, collecting memory files as relative keys (design
/// D2). Skips hidden entries and the version-store directory. Symlinks are
/// never followed — `DirEntry::metadata` is link-only — so a link pointing
/// outside the root is listed by its in-root path alone, and no path or
/// content beyond the root is ever read.
fn walk_files(root: &Path, dir: &Path, files: &mut Vec<String>) {
    let store = crate::versions::store_dir(root);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path == store {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            walk_files(root, &path, files);
        } else {
            let target = crate::paths::Target {
                path,
                is_root: false,
            };
            if let Ok(rel) = crate::ops::rel_key(root, &target) {
                files.push(rel);
            }
        }
    }
}

#[derive(Serialize)]
struct FileResponse {
    /// The path as the page should keep addressing it (`/file?path=…`).
    path: String,
    /// The version rendered: the requested record's seq, or the latest
    /// recorded seq for the current content (`null` without history).
    version: Option<u64>,
    /// The latest recorded version of the file, if it has one.
    latest_version: Option<u64>,
    /// Whether the rendered content is the file's current content and latest
    /// recorded version (design D4).
    is_current: bool,
    /// The line count of the content actually returned: the live file or the
    /// chosen record (design D7).
    lines: usize,
    /// The content rendered from markdown into HTML (design D2).
    html: String,
    /// The file's version timeline, oldest first (design D8).
    history: Vec<HistoryEntry>,
}

/// One entry of the Reader's version strip (design D4, D8): the record plus
/// its `+N -M` line-count summary against the previous version.
#[derive(Serialize)]
struct HistoryEntry {
    seq: u64,
    recorded_at: String,
    cause: String,
    added: usize,
    removed: usize,
}

/// The version timeline of one file, oldest first, with diff counts against
/// the previous record (design D4).
fn history_of(records: &[crate::versions::Record]) -> Vec<HistoryEntry> {
    records
        .iter()
        .enumerate()
        .map(|(index, record)| {
            let previous = if index == 0 {
                ""
            } else {
                records[index - 1].content.as_str()
            };
            let (added, removed) = crate::ops::diff_counts(previous, &record.content);
            HistoryEntry {
                seq: record.seq,
                recorded_at: record.recorded_at.clone(),
                cause: record.cause.clone(),
                added,
                removed,
            }
        })
        .collect()
}

/// `GET /api/files`: the all-files index (design D1, D4) — every memory file
/// in the root with its line count, latest version, and last change, sorted
/// by path, regardless of recorded changes. A nonexistent root returns an
/// empty list, never an error.
async fn api_files(State(state): State<WebState>) -> ApiResult<FilesResponse> {
    let changes = match inspect::scan_changes(&state.root, Some(&Timespan::All)) {
        Ok(changes) => changes,
        Err(message) => return Err(api_error(StatusCode::INTERNAL_SERVER_ERROR, message)),
    };
    Ok((
        StatusCode::OK,
        Json(FilesResponse {
            files: file_entries(&state.root, &changes),
        }),
    ))
}

/// `GET /api/file?path=…[&version=…]`: one memory file rendered as HTML, with
/// its latest recorded version and version timeline (design D4). Without
/// `version` the current content is rendered; with it, that record's content.
/// The path goes through the same `paths::resolve` sandbox as every other
/// surface (design D6), so traversal and symlink escapes are refused, a
/// missing file is a clear 404, and an unknown version a clear 400 naming the
/// available range.
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
    let rel = crate::ops::rel_key(&state.root, &target)
        .map_err(|message| api_error(StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let store = crate::versions::store_dir(&state.root);
    let records = crate::versions::read(&store, &rel)
        .map_err(|message| api_error(StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let latest_version = records.last().map(|record| record.seq);
    let history = history_of(&records);

    let (version, is_current, lines, html) = match query.version.as_deref() {
        None => {
            let content = std::fs::read_to_string(&target.path).map_err(|error| {
                api_error(
                    StatusCode::NOT_FOUND,
                    format!(
                        "The path {} is not a readable text file: {error}",
                        query.path
                    ),
                )
            })?;
            (
                latest_version,
                true,
                content.lines().count(),
                markdown::render(&content),
            )
        }
        Some(raw) => {
            let version: i64 = raw.trim().parse().map_err(|_| {
                api_error(
                    StatusCode::BAD_REQUEST,
                    format!("Error: `{raw}` is not a version number"),
                )
            })?;
            let record = crate::ops::record_at(&store, &state.root, &query.path, &target, version)
                .map_err(|message| api_error(StatusCode::BAD_REQUEST, message))?;
            (
                Some(record.seq),
                Some(record.seq) == latest_version,
                record.content.lines().count(),
                markdown::render(&record.content),
            )
        }
    };
    Ok((
        StatusCode::OK,
        Json(FileResponse {
            path: rel,
            version,
            latest_version,
            is_current,
            lines,
            html,
            history,
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
    fn raw_get(port: u16, path: &str) -> String {
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
        raw
    }

    /// A raw response's status, headers, and body.
    fn split_response(raw: &str) -> (u16, String, String) {
        let (head, body) = raw.split_once("\r\n\r\n").expect("headers and body");
        let status: u16 = head
            .split_whitespace()
            .nth(1)
            .expect("a status line")
            .parse()
            .expect("a numeric status");
        (status, head.to_string(), body.to_string())
    }

    /// One header's value from a raw response, case-insensitively.
    fn header_value(raw: &str, name: &str) -> Option<String> {
        let (_, head, _) = split_response(raw);
        head.lines().skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case(name)
                .then(|| value.trim().to_string())
        })
    }

    /// A blocking HTTP GET returning the status and body.
    fn http_get(port: u16, path: &str) -> (u16, String) {
        let (status, _, body) = split_response(&raw_get(port, path));
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
    async fn every_screen_address_serves_the_shell_with_a_tight_csp() {
        let root = root::root_in(&temp_dir("web-shell"));
        let port = spawn_app(root).await;

        for path in ["/", "/changes", "/files", "/file", "/not-a-screen"] {
            let raw = raw_get(port, path);
            let (status, _, body) = split_response(&raw);
            assert_eq!(status, 200, "{path}: {body}");
            assert!(
                body.contains("<!DOCTYPE html"),
                "{path} returns the document: {body}"
            );
            assert!(
                body.contains("/app.css") && body.contains("/app.js") && body.contains("/prefs.js"),
                "{path} references the split assets: {body}"
            );
            // The settings control and its dialog are part of the shared shell
            // on every screen, and the dialog is closed in the served markup so
            // it stays out of the way without JavaScript (design D1, D2).
            assert!(
                body.contains(r#"aria-label="Settings""#)
                    && body.contains(r#"aria-haspopup="dialog""#),
                "{path} carries the settings control: {body}"
            );
            let dialog = body.find("<dialog class=\"settings\"").unwrap_or_else(|| {
                panic!("{path} serves the settings dialog: {body}");
            });
            assert!(
                body.contains(r#"id="settings-title""#)
                    && body.contains("Appearance")
                    && body.contains("Page width")
                    && body.contains("data-settings-close"),
                "{path} fills the settings dialog: {body}"
            );
            // Served closed: the opening tag carries no `open` attribute, so
            // the dialog stays out of the way without JavaScript.
            let tag_end = dialog + body[dialog..].find('>').expect("a dialog start tag");
            let start_tag = &body[dialog..tag_end];
            assert!(
                !start_tag.contains(" open"),
                "{path} serves the dialog closed: {start_tag}"
            );
            // Both settings live inside the dialog, not in the header, and the
            // appearance choice is a labelled three-segment control named by
            // its legend - no separate status readout (design D3).
            assert!(
                body.contains(r#"<legend class="settings-label">Appearance</legend>"#)
                    && body.contains("<span>Light</span>")
                    && body.contains("<span>Dark</span>")
                    && body.contains("<span>System</span>")
                    && !body.contains("THEME:")
                    && !body.contains(r#"id="theme-label""#),
                "{path} carries the labelled appearance control: {body}"
            );
            let settings_group = body
                .find("settings-label\">Appearance")
                .expect("an appearance group");
            let close = body.find("data-settings-close").expect("a close control");
            assert!(
                dialog < settings_group && dialog < close,
                "{path} keeps the appearance control inside the dialog: {body}"
            );
            assert!(
                !body[..dialog].contains("data-theme-choice"),
                "{path} keeps the appearance controls out of the header: {body}"
            );
            // The width control is a range inside the dialog with a name and a
            // spoken value, beside a visible readout and its end labels
            // (design D5).
            assert!(
                body.contains(r#"type="range""#)
                    && body.contains(r#"aria-label="Page width""#)
                    && body.contains(r#"aria-valuetext="55% of the window""#)
                    && body.contains(r#"id="width-value""#)
                    && body.contains(">40%<")
                    && body.contains(">100%<"),
                "{path} carries the page-width control: {body}"
            );
            let width_control = body.find(r#"id="page-width""#).expect("the width control");
            assert!(
                dialog < width_control,
                "{path} keeps the width control inside the dialog: {body}"
            );
            let csp = header_value(&raw, "content-security-policy").expect("a CSP header");
            // One documented relaxation, for styles only, because Mermaid
            // writes its SVG's styling inline (design D2); script stays
            // same-origin and inline-free on every screen.
            assert_eq!(
                csp,
                "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; base-uri 'none'; form-action 'none'",
                "{path} carries the tightened policy"
            );
            let directives: std::collections::HashMap<&str, &str> = csp
                .split(';')
                .filter_map(|directive| directive.trim().split_once(' '))
                .collect();
            assert_eq!(
                directives.get("script-src"),
                Some(&"'self'"),
                "{path} allows script only from this origin: {csp}"
            );
            assert_eq!(
                directives.get("connect-src"),
                Some(&"'self'"),
                "{path} connects only to this origin: {csp}"
            );
            assert_eq!(
                directives.get("default-src"),
                Some(&"'none'"),
                "{path} allows nothing by default: {csp}"
            );
            assert!(
                !csp.contains("unsafe-eval") && !csp.contains("script-src 'self' 'unsafe-inline'"),
                "{path} must not execute inline or evaluated script: {csp}"
            );
            assert_eq!(
                header_value(&raw, "cache-control").as_deref(),
                Some("no-cache"),
                "{path} must not be cached across rebuilds"
            );
        }

        // An unknown API path stays a JSON 404, never the shell.
        let (status, body) = http_get(port, "/api/nope");
        assert_eq!(status, 404, "{body}");
        assert!(body.contains(r#""error""#), "an error object: {body}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn assets_are_served_with_their_content_types_and_no_cache() {
        let root = root::root_in(&temp_dir("web-assets"));
        let port = spawn_app(root).await;

        for (path, content_type) in [
            ("/app.css", "text/css"),
            ("/app.js", "text/javascript"),
            ("/prefs.js", "text/javascript"),
            ("/mermaid.js", "text/javascript"),
        ] {
            let raw = raw_get(port, path);
            let (status, _, body) = split_response(&raw);
            assert_eq!(status, 200, "{path}: {body}");
            let kind = header_value(&raw, "content-type").expect("a content type");
            assert!(
                kind.starts_with(content_type),
                "{path} is served as {content_type}, got {kind}"
            );
            assert_eq!(
                header_value(&raw, "cache-control").as_deref(),
                Some("no-cache"),
                "{path} must not be cached across rebuilds"
            );
            assert!(!body.is_empty(), "{path} has content");
        }

        // The diagram renderer is the vendored bundle, not a stub: it is
        // large and exposes the global the client uses (design D1).
        let (status, bundle) = http_get(port, "/mermaid.js");
        assert_eq!(status, 200, "{}", &bundle[..bundle.len().min(80)]);
        assert!(
            bundle.len() > 1_000_000,
            "the embedded Mermaid bundle, got {} bytes",
            bundle.len()
        );
        assert!(
            bundle.contains(r#"globalThis["mermaid"]"#),
            "the bundle exposes the mermaid global"
        );

        // The stylesheet carries both palettes: the system preference, and the
        // explicit choices the header control sets on the root element
        // (design D2).
        let (status, css) = http_get(port, "/app.css");
        assert_eq!(status, 200, "{css}");
        for marker in [
            "prefers-color-scheme: dark",
            ":root:not([data-theme])",
            ":root[data-theme=\"dark\"]",
            ":root[data-theme=\"light\"]",
            "color-scheme: dark",
            "color-scheme: light",
        ] {
            assert!(css.contains(marker), "app.css carries `{marker}`: {css}");
        }
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
        assert_eq!(json["version"], serde_json::Value::Null, "no records yet");
        assert_eq!(json["is_current"], true, "the live content is current");
        assert_eq!(json["lines"], 7, "the rendered content's line count");
        assert_eq!(
            json["history"],
            serde_json::json!([]),
            "no version strip without records"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn api_file_renders_a_past_version_with_its_history() {
        let home = temp_dir("web-file-history");
        let root = root::root_in(&home);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), "a2\n").unwrap();
        seed_store(
            &root,
            &[
                ("a.txt", 1, "a1\n", "create", "2027-01-01T10:00:00Z"),
                ("a.txt", 2, "a1\na2\n", "insert", "2027-01-05T12:30:00Z"),
                ("a.txt", 3, "a2\n", "str_replace", "2027-01-06T08:00:00Z"),
            ],
        );
        let port = spawn_app(root).await;

        // The current view renders the live content at the latest version.
        let (status, body) = http_get(port, "/api/file?path=a.txt");
        assert_eq!(status, 200, "{body}");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["version"], 3, "{json}");
        assert_eq!(json["latest_version"], 3, "{json}");
        assert_eq!(json["is_current"], true, "{json}");
        assert_eq!(json["lines"], 1, "the live content's line count: {json}");
        let html = json["html"].as_str().unwrap();
        assert!(html.contains("a2"), "the live content: {html}");
        assert!(!html.contains("a1"), "not the older line: {html}");

        // The version timeline is oldest first, with diff counts.
        let history = json["history"].as_array().expect("a history array");
        let seqs: Vec<u64> = history
            .iter()
            .map(|entry| entry["seq"].as_u64().unwrap())
            .collect();
        assert_eq!(seqs, [1, 2, 3], "oldest first: {json}");
        assert_eq!(history[0]["cause"], "create", "{json}");
        assert_eq!(history[0]["recorded_at"], "2027-01-01T10:00:00Z", "{json}");
        let counts: Vec<(u64, u64)> = history
            .iter()
            .map(|entry| {
                (
                    entry["added"].as_u64().unwrap(),
                    entry["removed"].as_u64().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            counts,
            [(1, 0), (1, 0), (0, 1)],
            "each entry counts against the previous version: {json}"
        );

        // A past version renders its own recorded content.
        let (status, body) = http_get(port, "/api/file?path=a.txt&version=1");
        assert_eq!(status, 200, "{body}");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["version"], 1, "{json}");
        assert_eq!(json["latest_version"], 3, "{json}");
        assert_eq!(json["is_current"], false, "{json}");
        assert_eq!(json["lines"], 1, "version 1 is one line: {json}");
        let html = json["html"].as_str().unwrap();
        assert!(html.contains("a1"), "version 1's content: {html}");
        assert!(!html.contains("a2"), "nothing from later versions: {html}");

        // A multi-line past version reports its own line count, not the live
        // file's.
        let (_, body) = http_get(port, "/api/file?path=a.txt&version=2");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["lines"], 2, "version 2 is two lines: {json}");

        // The latest recorded version is still current when requested.
        let (_, body) = http_get(port, "/api/file?path=a.txt&version=3");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["is_current"], true, "{json}");

        // An unknown version names the available range.
        let (status, body) = http_get(port, "/api/file?path=a.txt&version=9");
        assert_eq!(status, 400, "{body}");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let message = json["error"].as_str().unwrap();
        assert!(message.contains("no version 9"), "{message}");
        assert!(message.contains("1..=3"), "names the range: {message}");
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
        // A file outside the root, reached by traversal everywhere and, on
        // Unix only, through a symlink inside it (design D8).
        std::fs::write(home.join("secrets.txt"), "classified\n").unwrap();
        let mut escaping = vec!["/memories/../secrets.txt", "../secrets.txt"];
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(home.join("secrets.txt"), root.join("escape.md")).unwrap();
            escaping.push("escape.md");
        }
        let port = spawn_app(root).await;

        for path in escaping {
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

    #[tokio::test(flavor = "multi_thread")]
    async fn api_files_reports_metadata_for_recorded_and_unrecorded_files() {
        let home = temp_dir("web-files");
        let root = root::root_in(&home);
        std::fs::create_dir_all(&root).unwrap();
        // Files the version store knows nothing about, nested and top-level.
        std::fs::create_dir_all(root.join("projects/alpha")).unwrap();
        std::fs::write(root.join("scratch.md"), "scratch\n").unwrap();
        std::fs::write(root.join("projects/alpha/notes.txt"), "one\ntwo\nthree\n").unwrap();
        // Hidden entries and the store itself are not memory files.
        std::fs::write(root.join(".hidden"), "h\n").unwrap();
        // A recorded file, present on disk with the latest content.
        std::fs::write(root.join("notes.md"), "one\ntwo\n").unwrap();
        seed_store(
            &root,
            &[
                ("notes.md", 1, "one\n", "create", "2027-01-01T10:00:00Z"),
                (
                    "notes.md",
                    2,
                    "one\ntwo\n",
                    "insert",
                    "2027-01-05T12:30:00Z",
                ),
            ],
        );
        // A non-text file: its line count is unknown, not an error.
        std::fs::write(root.join("binary.dat"), [0xff, 0xfe, 0x00]).unwrap();
        let port = spawn_app(root).await;

        let (status, body) = http_get(port, "/api/files");
        assert_eq!(status, 200, "{body}");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let files = json["files"].as_array().expect("a files array");
        let paths: Vec<&str> = files
            .iter()
            .map(|f| f["path"].as_str().expect("a path string"))
            .collect();
        assert_eq!(
            paths,
            [
                "binary.dat",
                "notes.md",
                "projects/alpha/notes.txt",
                "scratch.md"
            ],
            "every memory file, sorted, nothing hidden: {json}"
        );

        // The recorded file joins the scan: lines, latest version, last change.
        let recorded = &files[1];
        assert_eq!(recorded["lines"], 2, "{json}");
        assert_eq!(recorded["latest_version"], 2, "{json}");
        assert_eq!(recorded["last_changed"], "2027-01-05T12:30:00Z", "{json}");

        // Nested and unrecorded files still carry their line count.
        assert_eq!(files[2]["lines"], 3, "{json}");
        assert_eq!(
            files[2]["latest_version"],
            serde_json::Value::Null,
            "{json}"
        );
        assert_eq!(files[2]["last_changed"], serde_json::Value::Null, "{json}");
        assert_eq!(files[3]["lines"], 1, "{json}");

        // Unreadable non-text files have no line count.
        assert_eq!(files[0]["lines"], serde_json::Value::Null, "{json}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn api_files_reads_versions_from_a_legacy_windows_store() {
        let home = temp_dir("web-files-legacy");
        let root = root::root_in(&home);
        std::fs::create_dir_all(root.join("projects/alpha")).unwrap();
        std::fs::write(root.join("projects/alpha/notes.txt"), "one\ntwo\n").unwrap();
        // The store an earlier Windows build wrote for that same file.
        seed_store(
            &root,
            &[
                (
                    "projects\\alpha\\notes.txt",
                    1,
                    "one\n",
                    "create",
                    "2027-01-01T10:00:00Z",
                ),
                (
                    "projects\\alpha\\notes.txt",
                    2,
                    "one\ntwo\n",
                    "insert",
                    "2027-01-05T12:30:00Z",
                ),
            ],
        );
        let port = spawn_app(root).await;

        let (status, body) = http_get(port, "/api/files");
        assert_eq!(status, 200, "{body}");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let files = json["files"].as_array().expect("a files array");
        assert_eq!(files.len(), 1, "{json}");
        assert_eq!(files[0]["path"], "projects/alpha/notes.txt", "{json}");
        assert_eq!(files[0]["latest_version"], 2, "{json}");
        assert_eq!(files[0]["last_changed"], "2027-01-05T12:30:00Z", "{json}");
        assert_eq!(files[0]["lines"], 2, "{json}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn api_files_on_a_missing_root_is_an_empty_list() {
        let root = root::root_in(&temp_dir("web-files-missing"));
        assert!(!root.exists(), "the viewer must not need the root to exist");
        let port = spawn_app(root).await;

        let (status, body) = http_get(port, "/api/files");
        assert_eq!(status, 200, "{body}");
        assert_eq!(body, r#"{"files":[]}"#, "an empty index, not an error");
    }

    // Symlinks are a Unix feature here, so the whole test is Unix-only
    // (design D8); the traversal half is covered on every platform above.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn api_files_never_leaks_content_or_paths_outside_the_root() {
        let home = temp_dir("web-files-escape");
        let root = root::root_in(&home);
        std::fs::create_dir_all(&root).unwrap();
        // Files outside the root, reached through in-root symlinks.
        let outside = home.join("secrets.txt");
        std::fs::write(&outside, "classified\n").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("escape.md")).unwrap();
        std::fs::create_dir_all(home.join("elsewhere")).unwrap();
        std::fs::write(home.join("elsewhere/deep.md"), "buried\n").unwrap();
        std::os::unix::fs::symlink(home.join("elsewhere"), root.join("links")).unwrap();
        let port = spawn_app(root).await;

        // The per-file view refuses the escaping link as before.
        let (status, body) = http_get(port, "/api/file?path=escape.md");
        assert_ne!(status, 200, "the symlink escape must stay refused: {body}");

        // The listing may name the in-root link paths, but nothing beyond.
        let (_, body) = http_get(port, "/api/files");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let files: Vec<&str> = json["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["path"].as_str().unwrap())
            .collect();
        assert_eq!(
            files,
            ["escape.md", "links"],
            "in-root link paths only: {json}"
        );
        // A symlink's line count is unknown rather than read through.
        for file in json["files"].as_array().unwrap() {
            assert_eq!(file["lines"], serde_json::Value::Null, "{json}");
        }
        assert!(!body.contains("secrets"), "no outside file names: {body}");
        assert!(
            !body.contains("elsewhere"),
            "no outside directory names: {body}"
        );
        assert!(
            !body.contains("deep.md"),
            "no paths from beyond the root: {body}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn api_events_pushes_the_file_list_when_the_root_moves_without_the_store() {
        let home = temp_dir("web-events-files");
        let root = root::root_in(&home);
        root::ensure(&root).unwrap();
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

        // The current feed and index arrive immediately on connect.
        read_until_contains(
            &mut stream,
            "MEMORY.md",
            Duration::from_secs(10),
            "initial update",
        );

        // A file created without any store record (a manual touch) must
        // still reach the page: the root's shape moves the poller.
        std::fs::write(root.join("unrecorded.md"), "scratch\n").unwrap();
        read_until_contains(
            &mut stream,
            "unrecorded.md",
            Duration::from_secs(10),
            "root movement",
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
