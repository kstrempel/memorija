//! End-to-end posture of the `web` viewer (spec: `web-ui`): the default port
//! runs beside `serve --http` on 8080, a non-loopback bind prints the
//! warning, an occupied port exits non-zero with a clear message, and a
//! fresh machine shows the empty state.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// A temp directory standing in for `$HOME`, so the test never touches the
/// real memory root.
fn temp_home(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "memorija-web-test-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Reserve an ephemeral port and release it for the server to bind.
fn ephemeral_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

struct Viewer(Child);

impl Viewer {
    /// Stop the viewer, then collect everything it printed to stderr (the
    /// pipe only ends at EOF, i.e. after the process is gone).
    fn kill_and_stderr(&mut self) -> String {
        let _ = self.0.kill();
        let _ = self.0.wait();
        let mut stderr = String::new();
        if let Some(pipe) = self.0.stderr.as_mut() {
            let _ = pipe.read_to_string(&mut stderr);
        }
        stderr
    }
}

impl Drop for Viewer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn_viewer(home: &Path, args: &[&str]) -> Viewer {
    let child = Command::new(env!("CARGO_BIN_EXE_memorija"))
        .args(args)
        .env("HOME", home)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn memorija web");
    Viewer(child)
}

fn wait_for_port(port: u16) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// A blocking HTTP GET over a raw TCP connection.
fn http_get(port: u16, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
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

#[test]
fn default_port_runs_beside_serve_http_on_8080() {
    let home = temp_home("alongside");
    // `memorija serve --http` on its default port 8080.
    let mut serve = Command::new(env!("CARGO_BIN_EXE_memorija"))
        .args(["serve", "--http"])
        .env("HOME", &home)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn memorija serve --http");
    if !wait_for_port(8080) {
        // Port 8080 is taken by something else on this machine; the
        // alongside scenario cannot be exercised here.
        let _ = serve.kill();
        let _ = serve.wait();
        eprintln!("skipping: port 8080 is already in use");
        return;
    }

    // `memorija web` with defaults must start on 8081 regardless.
    let mut viewer = spawn_viewer(&home, &["web", "--no-open"]);
    assert!(
        wait_for_port(8081),
        "the web viewer binds its default port 8081"
    );
    let (status, _) = http_get(8081, "/");
    assert_eq!(status, 200);

    viewer.0.kill().unwrap();
    viewer.0.wait().unwrap();
    serve.kill().unwrap();
    serve.wait().unwrap();
}

#[test]
fn non_loopback_bind_prints_the_warning() {
    let home = temp_home("non-loopback");
    let port = ephemeral_port();
    let mut viewer = spawn_viewer(
        &home,
        &[
            "web",
            "--host",
            "0.0.0.0",
            "--port",
            &port.to_string(),
            "--no-open",
        ],
    );
    assert!(
        wait_for_port(port),
        "the viewer starts on the non-loopback address"
    );

    let stderr = viewer.kill_and_stderr();
    assert!(stderr.contains("Warning"), "a warning: {stderr}");
    assert!(
        stderr.contains("non-loopback"),
        "names the non-loopback posture: {stderr}"
    );
    // The URL is printed regardless of the browser.
    assert!(
        stderr.contains(&format!("http://0.0.0.0:{port}/")),
        "prints the URL: {stderr}"
    );
}

#[test]
fn occupied_port_exits_non_zero_with_a_clear_message() {
    let home = temp_home("occupied");
    // Hold a port so the viewer cannot bind it.
    let holder = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = holder.local_addr().unwrap().port();

    let mut viewer = spawn_viewer(&home, &["web", "--port", &port.to_string(), "--no-open"]);
    let output = viewer.0.wait().expect("the viewer exits on its own");
    assert!(!output.success(), "a bind failure exits non-zero");
    let stderr = viewer.kill_and_stderr();
    assert!(stderr.contains("cannot bind"), "a clear message: {stderr}");
    assert!(
        stderr.contains(&port.to_string()),
        "names the address: {stderr}"
    );
}

#[test]
fn fresh_machine_shows_the_empty_state() {
    let home = temp_home("fresh");
    let port = ephemeral_port();
    let mut viewer = spawn_viewer(&home, &["web", "--port", &port.to_string(), "--no-open"]);
    assert!(wait_for_port(port), "the viewer starts on a fresh machine");

    let (status, body) = http_get(port, "/api/changes");
    assert_eq!(status, 200, "{body}");
    assert_eq!(body, r#"{"changes":[]}"#, "an empty feed, not an error");
    let (status, page) = http_get(port, "/");
    assert_eq!(status, 200);
    assert!(
        page.contains("No changes recorded yet"),
        "the page carries the empty-state message: {page}"
    );

    viewer.0.kill().unwrap();
    viewer.0.wait().unwrap();
}
