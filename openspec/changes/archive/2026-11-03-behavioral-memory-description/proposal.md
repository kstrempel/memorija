# Proposal

## Why

The `memory` tool's description is currently a pure API reference: it explains the six commands and path rules but gives the model no reason to ever call the tool unprompted. As a result, memorija only learns about the user when the user explicitly asks. The goal is for memorija to learn about the user and their work automatically — memories captured silently as a background action — and the tool description is the only lever that works with strictly zero configuration (the server ships no system prompt and requires no client setup).

## What Changes

- Rewrite the `memory` tool description in `src/main.rs` from a mechanics-only reference into a behavioral description with three layers:
  - **Policy**: recall memory at the start of non-trivial tasks; continuously and silently capture durable, work-relevant facts (role, preferences, projects, decisions, people, systems, open threads) without asking or announcing; never store secrets or sensitive personal data.
  - **Organization**: a directory convention — `/memories/about/` (who the user is), `/memories/projects/<name>/` (per-project knowledge), `/memories/sessions/YYYY-MM.md` (a brief per-month running work log).
  - **Mechanics**: the existing path rules, six commands, and error semantics, preserved but compressed and moved last.
- Trivial exchanges trigger no memory operations, except that a durable fact is always recorded even if it surfaces in a short exchange.
- No changes to tool behavior, schema, commands, validation, or file operations — the tool's interface and implementation are untouched.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `memory-tool`: The "Single unified memory tool" requirement's description guidance changes: the description must not only explain paths and commands but must also instruct proactive, silent capture and start-of-task recall for non-trivial tasks, the directory convention, and the secrets exclusion.

## Impact

- `src/main.rs` — the `#[tool(description = ...)]` attribute on the `memory` tool is the only code touched.
- The description grows from ~150 to ~400 tokens; it is loaded into every MCP client request that lists tools.
- Models are steered to call `memory` more often (recall + silent capture); clients with weak tool-description adherence may not follow the policy — accepted limitation, since zero-config rules out alternative levers.
