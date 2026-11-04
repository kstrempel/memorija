# Proposal

## Why

The server currently has no command-line interface: `memorija` always starts an stdio MCP server, and the transport is hardcoded. As more commands are planned for the future, the binary needs a proper command-line surface now — starting with `serve`, the main command that will be served by the CLI setup. The same change is the natural moment to add a second MCP transport: an HTTP server, so MCP clients can connect over the network instead of only spawning the process over stdio.

## What Changes

- Add a `serve` subcommand — `memorija serve` — as the first command-line command, structured so further commands can be added later.
- Add a `help` subcommand — `memorija help` prints usage for the binary and `memorija help <command>` prints usage for a specific command.
- `memorija serve --stdio` starts the MCP server over stdio (the current behavior, made explicit). This is the default when neither transport flag is given.
- `memorija serve --http` starts an MCP server over Streamable HTTP instead, listening on a local address.
- Add a `--port` option that applies to `--http` (default 8080) so the HTTP endpoint is configurable without another flag later.
- Add a `--host <address>` option that applies to `--http` (default `127.0.0.1`) so the HTTP server can be exposed to other interfaces deliberately, with a startup warning on non-loopback binds.
- The existing `memory` tool, memory root, and all tool semantics are unchanged; only how clients reach the server changes.
- Bare `memorija` (no subcommand) keeps working as today by behaving like `memorija serve --stdio`, so existing MCP client configurations that launch the binary directly are not broken.

Assumptions recorded during planning:

- The subcommand is `serve` (per the request's opening line); `memorija server …` in the same request is read as a typo of `memorija serve …`.
- The default HTTP bind address is `127.0.0.1` (localhost only) — an explicit flag for the address can come later if needed.
- Passing both `--stdio` and `--http` is a usage error, reported with a clear message and non-zero exit.

## Capabilities

### New Capabilities

- `cli`: The command-line interface of the `memorija` binary — the `serve` subcommand, its `--stdio`/`--http` transport flags, `--port`, the bare-invocation default, and how invalid invocations are reported.
- `mcp-http-transport`: Serving the MCP server (and its `memory` tool) over Streamable HTTP — endpoint behavior, binding to the configured port, and running until stopped.

### Modified Capabilities

(none — `memory-tool` and `memory-root` requirements are transport-independent and unchanged)

## Impact

- **Code**: `src/main.rs` replaces its hardcoded startup with CLI dispatch; a new module for argument parsing; a new module (or section) for HTTP transport setup. stdio startup logic is reused as-is.
- **Dependencies**: adds `clap` (CLI parsing, with derive) to `Cargo.toml`; enables `rmcp`'s Streamable HTTP server feature (`transport-streamable-http-server`, which brings the underlying HTTP server stack).
- **APIs/behavior**: `memorija` with no arguments behaves exactly as before; `memorija serve --http` exposes the server on `127.0.0.1:<port>` over Streamable HTTP.
- **Systems**: for the first time the server accepts network connections (localhost by default); the memory root and filesystem behavior are untouched.
