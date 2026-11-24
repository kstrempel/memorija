//! End-to-end test over Streamable HTTP (spec: `mcp-http-transport`): spawn
//! the compiled binary with `serve --http` on an ephemeral port, connect an
//! rmcp HTTP client, and verify the tool surface matches what stdio serves.

use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransport;
use serde_json::{Value, json};

/// The `memory` tool schema as served over stdio (shared handler, schemars
/// output; identical over both transports).
const EXPECTED_SCHEMA: &str = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "command": {
      "description": "The eight memory commands, serialized as Anthropic's string enum.",
      "enum": ["view", "create", "str_replace", "insert", "delete", "rename", "history", "restore"],
      "type": "string"
    },
    "file_text": {"type": ["string", "null"]},
    "insert_line": {"format": "int64", "type": ["integer", "null"]},
    "insert_text": {"type": ["string", "null"]},
    "new_path": {"type": ["string", "null"]},
    "new_str": {"type": ["string", "null"]},
    "old_str": {"type": ["string", "null"]},
    "path": {"type": "string"},
    "version": {"description": "A past version number, honored by `view` and required by `restore`.", "format": "int64", "type": ["integer", "null"]},
    "view_range": {
      "items": {"format": "int64", "type": "integer"},
      "maxItems": 2,
      "minItems": 2,
      "type": ["array", "null"]
    }
  },
  "required": ["command", "path"],
  "type": "object"
}"#;

fn temp_home(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "memorija-http-test-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn spawn_http_server(home: &Path, port: u16) -> Child {
    Command::new(env!("CARGO_BIN_EXE_memorija"))
        .args([
            "serve",
            "--http",
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ])
        .env("HOME", home)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn memorija serve --http")
}

/// Reserve an ephemeral port and release it for the server to bind.
fn ephemeral_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn wait_for_port(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("http server never started listening on port {port}");
}

#[tokio::test]
async fn http_client_sees_stdio_identical_memory_tool() {
    let home = temp_home("e2e");
    let port = ephemeral_port();
    let mut server = spawn_http_server(&home, port);
    wait_for_port(port);

    let url = format!("http://127.0.0.1:{port}/mcp");
    let client =
        ().serve(StreamableHttpClientTransport::from_uri(url))
            .await
            .expect("client initialize over HTTP");

    // Tool listing: exactly `memory`, with the stdio-identical schema.
    let listed = client
        .list_tools(Default::default())
        .await
        .expect("list tools");
    assert_eq!(listed.tools.len(), 1, "exactly one tool over HTTP");
    let tool = &listed.tools[0];
    assert_eq!(tool.name, "memory");
    let schema = serde_json::to_value(&*tool.input_schema).unwrap();
    let expected: Value = serde_json::from_str(EXPECTED_SCHEMA).unwrap();
    assert_eq!(
        schema, expected,
        "schema must match the stdio-published one"
    );
    assert!(
        tool.description
            .as_deref()
            .unwrap_or("")
            .contains("Persistent memory files"),
        "description should be the shared `memory` tool description"
    );

    // A `view` call returns what stdio would for the same fresh-root state:
    // the root listing, which contains only the seeded MEMORY.md.
    let result = client
        .call_tool(
            CallToolRequestParams::new("memory").with_arguments(
                json!({"command": "view", "path": "/"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .expect("view call over HTTP");
    assert!(
        !result.is_error.unwrap_or(false),
        "view of a fresh root should succeed"
    );
    let text = result
        .content
        .iter()
        .filter_map(|block| match block {
            rmcp::model::ContentBlock::Text(text) => Some(text.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        text.contains("MEMORY.md"),
        "root listing names the seed: {text}"
    );

    // First HTTP launch behaves like first stdio launch: root created + seeded.
    let seed = std::fs::read_to_string(home.join(".memorija/memories/MEMORY.md"))
        .expect("seed file written on first HTTP launch");
    assert!(seed.contains("persistent memory"));

    client.cancel().await.ok();
    server.kill().unwrap();
    server.wait().unwrap();
    let _ = std::fs::remove_dir_all(&home);
}
