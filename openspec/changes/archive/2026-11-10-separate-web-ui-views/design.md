# Design

## Context

See proposal.md — Why. The viewer today is one embedded `page.html` served at `/`, plus `/api/changes`, `/api/files`, `/api/file?path=…`, and `/api/events` (SSE over a 1s poll of the version store and the memory root's top level) in `src/web/mod.rs`. The page fetches everything client-side; there is no front-end build step, no framework, and the CSP allows only inline style/script plus same-origin connections.

Constraints that shape this design:

- The binary must stay self-contained: assets are embedded and served from the same origin, no vendored JS.
- The viewer is read-only; it may read the memory root and the version store, never write either.
- Path safety stays in `paths::resolve`; markdown rendering stays in `web::markdown`.
- Available data: `inspect::scan_changes` (all changes newest first, with diff counts), `versions::read` (all records for one path), `ops::record_at` (one record by sequence), `ops::diff_counts`, `ops::rel_key`.
- Current tests pin the one-page document (its endpoint references and empty-state text) and the exact `/api/files` JSON, so they change with this design.

## Goals / Non-Goals

**Goals:**

- Three screens — Changes, Files, Reader — each at its own address, sharing one live connection.
- Reader can show any recorded version of a file, and a change opens the version it recorded.
- No new dependency, no build step, and a tighter Content-Security-Policy than today.
- Keep read-only, path-safety, and markdown-escaping guarantees unchanged.

**Non-Goals:**

- Editing, creating, deleting, or restoring memories from the browser.
- Diffing two versions, searching file contents, authentication, pagination or virtualisation.
- Viewing versions of a file that no longer exists on disk (the CLI has the same limit today).
- Sub-second updates; the existing ~1s poll is the budget.

## Decisions

### D1: One shell with a client router, not one server-rendered page per screen

The server returns the same embedded shell for `/`, `/changes`, `/files`, `/file`, and any other non-API path; `app.js` reads `location.pathname` and `location.search` and renders the matching screen. An unknown path renders the not-found screen with a way back to Changes.

Alternatives considered:

- **Server-rendered page per route.** Each screen becomes a Rust handler that builds HTML. This ports the entire rendering path into Rust, duplicates layout chrome, and still needs JS for live updates. It would only be worth it if the viewer had to work without JS; today's page does not either.
- **Several embedded HTML files.** Duplicates the header, the SSE code, and the router-less navigation; every asset change touches several files.

The shell approach keeps one layout, one connection, and instant navigation. Its cost — a blank page without JavaScript — is the status quo, and a `<noscript>` line explains it.

### D2: Split the embedded page into three assets and tighten the CSP

`index.html`, `app.css`, `app.js`, each `include_str!`ed and served from `/`, `/app.css`, `/app.js` with the right content types and `Cache-Control: no-cache` (so a rebuilt binary never serves a stale asset from browser cache). The CSP becomes:

```
default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'
```

No inline handlers, no inline styles, no `eval`. This is stricter than today's `'unsafe-inline'` allowances while adding no remote origin. Images in markdown stay blocked, exactly as the current `default-src 'none'` already does.

### D3: Addresses carry the screen state

| Address | Screen | Meaning |
|---|---|---|
| `/` | Changes | default entry point, no filter written to the URL |
| `/changes?span=24h&path=notes` | Changes | `span` ∈ `24h`, `7d`, `all` (default `24h`); `path` is a substring filter |
| `/files?q=proj` | Files | `q` is a substring filter on paths |
| `/file?path=<rel>&version=<n>&from=changes\|files` | Reader | `version` absent means current; `from` picks the back link target |

- `/?path=<rel>` is translated on load to `/file?path=<rel>` with `history.replaceState`, so old bookmarks keep working without a server route.
- Unknown non-API paths serve the shell; the client shows the not-found screen.
- Filters live in the address so reload, bookmark, and back/forward restore them (spec: viewer presents each screen at its own address).
- `/` deliberately does not rewrite itself to `/changes`, avoiding a history entry that fights the back button; the first filter choice navigates to `/changes?…`.

### D4: Extend the JSON surface instead of adding parallel endpoints

`GET /api/files` returns objects, not bare strings:

```json
{ "files": [
  { "path": "notes.md", "lines": 38, "latest_version": 3,
    "last_changed": "2026-11-10T12:31:04Z" }
] }
```

`lines` is `null` for an unreadable or non-text file; `latest_version` and `last_changed` are `null` without recorded history.

`GET /api/file?path=…&version=…` returns one rendered version plus the metadata the Reader needs in the same response:

```json
{ "path": "notes.md", "version": 2, "latest_version": 3, "is_current": false,
  "html": "<h1>…</h1>", "history": [
    { "seq": 1, "recorded_at": "…", "cause": "create", "added": 12, "removed": 0 }
  ] }
```

- No `version`: render the file's current content; `version` is the latest recorded sequence or `null`, `is_current` is `true`.
- With `version=n`: render that record's content through `web::markdown`; `is_current` is `n == latest_version`. An unknown version is a 400 naming the available range, reusing `ops::record_at`'s message.
- `history` is oldest first (matching `versions` in the CLI) with `+N -M` against the previous record, computed by `ops::diff_counts`.
- Bundling history here avoids a second round trip and a second endpoint; full content of past versions is never shipped in bulk.

`GET /api/changes` and `GET /api/events` keep their shapes except that the pushed `files` array becomes the object array above. **Filtering happens in the browser** over the single pushed payload: the timespan presets are fixed, the path filter is a substring, and the server would otherwise have to filter per client on a broadcast stream. This leaves the CLI's timespan parser with its current owner (the CLI); the browser implements the three presets by comparing stored `YYYY-MM-DDTHH:MM:SSZ` strings, which sort chronologically.

### D5: One live stream, per-screen update behavior

The existing poller and `watch` channel stay: a `MovementSignal` (store mtime/size + root top-level count/newest mtime) gates a rebuild of `{changes, files}`. On movement, each open screen applies the new payload:

- **Changes:** re-render the day groups; when new changes arrive while the user is scrolled away from the top, show a "N new changes" affordance instead of yanking the list.
- **Files:** add/remove/replace rows in place.
- **Reader:** compare the displayed file's `latest_version` against the payload; if it advanced, show an "updated — view vN" banner that switches to the new version on click. It does not silently replace content the user is reading.

The client opens one `EventSource` for the page's lifetime; navigation never reconnects it.

Known limit, unchanged from today: the poll signal covers the store and the root's top-level shape, so a hand edit of a *nested* file with no version record does not trigger a push. Agent writes always do, and that is the live case; closing the gap would need a per-tick recursive stat, which the tool does not need yet.

### D6: File metadata costs one read per file per push

Version and last-change come free from the same `scan_changes` output the feed already uses, joined by path. Line counts require reading each file. The walker therefore builds the list by reading each file once when the signal moves — no work while idle. For a personal root of dozens of small files this is negligible; if roots grow, the count column becomes lazy or the payload pages, without changing the endpoint shape.

### D7: Vanilla router with real links

Navigation uses `<a href>` elements with intercepted clicks (`pushState`) so middle-click, copy-link, and open-in-new-tab work; `popstate` re-renders. Screen renderers build lists with DOM APIs and `textContent` for user-controlled strings; `innerHTML` is used only for the server-rendered markdown HTML, preserving the existing trust boundary. No template strings containing file content, no `eval`.

### D8: Version strip shows every recorded version

The Reader renders `history` as a horizontally scrollable strip of version chips, oldest to newest, with the current selection marked and scrolled into view. No elision logic: a file with 40 versions gets a scrolling strip, which is honest and trivial. Selecting a chip navigates to `/file?…&version=n`, so the choice is addressable and survives reload.

### D9: Back link uses `from`

The Reader's back link targets `from` (`changes` or `files`), defaulting to Changes. It is carried in the address rather than in history state so a reload keeps it. This is a display hint only; it never restricts navigation.

### D10: Error handling stays inside the shell

- Unknown screen path → shell + not-found screen with a link to Changes.
- Missing file, directory, unreadable file, path escape → the Reader's error screen, keeping the page and header usable (`/api/file` still returns 400/404 with the same message text).
- Unknown version → 400 naming the available range, shown as the Reader's error screen for that file.
- A failed `/api/changes` or `/api/events` connection shows an inline notice on the visible screen; the page keeps working and `EventSource` reconnects on its own.

## Risks / Trade-offs

- **The pushed payload grows with the whole change history and file list.** → Fine for personal roots today; if it becomes a problem, the payload can page or push a delta without changing the screen contracts.
- **Line counts read every file on every push.** → Gated by the movement signal, so idle costs nothing; a large root can drop the column to lazy without an endpoint change.
- **Hand edits to nested files are invisible until a version is recorded.** → Documented limit, already present today; agent writes always arrive through the store.
- **A blank page without JavaScript.** → Status quo; add a `<noscript>` line and keep the server-side data endpoints simple so a future no-JS fallback is possible.
- **Test churn.** → The page test (`root_serves_the_embedded_page`), the `/api/files` shape tests, and `web_posture::fresh_machine_shows_the_empty_state` (which greps the page for the empty-state text) must be updated deliberately; the empty-state text moves to `app.js`.
- **CSP tightening can break the page silently.** → Serve assets with correct content types, avoid inline event attributes, and add a test asserting the new header and that `/app.js` and `/app.css` are served.

## Migration Plan

Additive for users: same binary, same `web` subcommand, same default address; `/` still lands on the changes feed. Old `/?path=` links resolve to the Reader. No persisted state or data-format change, so rollback is reverting the commit.

## Open Questions

None.
