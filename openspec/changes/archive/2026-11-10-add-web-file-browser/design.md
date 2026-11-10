# Design

## Context

The viewer (`src/web/mod.rs`) is an axum app serving one embedded page (`src/web/page.html`) plus three read-only JSON/SSE endpoints: `/api/changes` (feed from `inspect::scan_changes`), `/api/file?path=…` (one file, rendered via `web::markdown`, resolved through `paths::resolve`), and `/api/events` (SSE fed by a ~1s version-store poller through a `watch` channel). Files are currently reachable only via feed entries, so files without recorded changes are invisible. See proposal.md — Why.

## Goals / Non-Goals

**Goals:**
- A complete index of memory files, served read-only and kept live through the existing poll/SSE machinery.
- Minimal surface: reuse the existing navigation (`select()`, `/?path=…`), rendering, and path-safety code unchanged.

**Non-Goals:**
- Any write capability; authentication; directory-tree UI with expand/collapse; search or filtering over files; changes to the version store format.

## Decisions

- **D1 — Listing endpoint over the real filesystem, not the version store.** Add `GET /api/files` returning `{ "files": [<relative paths>] }` by walking the memory root with a recursive directory read, skipping the version-store directory (`.memorija/` under the root, whatever `versions::store_dir` yields) and hidden entries. Rationale: the requirement is "every memory file in the root", which the store does not know for never-recorded files. Alternative considered — deriving the list from store records — rejected: it misses files without history and conflates feed with index.
- **D2 — Relative-path normalization via `ops::rel_key`.** The walker produces paths the same way `/api/file` resolves them, so every listed path is directly usable in `/?path=…` and guaranteed inside the sandbox. The listing itself only reads metadata and never follows a symlink target outside the root: a symlinked entry is listed by its in-root path (and `api_file`'s existing refusal still applies when opened).
- **D3 — Index rides the existing poller/SSE channel.** Extend the poller's watch payload to carry both feed and file list (e.g. emit a second SSE event type `files`, or include the list in the same JSON). Rationale: the store poll already wakes once per second; file appearance is normally accompanied by a store record (create), so piggybacking keeps one timing mechanism. Edge: a file created/removed *without* a store record (manual touch) would not move the store — the poller's signal therefore also cheaply stats the root's top-level shape (entry count + newest mtime), so filesystem-only movement triggers the push too. Alternative — a separate filesystem-watch (notify) crate — rejected: new dependency for an edge case.
- **D4 — UI: an "All files" section under the feed.** Same row styling as the feed, sorted by path, clicking calls the existing `select(path)`. A file can be highlighted as selected in both lists. Rationale: keeps the single-column page layout and the mobile view usable; a sidebar split is a larger layout change than the request warrants. A folder tree can come later without breaking this contract.

## Risks / Trade-offs

- [Large memory roots make the page long] → The index is compact text rows; if roots grow very large, virtualization or pagination can be added later without changing the endpoint contract.
- [Walking the root on every poll tick costs I/O] → The walker only runs when the cheap stat signal moves; an idle root is never walked.
- [Root-shape polling could miss movement deeper in the tree between ticks] → Same ~1s granularity the feed already has; the spec says "within a few seconds", which one poll interval satisfies.
- [Listing leaks file names of otherwise unrecorded files] → Same exposure the tool already grants any local reader of the root; the viewer remains loopback-by-default with the existing non-loopback warning.

## Migration Plan

Single binary, no data format changes; ship as a normal version bump. Rollback is reverting the commit — no persisted state.

## Open Questions

(none)
