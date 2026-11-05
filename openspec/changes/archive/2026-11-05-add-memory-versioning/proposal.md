# Proposal

## Why

Every memory mutation is destructive: `create` silently overwrites, `delete` removes recursively and permanently, and no past state survives. The tool's only client is an LLM, which is exactly the kind of writer that occasionally deletes the wrong subtree or overwrites a file it meant to edit. Beyond recovery, there is no way to answer "what changed in this memory and when" — a question the model itself should be able to answer when the user asks it. Two planned futures shape the design now: memories may later move to a database backend, and a "dreaming" feature will consolidate and summarize memories in bulk, so versioning must live at a logical, storage-neutral layer and must distinguish what caused each change.

## What Changes

- Add a logical version record layer beneath the file storage: every mutation of a memory file appends a version record containing the path, full content, a cause (`create`, `edit`, `restore`; consolidation causes reserved for later), and a timestamp. Records are plain data — the same shape a future database table would hold — so storage can later move to Postgres without translating history.
- Extend the `memory` tool's command enum with two additive commands: `history` (timeline of a file's versions with compact diffs) and `restore` (re-create a file's content from a past version). The six existing commands keep their exact semantics.
- Add an optional `version` parameter to `view` and `restore` for reading and restoring full past content.
- Restore is append-only: restoring to an old version creates a new version with cause `restore`; history is never rewritten.
- First cut covers living files only. Undelete of deleted paths and whole-directory restore are explicitly out of scope; `delete` keeps removing the live file, with its version records retained as a side effect of record retention.

## Capabilities

### New Capabilities

- `memory-versions`: The version record layer for memory files — what constitutes a version record, when records are appended, how history is presented, and what restore does. Storage-neutral by design: records are logical data, not filesystem artifacts.

### Modified Capabilities

- `memory-tool`: The tool schema changes — `command` gains `history` and `restore` values, `version` becomes an optional property honored by `view` and `restore`, per-command validation rules cover the new commands, and the tool description documents versioning capabilities. The single-tool requirement and all six original command semantics are unchanged.

## Impact

- `src/args.rs` — new `Command` variants, `version` argument parsing, updated tool schema/description text.
- `src/ops.rs` — new `history` and `restore` handlers; all mutating commands append version records.
- New module for the version record store (implementation detail recorded in design.md; must not leak record storage into the memory root's visible tree).
- `openspec/specs/memory-tool/spec.md` — schema and validation deltas.
- No new external dependencies; existing memory files under `~/.memorija/memories` are unaffected in content or location.
