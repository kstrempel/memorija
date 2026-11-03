# Tasks

## 1. Rewrite the tool description

- [x] 1.1 Replace the `#[tool(description = ...)]` string on `Memorija::memory` in `src/main.rs` with the behavioral description: policy (recall for non-trivial tasks, silent capture of durable facts, secrets exclusion), organization (`about/`, `projects/<name>/`, `sessions/YYYY-MM.md`, update over duplicate), then the existing mechanics (paths, six commands, error semantics) last. Verify the wording matches the delta spec's four ADDED requirements in `specs/memory-tool/spec.md`.
- [x] 1.2 Verify the full new description text is present and well-formed in the compiled tool listing: `cargo build` succeeds and the description string compiles without attribute errors.

## 2. Verify behavior is unchanged

- [x] 2.1 Run `cargo test` and confirm the full existing suite passes — the tool's runtime behavior, schema, and command semantics are untouched, so all `ops.rs` tests must still pass.
- [x] 2.2 Run `openspec validate --strict` and confirm the change and its delta spec validate cleanly.

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
