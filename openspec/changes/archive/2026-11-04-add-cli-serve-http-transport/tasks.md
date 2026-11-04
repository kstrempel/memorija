# Tasks

## 1. CLI argument parsing

- [x] 1.1 Add `clap` (derive feature set per design D1) to `Cargo.toml` and verify `cargo build` succeeds
- [x] 1.2 Add the CLI module (`src/cli.rs`) with the `Parser`/`Subcommand`/`Args` structures from design D1 — `serve` subcommand, `--http`, `--stdio` (parsed, conflicts with `--http`), `--port` (requires `--http`) — keeping clap's auto-registered `help` subcommand, with unit tests asserting: bare invocation maps to serve/stdio; `--http` maps to HTTP on default port 8080; `--http --port 9000` maps to HTTP/9000; `--stdio --http`, `--port` without `--http`, and `--port 70000` are usage errors; `help` parses as the help subcommand and `help serve` resolves to serve's help while `help flush` errors; verify with `cargo test`
- [x] 1.3 Rewire `src/main.rs` to parse the CLI first and dispatch on the subcommand, keeping bare `memorija` equivalent to `serve --stdio`, and verify `cargo run -- --help` and `cargo run -- help` print serve usage (exit code zero), `cargo run -- help serve` prints the serve options, and `cargo run -- flush` and `cargo run -- help flush` exit non-zero with a usage error

## 2. Server startup refactor

- [x] 2.1 Extract a `run_server(transport)` startup path (design D2) that resolves + ensures the memory root then serves stdio exactly as today, with unit tests reusing the existing root tests; verify `cargo test` passes and `memorija serve --stdio` still appears as the `memory` tool to an MCP client (e.g. this session's server connection)
- [x] 2.2 Verify usage errors create nothing on disk: run the binary with invalid arguments on a machine without `~/.memorija` and confirm the directory still does not exist afterward

## 3. Streamable HTTP transport

- [x] 3.1 Enable `rmcp`'s `transport-streamable-http-server` feature and verify `cargo build` succeeds with the new dependency tree
- [x] 3.2 Add the HTTP serving path to `run_server` per design D3 (`StreamableHttpService` with `Memorija::new(root)` per connection, minimal axum router, bind `127.0.0.1:port`) and verify `memorija serve --http` starts and reports a clear non-zero failure when the port is taken
- [x] 3.3 Add an integration test that connects an rmcp HTTP client to a spawned server instance on an ephemeral port and verifies: tool listing returns exactly `memory` with the stdio-identical schema, and a `view` call returns the same result stdio would; verify with `cargo test`

## 4. Integration verification

- [x] 4.1 Full pass: `cargo build && cargo test && cargo clippy`, then manually exercise `memorija serve --stdio` from an MCP client config and `memorija serve --http --port <free port>` with a client connect, tool call, and shutdown; confirm memory-root behavior on first HTTP launch matches first stdio launch (created + seeded)

## 5. Host option

- [x] 5.1 Add a `--host <address>` option to `serve` (requires `--http`, default `127.0.0.1`) that sets the bind address of the HTTP server, with unit tests for mapping, `--host` without `--http`, `--host` with `--stdio`, and an invalid address; a stderr warning on non-loopback binds; and an end-to-end check passing `--host 127.0.0.1` through the spawned binary; update the design and spec deltas accordingly
