# Design

## Context

Fresh Rust binary crate (`memorija`, edition 2024, no dependencies, hello-world `src/main.rs`). No existing code constrains the approach. See proposal.md for motivation and the two spec deltas (`memory-tool`, `memory-root`) for the behavior contract.

## Goals / Non-Goals

**Goals:**

- A single-binary stdio MCP server any MCP client can launch (`memorija` as the command in the client's MCP config).
- Input schema byte-for-byte equivalent to Anthropic's memory tool schema: `required: ["command", "path"]`, string enum for `command`, per-field descriptions.
- Path handling that cannot escape the memory root by any input the model can produce.
- Error messages written for the model, so failures self-correct in the next turn.

**Non-Goals:**

- No alternative memory backends (SQLite, vector store, sync). Plain files only.
- No config file or env-var configuration; the root is fixed at `~/.memorija/memories` in v1 (a `--root` flag can be added later without breaking the schema).
- No streaming HTTP transport; stdio only.
- No resource or prompt endpoints; tools only.

## Decisions

### D1: rmcp 3.x over a hand-rolled JSON-RPC loop

Use `rmcp` (stable 3.x, MCP spec 2026-07-28, compatible with 2025-11-25) with features `transport-io` (server stdio), default-on `server` and `macros`. Alternatives: hand-rolled JSON-RPC (small surface but reimplements protocol details like error shapes and negotiation for zero real gain), or the MCP spec by hand (same argument, worse). rmcp is maintained alongside the spec.

### D2: Schema via schemars derive on a flat struct, not a hand-written literal

```rust
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum Command { View, Create, StrReplace, Insert, Delete, Rename }

#[derive(Deserialize, schemars::JsonSchema)]
struct MemoryArgs {
    command: Command,
    path: String,
    file_text: Option<String>,
    old_str: Option<String>,
    new_str: Option<String>,
    insert_line: Option<i64>,
    insert_text: Option<String>,
    new_path: Option<String>,
    view_range: Option<[i64; 2]>,
}
```

schemars marks non-`Option` fields required and `Option` fields optional, so this derives exactly `required: ["command", "path"]` with `command` as a plain string enum — the flat Anthropic shape. A serde internally-tagged enum was rejected: it changes the schema shape (per-variant requireds) and diverges from the reference interface. If the derived schema drifts from the target (unexpected property names, missing descriptions), fall back to the `#[tool(input_schema = ...)]` override with a `serde_json::json!` literal — the macro supports it, so fidelity is guaranteed either way. Decide by diffing the generated schema against Anthropic's during implementation; that diff becomes a test.

> **Implementation note (diff result):** the generated schema matches the reference on all checked aspects — property set (9), `required` exactly `["command", "path"]`, six-value `command` enum, `view_range` an integer array. No literal-schema fallback was needed. One refinement: `command` deserializes through a wrapper (`CommandArg`) so unrecognized values reach dispatch as tool-level errors (spec: "Unsupported command") instead of protocol errors, while the schema still advertises exactly the six-value enum.

### D3: One tool, one handler, one dispatch match

`#[tool_router]` with a single `#[tool]` method taking `Parameters<MemoryArgs>`; dispatch on `command` in a match. Not one MCP tool per command: the unified schema *is* the Anthropic design — models were optimized against it, and splitting it changes the interface the spec pins down.

### D4: Recoverable failures as `isError` results, protocol errors only for uninterpretable requests

Operational failures (missing file, 2× `old_str` match, path escape, missing per-command field) return `CallToolResult` with `isError: true` and a message that states what happened and what to change. Unknown `command` values that survive deserialization, malformed JSON-RPC, etc. remain protocol errors. Rationale: tool-level errors are the model's feedback channel; JSON-RPC errors are the client integrator's. Alternatives considered: rejecting validation failures as `McpError` (breaks self-correction; rejected).

### D5: Containment by canonicalization, checked before every operation

Resolve every `path`/`new_path` as: strip optional leading `/memories`, join onto the root, normalize lexically, then canonicalize and require the result to start with the canonical root. Traversal (`..`) and absolute inputs that resolve outside are refused with an error result naming the rule. Symlinks: after lexical resolution, verify the canonical target stays under the root; creating symlinks via `create`/`insert` content is just bytes, but *following* an existing escaping symlink is refused. Root itself (`/memories`, empty path): legal for `view`, refused for `delete`/`rename`. Alternatives considered: per-operation chdir + relative paths (fragile across commands; rejected), rejecting all symlinks (over-broad; a symlink *within* the root is harmless and will simply work).

### D6: Line operations use a single in-memory file model

`view`, `str_replace`, and `insert` read the file, operate on a `Vec<String>` (or `Vec<Line>` with line numbers), and write back. `insert_line` is 0-based-before semantics: insert after line N, `0` = top; `N > line_count` errors reporting the count. `str_replace` operates on the full text (not lines) to keep multi-line replacements simple, matching Anthropic's edit semantics: replace exactly one occurrence, count matches first. Write-back is a plain write (no file locking); concurrent writers to the same memory are out of scope (one client owns the directory in practice).

### D7: Upsert create with recursive parent creation

`create` never errors on an existing file (overwrite) and creates missing parent directories via `create_dir_all`. Rationale: models use `create` both for initial writes and wholesale rewrites; erroring on exists would push the model into a delete-then-create dance that risks memory loss if delete hits the wrong path.

### D8: Seeding on first launch

When the server creates `~/.memorija/memories` at startup, it writes a starter `MEMORY.md` (short guidance: purpose of the directory, suggested organization, reminder that it persists across sessions). If the directory already exists, startup touches nothing. The seed content lives in the binary as a `const` string.

## Risks / Trade-offs

- [rmcp macro schema drift from Anthropic's exact schema] → Diff generated `inputSchema` against the reference in a test; D2's literal-schema fallback exists for this.
- [Model supplies bare relative paths with no `/memories` prefix (e.g. `notes.txt`)] → Accepted by design: D5 joins onto the root either way; the tool description documents both forms.
- [`insert_line` off-by-one silently corrupts memories] → Spec pins the semantics (after line N, 0 = top, beyond-EOF errors) and each scenario is a test case.
- [Overwriting the wrong file via upsert `create`] → Accepted trade-off of D7; mitigated by `view` before rewrite being the model's habit, not by gating.
- [Two clients sharing `~/.memorija/memories` concurrently] → No locking; last write wins. Documented limitation; per-client roots would need the deferred `--root` flag.
- [Blocking filesystem calls inside async handlers] → Use `std::fs` in `spawn_blocking` or accept the tiny blocking cost (files are small); decide in review, spec-visible behavior is identical.

## Migration Plan

Not applicable — greenfield. Rollback is deleting the binary and `~/.memorija` (the directory contains only memory files the user may want to keep; nothing else is created outside it).

## Open Questions

- Exact seed text for `MEMORY.md` — written during implementation, reviewed in the PR; does not affect schema or task structure.
