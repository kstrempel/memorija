//! End-to-end test: drive the compiled `memorija` binary over stdio as an MCP
//! client would — initialize, list tools, then run a create → view →
//! str_replace → insert → rename → delete sequence, including one failure the
//! client retries after reading the error message.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

struct Client {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl Client {
    fn spawn(home: &Path) -> Client {
        let mut child = Command::new(env!("CARGO_BIN_EXE_memorija"))
            .env("HOME", home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn memorija");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Client {
            child,
            stdin,
            stdout,
            next_id: 1,
        }
    }

    /// Send a request and wait for its response (skipping any interleaved
    /// notifications), mimicking a synchronous MCP client.
    fn call(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        writeln!(self.stdin, "{request}").expect("write request");
        self.stdin.flush().expect("flush request");

        loop {
            let mut line = String::new();
            let read = self
                .stdout
                .read_line(&mut line)
                .expect("read response line");
            assert!(read > 0, "server closed stdout before responding to {method}");
            let message: Value = serde_json::from_str(&line).expect("parse JSON-RPC message");
            if message.get("id").and_then(Value::as_i64) == Some(id) {
                return message;
            }
        }
    }

    /// Call the `memory` tool, returning (is_error, text).
    fn memory(&mut self, arguments: Value) -> (bool, String) {
        let response = self.call(
            "tools/call",
            json!({ "name": "memory", "arguments": arguments }),
        );
        let result = response
            .get("result")
            .unwrap_or_else(|| panic!("expected result, got: {response}"));
        let is_error = result
            .get("isError")
            .and_then(Value::as_bool)
            .expect("isError flag");
        let text = result["content"][0]["text"]
            .as_str()
            .expect("text content")
            .to_string();
        (is_error, text)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn temp_home(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "memorija-e2e-{}-{}-{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn full_session_over_stdio() {
    let home = temp_home("session");
    let mut client = Client::spawn(&home);

    // 1. Handshake.
    let response = client.call(
        "initialize",
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "e2e-test", "version": "0.0.1" }
        }),
    );
    assert!(
        response["result"]["serverInfo"].is_object(),
        "initialize must answer: {response}"
    );
    writeln!(client.stdin, r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#).unwrap();
    client.stdin.flush().unwrap();

    // 2. Exactly one tool, `memory`, with the pinned input schema.
    let response = client.call("tools/list", json!({}));
    let tools = response["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1, "exactly one tool: {response}");
    assert_eq!(tools[0]["name"], "memory");
    let description = tools[0]["description"].as_str().unwrap();
    assert!(description.to_lowercase().contains("persistent memory"), "{description}");
    // Versioning guidance (spec: tool description documents versioning).
    assert!(description.contains("history"), "{description}");
    assert!(description.contains("restore"), "{description}");
    assert!(description.contains("version"), "{description}");
    assert_eq!(tools[0]["inputSchema"]["required"], json!(["command", "path"]));
    let command_enum: Vec<&str> = tools[0]["inputSchema"]["properties"]["command"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(
        command_enum,
        ["view", "create", "str_replace", "insert", "delete", "rename", "history", "restore"]
    );
    assert!(tools[0]["inputSchema"]["properties"]["version"].is_object());

    // 3. Create a memory file in a nested directory.
    let (error, text) = client.memory(json!({
        "command": "create",
        "path": "/memories/demo/hello.txt",
        "file_text": "first line\nsecond line\nthird line\n"
    }));
    assert!(!error, "{text}");
    assert!(text.contains("created"), "{text}");
    assert_eq!(
        std::fs::read_to_string(home.join(".memorija/memories/demo/hello.txt")).unwrap(),
        "first line\nsecond line\nthird line\n"
    );

    // 4. View with line numbers.
    let (error, text) = client.memory(json!({
        "command": "view",
        "path": "/memories/demo/hello.txt"
    }));
    assert!(!error, "{text}");
    assert!(text.contains("1\tfirst line"), "{text}");

    // 5. str_replace with a wrong old_str: the model reads the error and
    //    retries with corrected arguments.
    let (error, text) = client.memory(json!({
        "command": "str_replace",
        "path": "/memories/demo/hello.txt",
        "old_str": "typo that is not there",
        "new_str": "x"
    }));
    assert!(error, "missing old_str must be an error result: {text}");
    let (error, text) = client.memory(json!({
        "command": "str_replace",
        "path": "/memories/demo/hello.txt",
        "old_str": "second line",
        "new_str": "second line, edited"
    }));
    assert!(!error, "{text}");
    assert!(
        std::fs::read_to_string(home.join(".memorija/memories/demo/hello.txt"))
            .unwrap()
            .contains("second line, edited")
    );

    // 6. Insert at the top.
    let (error, text) = client.memory(json!({
        "command": "insert",
        "path": "/memories/demo/hello.txt",
        "insert_line": 0,
        "insert_text": "top line\n"
    }));
    assert!(!error, "{text}");
    assert!(std::fs::read_to_string(home.join(".memorija/memories/demo/hello.txt"))
        .unwrap()
        .starts_with("top line\n"));

    // 6b. Versioning: the three mutations are recorded; history shows them;
    //     view at a version reads the past; restore reverts and records.
    let (error, text) = client.memory(json!({
        "command": "history",
        "path": "/memories/demo/hello.txt"
    }));
    assert!(!error, "{text}");
    assert!(text.contains("v1") && text.contains("v3"), "{text}");

    let (error, text) = client.memory(json!({
        "command": "view",
        "path": "/memories/demo/hello.txt",
        "version": 2
    }));
    assert!(!error, "{text}");
    assert!(text.contains("first line"), "{text}");
    assert!(text.contains("second line, edited"), "{text}");
    assert!(!text.contains("top line"), "{text}");

    let (error, text) = client.memory(json!({
        "command": "restore",
        "path": "/memories/demo/hello.txt",
        "version": 2
    }));
    assert!(!error, "{text}");
    let content = std::fs::read_to_string(home.join(".memorija/memories/demo/hello.txt")).unwrap();
    assert!(content.contains("second line, edited"), "{content}");
    assert!(!content.starts_with("top line\n"), "{content}");

    let (error, text) = client.memory(json!({
        "command": "history",
        "path": "/memories/demo/hello.txt"
    }));
    assert!(!error, "{text}");
    assert!(text.contains("v4") && text.contains("restore"), "{text}");

    // The version store lives beside the memory root, out of the tree.
    assert!(home.join(".memorija/versions").is_dir());

    // 7. Rename (with a new parent directory).
    let (error, text) = client.memory(json!({
        "command": "rename",
        "path": "/memories/demo/hello.txt",
        "new_path": "/memories/demo/archive/hello-renamed.txt"
    }));
    assert!(!error, "{text}");
    assert!(!home.join(".memorija/memories/demo/hello.txt").exists());
    assert!(home.join(".memorija/memories/demo/archive/hello-renamed.txt").exists());

    // 8. Delete the renamed file, then the demo directory.
    let (error, text) = client.memory(json!({
        "command": "delete",
        "path": "/memories/demo/archive/hello-renamed.txt"
    }));
    assert!(!error, "{text}");
    assert!(!home.join(".memorija/memories/demo/archive/hello-renamed.txt").exists());

    let (error, text) = client.memory(json!({
        "command": "delete",
        "path": "/memories/demo"
    }));
    assert!(!error, "{text}");
    assert!(!home.join(".memorija/memories/demo").exists());
}
