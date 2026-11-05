# Design

## Context

The binary is a single Rust crate: an MCP server (`serve`) plus read-only inspection commands (`list`, `show`, `versions`, `changes`). axum and tokio are already dependencies for the MCP HTTP transport (`src/serve.rs`), and `src/inspect.rs` already contains the store scan, newest-first sort, and `+N -M` diff counts behind the `changes` command — but exits through human-formatted strings. The version store (`src/versions.rs`) appends one JSONL line per mutation, so it doubles as a change signal. Paths from any client are sandboxed by `src/paths.rs`.

The decisions below were explored and confirmed with the user; see proposal.md for scope.

## Goals / Non-Goals

**Goals:**

- One binary, no front-end build step: the web page is embedded in the executable and works offline.
- Live view of memory activity with latency of at most a few seconds.
- Same security posture as the existing MCP HTTP transport.
- A structured store scan shared by CLI and web, so the two never disagree.

**Non-Goals:**

- Editing memories from the browser (later change, would reuse `ops::run`).
- A file-tree browser, authentication, TLS, or multi-user concerns.
- Instant (sub-second) change notification.

## Decisions

- **D1: Separate `web` subcommand, not a `serve` flag.** `serve`'s flags are MCP-transport-shaped (`--stdio`/`--http` conflict rules); a viewer is a different beast. A top-level `web` subcommand keeps clap validation, defaults, and help text clean, and follows the `cli-inspection` precedent of new surfaces as new commands. Its CLI requirements (port/host defaults, non-loopback warning, bind failure) mirror the `mcp-http-transport` spec's posture.
- **D2: Server-side markdown rendering with `pulldown-cmark`.** The handler converts a file's content to HTML before responding; the browser page renders received HTML directly. Alternatives: client-side JS markdown library (adds a vendored third-party blob and a trust boundary) or termimad (terminal/ANSI output, not HTML). pulldown-cmark escapes raw HTML by default (`Options::all()` includes it; we deliberately do not enable `ALLOW_HTML`... note: escape options are chosen so embedded tags render as text), which satisfies the no-script requirement with no extra sanitizer.
- **D3: Single embedded page, no JS dependencies.** One HTML file with inline CSS/JS, embedded via `include_str!` from `src/web/`. The page talks to three JSON endpoints and one SSE endpoint. Vanilla JS is enough for: fetch feed, fetch file, click routing, one `EventSource`. No npm, no CDN, works offline.
- **D4: Live updates = poll the version store, push over SSE.** A background task checks the store every ~1s: cheapest reliable signal is the max mtime plus total size across `versions/*.jsonl`; a change triggers an SSE event with the latest records. Chosen over the `notify` crate (FSEvents/inotify) to avoid a platform-sensitive dependency for a personal tool where 1s latency is fine. The browser side reconnects automatically (`EventSource` semantics).
- **D5: Extract a shared structured scan in `inspect.rs`.** New internal function returning the newest-first change list (timestamp, path, seq, cause, added, removed) as data; `changes` keeps its exact current text output on top of it, and the web API serializes the same data as JSON. No CLI behavior change; existing `changes` tests keep passing untouched.
- **D6: Endpoints.** `GET /` (embedded page), `GET /api/changes` (feed JSON), `GET /api/file?path=…` (content JSON: path, rendered HTML, version info), `GET /api/events` (SSE). File requests go through `paths::resolve`, satisfying the escape-refusal requirement by construction. Per-file URLs are `/?path=<rel>` — simple, avoids a second HTML route, refresh-safe.
- **D7: Browser launch.** Spawn the platform opener (`/usr/bin/open <url>` on macOS, `xdg-open` on Linux, `cmd /c start` on Windows); failure to spawn is non-fatal because the URL is printed regardless. No new dependency for one `Command::spawn` per platform; a helper crate (`open`) is the alternative if the platform matrix grows annoying.

## Risks / Trade-offs

- [XSS via markdown edge cases] → pulldown-cmark's HTML escaping plus a page-side `Content-Security-Policy` header (`default-src 'none'`-style, inline style/script allowed but no remote origins) as defense in depth; a test renders a `<script>`-bearing file and asserts no live tag in the output.
- [Version store grows large; 1s full rescan gets slow] → the scan reads only file metadata per tick and re-reads JSONL contents only when the store moves; if personal stores ever outgrow this, the tick can hash tails instead — no interface change.
- [Feed semantics drift between CLI and web] → both consume the one shared scan (D5); existing CLI tests pin the text behavior.
- [Port 8081 already in use] → same clear bind-failure exit as `serve --http`; `--port` is the remedy.
- [Non-loopback bind exposes unauthenticated read access] → warning printed, exactly matching the MCP HTTP transport's posture.

## Migration Plan

Additive: new subcommand, new module, one internal extraction with unchanged CLI output. No data migration; the version store is read as-is. Rollback is reverting the commit.

## Open Questions

None.
