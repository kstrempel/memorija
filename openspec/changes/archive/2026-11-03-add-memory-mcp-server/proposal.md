# Proposal

## Why

AI coding assistants and other MCP clients have no persistent memory between sessions. Anthropic's memory tool design — a single `memory` tool exposing filesystem primitives under a designated memory root — is a proven interface for this, and no local Rust implementation exists. This project (a fresh Rust binary crate) is the vehicle for one.

## What Changes

- Add a Model Context Protocol (MCP) server, implemented with the official Rust SDK (`rmcp` 3.x), communicating over stdio so any MCP client (Claude, OpenCode, Codex, …) can connect.
- Expose exactly one tool, `memory`, whose input schema replicates Anthropic's unified memory-tool interface: required `command` (enum: `view`, `create`, `str_replace`, `insert`, `delete`, `rename`) and `path`, plus the optional per-command fields (`file_text`, `old_str`, `new_str`, `insert_line`, `insert_text`, `new_path`, `view_range`).
- Implement the six file operations against a local memory root at `~/.memorija/memories`, with path containment (no traversal or symlink escapes) and root protections.
- Seed the memory root with a starter `MEMORY.md` on first launch.
- Recoverable tool failures (missing file, ambiguous replacement, path escapes, etc.) are reported as tool results with `isError: true` so the model can self-correct; protocol failures remain JSON-RPC errors.

## Capabilities

### New Capabilities

- `memory-tool`: The unified `memory` MCP tool — its input schema, the semantics of each command (`view`, `create`, `str_replace`, `insert`, `delete`, `rename`), per-command argument validation, and error-result behavior.
- `memory-root`: The memory root directory — its location and creation, first-launch seeding, path containment and symlink policy, and protections for the root itself.

### Modified Capabilities

(none — the project has no existing specs)

## Impact

- **Code**: All new — `src/main.rs` grows from hello-world into the server binary; new modules for the tool handler, command dispatch, and filesystem layer. No existing behavior is modified or removed.
- **Dependencies**: adds `rmcp` (features: `server`, `macros`, `transport-io`), `tokio`, `serde`, `serde_json`; `schemars` is re-exported by `rmcp`.
- **Filesystem**: creates and manages `~/.memorija/memories` (only this directory and its contents).
- **Systems**: none — fully local, no network, no external services.
