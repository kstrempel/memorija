# Tasks

## 1. Server skeleton

- [x] 1.1 Add dependencies to `Cargo.toml` (`rmcp` with `transport-io` feature, `tokio`, `serde`, `serde_json`) and verify `cargo build` succeeds
- [x] 1.2 Implement the minimal rmcp server (`#[tool_router]` with a stub `#[tool]` for `memory`, `#[tool_handler]`, `serve` over stdio) and verify it answers an `initialize` + `tools/list` handshake when driven by a JSON-RPC script over stdin/stdout
- [x] 1.3 Define `Command` (string enum) and `MemoryArgs` (flat struct, `command`/`path` required) and verify a unit test asserts the generated schema has `required: ["command", "path"]`, the six-value command enum, and all seven optional properties — diff against Anthropic's reference schema and record the result

## 2. Path containment (`memory-root`)

- [x] 2.1 Implement root resolution (`~/.memorija/memories`), first-launch creation, and `MEMORY.md` seeding (never overwrite an existing one); verify with a test against a temp-HOME root: fresh start creates both, second start changes nothing
- [x] 2.2 Implement path resolution: strip optional `/memories` prefix, join onto root, canonicalize, enforce containment; verify with unit tests for traversal (`/memories/../..`), bare-relative paths (`notes.txt`), and bare root (`/memories`) — each escapes-or-resolves as the spec requires
- [x] 2.3 Implement symlink containment (canonical target must stay under root) and root protections (`delete`/`rename` on the root refused, `view` allowed); verify with tests using a symlink into a temp dir outside the root

## 3. Command implementations (`memory-tool`)

- [x] 3.1 Implement `view` for files (1-based numbered lines, `view_range` with `-1` end sentinel, beyond-EOF error) and directories (recursive listing with line counts, `view_range` ignored); verify each spec scenario with a unit test on temp files
- [x] 3.2 Implement `create` (upsert with recursive parent creation); verify tests for nested creation and overwrite
- [x] 3.3 Implement `str_replace` (unique-match replace, match-count error on multiples, zero-match error, omitted `new_str` deletes); verify each behavior with a unit test, including a multi-line `old_str`
- [x] 3.4 Implement `insert` (after line `insert_line`, `0` = top, beyond-EOF error reporting the line count); verify each scenario with a unit test
- [x] 3.5 Implement `delete` (recursive, missing-path error) and `rename` (move with parent creation, existing-destination error); verify each scenario with a unit test
- [x] 3.6 Implement per-command argument validation returning `isError` results with messages naming the missing field; verify tests for create-without-`file_text`, str_replace-without-`old_str`, rename-without-`new_path`, and an unknown `command` value

## 4. Integration verification

- [x] 4.1 Wire the dispatch into the tool handler with errors surfaced as `isError: true` results and verify with an end-to-end test driving the binary over stdio: initialize, list tools, and run a create → view → str_replace → insert → rename → delete sequence, including one failure the model-style client retries after reading the error message
