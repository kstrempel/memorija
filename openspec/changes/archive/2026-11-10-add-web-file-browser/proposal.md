# Proposal

## Why

The web viewer only reaches a memory file through the changes feed: a file is listed once it (or a path that feeds into it) has recorded changes, and files that never changed recently — or on a machine whose store was cleared — are invisible even though they exist in the memory root. The user asked to be able to read all memories from the web UI, so the viewer needs a complete, always-current index of every memory file alongside the feed.

## What Changes

- Add a read-only "all files" index to the web viewer: every memory file in the memory root, listed with its path, opening in the existing rendered file view.
- Add a read-only JSON endpoint that walks the memory root and returns the relative paths of all memory files (path-safety rules unchanged — it only lists inside the root).
- The index updates live like the feed: when a new file appears in the memory root (or a file disappears), the list follows without a manual reload.
- The file view remains unchanged: same URL scheme (`/?path=…`), same rendering, same empty state and error handling.
- No write capability is added; the viewer stays strictly read-only.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `web-ui`: New requirement — the viewer shows an index of all memory files in the root, navigable to the existing per-file rendered view, kept live without manual reload; the index reveals files regardless of their change history.

## Impact

- `src/web/mod.rs`: new listing endpoint (reusing the root scan), route registration, index pushed through the existing store poller so it updates with the feed.
- `src/web/page.html`: an "all files" list (sidebar or section) next to the changes feed, wired to the existing `select()` navigation.
- No changes to `openspec/specs/web-ui/spec.md` requirements beyond the added one; no dependency changes; the viewer's read-only and path-safety guarantees are untouched.
