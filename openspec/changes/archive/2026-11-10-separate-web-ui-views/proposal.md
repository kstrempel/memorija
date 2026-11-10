# Proposal

## Why

The web viewer puts the changes feed, the all-files index, and the rendered file in one scrolling column. Each part steals height from the others: the file you came to read sits below two lists, neither list can grow (no grouping, filters, or metadata) without making the other worse, and only the current content of a file is reachable even though the version store keeps every version. Splitting the one page into dedicated screens lets each job get the space and the controls it needs.

## What Changes

- **Three screens, three addresses.** The viewer grows a shared header with navigation and renders one screen at a time: Changes (`/changes`), Files (`/files`), and the Reader (`/file?path=…`). `/` lands on Changes. Each address works on refresh, bookmark, and browser back/forward.
- **Changes screen.** The feed stays newest first but is grouped by day, with a timespan filter (`24h`, `7d`, `all`, the same choices as the CLI's timespan grammar) and a path filter. Selecting a change opens the Reader at that recorded version.
- **Files screen.** The all-files index is grouped by directory and shows, per file, its line count, latest recorded version, and last change. A client-side filter narrows the list. Selecting a file opens the Reader at its latest version.
- **Reader screen.** The file is rendered from markdown at a selected version, with a version strip built from its recorded history and an explicit back link to the screen it was opened from. A past version is visibly marked as such and offers a return to the latest.
- **Live updates survive navigation.** One Server-Sent Events stream feeds every screen, so switching screens does not reconnect or reload. The Changes screen surfaces new activity with an indicator instead of reordering rows under the pointer.
- **API grows for the screens.** `/api/files` returns per-file metadata instead of bare paths, and `/api/file` accepts a `version` parameter and returns the file's version timeline alongside the rendered content, so the Reader needs one request per screen.
- **Compatibility.** Existing `/?path=<file>` addresses continue to resolve to the Reader, so old bookmarks keep working.
- **Assets split, still no build step.** The single embedded page becomes `index.html`, `app.css`, and `app.js`, embedded in the binary and served from the same origin; the Content-Security-Policy tightens from inline allowances to `'self'`.
- **Read-only throughout.** No screen gains a way to create, edit, delete, or restore a memory file or version record.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `web-ui`: The viewer's page structure, addresses, navigation, per-screen contents, live-update behavior, and JSON surface change; the existing read-only, path-safety, and rendered-markdown guarantees stay as they are.

## Impact

- `src/web/mod.rs`: routes for the screens and static assets, extended `/api/files`, `/api/file` version parameter and version timeline, redirect handling for `/?path=`, CSP header.
- `src/web/page.html`: replaced by `index.html`, `app.css`, and `app.js` (client router, screen rendering, SSE handling).
- `src/web/markdown.rs`: unchanged rendering; reused for historical versions.
- `src/inspect.rs` / `src/versions.rs`: read-only reuse of the change scan and per-file version records; no data-format change.
- `tests/`: the web module tests and `tests/web_posture.rs` assumptions about the single page, the `/api/files` payload shape, and the endpoint references on the page.
- `README.md` web viewer section; crate version bump on archive.
- Builds on the all-files index introduced by the `add-web-file-browser` change now in the working tree.
