# Proposal

## Why

Memory files are written and updated by agent sessions throughout the day, but there is no human-friendly way to look at them: the CLI prints raw text, and reading a file with rendered markdown requires knowing which file changed and opening it by hand. A local web viewer turns the version store — which already records every change — into a live, readable view of recent memory activity.

## What Changes

- New `memorija web` subcommand that starts a local, read-only web server (axum, already a dependency) and opens the main page in the system browser when possible, printing the URL either way.
- Main page shows a live changes feed (from the version store, newest first) with the most recently changed file's content below, rendered server-side from markdown to HTML.
- Clicking a feed row shows that file's rendered content, with per-file URLs so a browser refresh returns to the same file.
- Live updates: the server polls the version store (~1s) and pushes changes to the page over Server-Sent Events — the page refreshes its feed and content without manual reload.
- Small refactor: the store scan/sort/diff-count logic inside `inspect::changes` is extracted into a structured, shared form used by both the CLI text output and the web JSON API (no CLI behavior change).
- Safety posture matching the existing MCP HTTP transport: loopback bind by default, a warning on non-loopback binds, and path handling reusing the existing `paths::resolve` sandbox.

Out of scope for this change: editing memories from the web UI (create/str_replace/insert/delete/rename/restore), a file-tree browser, authentication, and any non-local access story.

## Capabilities

### New Capabilities

- `web-ui`: The local, read-only web viewer — the `web` subcommand and its flags, the served page (changes feed, rendered markdown content, per-file URLs), the live-update behavior, the JSON/SSE endpoints behind it, and the binding/security posture.

### Modified Capabilities

<!-- None: the CLI gains a subcommand without changing any existing command's
     observable behavior. Following the `cli-inspection` precedent, the new
     subcommand's surface is owned by the new `web-ui` capability rather than
     by `cli`. -->

## Impact

- **Code**: new module for the web server (handlers, embedded page, markdown rendering); small extraction in `src/inspect.rs` (structured store scan shared with `src/cli.rs` output); new subcommand wiring in `src/cli.rs`/`src/main.rs`.
- **Dependencies**: one new crate for markdown→HTML (planned: `pulldown-cmark`); browser launch via the platform opener (e.g. `/usr/bin/open` on macOS) or a small helper crate.
- **Runtime**: a second local HTTP server beside the MCP one; default port 8081 so `serve --http` (8080) and `web` can run at the same time.
- **Data**: read-only consumers of the existing memory root and version store; no store format or memory file changes.
