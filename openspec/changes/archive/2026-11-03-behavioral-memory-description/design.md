# Design

## Context

The tool description lives in a single `#[tool(description = ...)]` attribute on `Memorija::memory` in `src/main.rs` (rmcp macro). It is currently mechanics-only. The server is strictly zero-config: it ships no system prompt and no client-side setup, so the description is the only place behavior-steering text can live. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**
- The model recalls memory at the start of non-trivial tasks and silently captures durable facts during work.
- Memory stays navigable over time via a fixed directory convention.
- Mechanics documentation is preserved (required by the existing `memory-tool` spec).

**Non-Goals:**
- Any change to the tool's schema, commands, validation, or file operations.
- Server-side enforcement of the policy (e.g., auto-injection of memory content into results) — the description is advisory text; adherence depends on the client's model.
- Alternative steering levers such as shipping an AGENTS.md snippet (rejected: zero-config).

## Decisions

- **Description carries policy, organization, mechanics — in that order.** Policy changes behavior and must be read as instruction; mechanics only matter at call time. The existing spec's scenario ("Tool description guides usage") only requires mechanics to be present, not first.
- **Recall is gated on non-trivial tasks.** Saves a tool call and tokens on drive-by questions. The gate is model-judged; wording is unconditional within its scope ("for non-trivial tasks... view /memories") rather than "when relevant," which models follow weakly.
- **Capture is not gated by task size.** A durable fact surfacing in a trivial exchange (e.g., a build-tool preference dropped in passing) is exactly the fact worth keeping. Only the recall and the session log pay the per-session cost; capture pays per-fact.
- **"Background action, never part of the reply"** is the operative phrase for silence — models otherwise tend to append "I've saved this to memory" to their reply.
- **Month-per-file session log (`sessions/YYYY-MM.md`)** keeps the root listing at ~3 entries forever and lets old months age out; it is the only part of the tree allowed to grow, and it grows slowly. Alternative considered: a single `sessions.md` (simpler, but grows unbounded) or per-session files (root listing explodes).
- **Description grows from ~150 to ~400 tokens.** Accepted: this is the one tool meant to run constantly; the cost is paid on every tools-listed request, but it is what makes zero-config memory work.

## Risks / Trade-offs

- [Weak clients ignore behavioral text in tool descriptions] → Accepted limitation; zero-config rules out system prompts. The description uses imperative, unconditional phrasing, which models follow best.
- [Model over-captures, memory becomes noise] → The description scopes capture to durable, work-relevant facts, forbids one-off task details, and directs updating existing memories over duplicating.
- [Model captures something sensitive] → The description explicitly forbids secrets, credentials, and sensitive personal data. Enforcement remains the model's judgment; no server-side filtering (would break the single-tool file-API design).
- [Policy wording drifts back to mechanics-only in a future edit] → The delta spec's ADDED requirements pin the behavioral content; `openspec validate` keeps it honest.

## Migration Plan

None needed: single-string change, no data migration, rollback is reverting the attribute. Existing memory roots keep working; the layout convention applies to what the model writes next.

## Open Questions

None.
