# Design

## Context

Memorija is a single-crate Rust MCP server; one dispatch (`ops::run`) executes all six commands against plain files under `~/.memorija/memories`. There are no dependencies and no persistence beyond the filesystem. See proposal.md — Why for the motivation.

Two planned futures constrain the design: a possible database backend (Postgres) and a "dreaming" consolidation feature. Both are out of scope here, but the version record shape must survive them unchanged.

## Goals / Non-Goals

Goals:
- A logical version record model that is storage-neutral — flat files today, a database table later, with the same record shape.
- Model-queryable history and restore through the existing single `memory` tool, additively.
- Append-only history: no operation ever rewrites or removes a record.

Non-Goals:
- Undelete of removed paths and whole-directory restore (deferred until dreaming clarifies identity across paths).
- Content deduplication and retention/pruning policies (records are small; memory files are small; revisit when growth is observable).
- Any mechanism tied to the filesystem's nature: no git, no shadow-copy trees, no diffing of on-disk layout.

## Decisions

### Record the resulting content, keyed per path

A version record is `{ path, seq, content, cause, recorded_at }` where `seq` is a per-path 1-based sequence number. Records capture the state *after* the change; restoring to version N means taking record N's content.

- Alternatives: before-images (restore would need to reconstruct "latest before N", awkward for the first record), git (rejected in exploration: byte/tree-oriented history is meaningless once dreaming bulk-rewrites, and `.git` cannot move to Postgres), shadow-copy directories (no schema to port; pollutes the visible tree).
- Per-path numbering breaks under cross-path merges; that is acceptable until dreaming exists and is recorded as the known cutover point for an identity scheme (global sequence or content addresses).

### A version store module beside the memory root, not inside it

Records live in a dedicated store (layout: one append-friendly store directory next to the memories root, e.g. `~/.memorija/versions/`, structured so the same records map 1:1 to future DB rows). It must never appear in `paths::resolve` results, `view` listings, or be writable through the tool — the spec requires invisibility, so the store is resolved from the root's parent, and `ops::run` threads both roots.

- Alternative: a hidden `.versions/` directory inside the root — rejected; it would surface in listings, invite tool writes, and tangle with symlink containment.

### Causes are the originating command name; `restore` is the only synthetic cause

`create`, `str_replace`, `insert` record themselves; `restore` records `restore`. This gives dreaming its extension point (`consolidation`) without inventing behavior now.

### Diffs computed on the fly, returned as compact summaries

`history` computes each entry's summary (lines added/removed and a short unified-diff excerpt) from adjacent records at call time. No diff is persisted — records stay minimal and portable.

- Alternative: store precomputed diffs — rejected; derivable data would bloat records and the future DB schema.

### Restore composes existing primitives

`restore` reads the referenced record's content and writes it through the same code path as `create` (append record, cause `restore`). Restore onto a path whose live file was deleted is out of scope: `restore` requires the file to exist, which keeps the first cut to living files.

## Risks / Trade-offs

- [Unbounded record growth on hot files] → Acceptable for a first cut; memory files change a few times a day. Retention is a deliberate later change, and the append-only store makes pruning easy (drop old seq ranges).
- [Rename orphans history] → Documented per-path limitation (spec scenario); revisit with dreaming's identity model rather than inventing path-UUIDs now.
- [Record store and memory root can diverge if a user edits `~/.memorija` by hand] → Out-of-band edits simply have no records; `history` reports only recorded versions. No reconciliation logic in this change.
- [Every mutation now does an extra write] → Two small sequential writes per op; no measurable risk at memory-file scale.

## Migration Plan

No existing data is migrated: the store starts empty, files without records simply report none, and the six original commands behave identically (the `version` parameter is only honored when explicitly passed). Rollback is trivial — removing the versioning code leaves the memory tree untouched.

## Open Questions

- Exact on-disk layout of the store (one file per path vs. a single journal) — an implementation choice for apply; both map to the same records.
