# Tasks

## 1. Server: screens, assets, and the data the screens need

- [x] 1.1 Split the embedded page into `src/web/index.html`, `src/web/app.css`, and `src/web/app.js` (minimal shell for now: header, nav, empty screen containers), embed all three, and serve `GET /`, `/changes`, `/files`, `/file`, plus a non-API fallback with the shell; serve `/app.css` and `/app.js` with `text/css` / `javascript` content types and `Cache-Control: no-cache`, and replace the CSP with `default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'`. Verify with tests that each screen address and an unknown path return the document, both assets return 200 with their content type, and `/` carries the tightened CSP.
- [x] 1.2 Change `GET /api/files` to return objects `{path, lines, latest_version, last_changed}`, joining the existing root walk with the newest change per path from `scan_changes` and counting lines from the file read; `lines` is `null` for unreadable files and version/time are `null` without history. Verify with tests for a seeded root mixing recorded, unrecorded, nested, hidden, and unreadable files, and for a missing root returning an empty list.
- [x] 1.3 Extend `GET /api/file` with an optional `version` parameter and a `history` array (`seq`, `recorded_at`, `cause`, `added`, `removed`, oldest first) computed with `ops::diff_counts`, returning `{path, version, latest_version, is_current, html, history}`; render the requested record through `web::markdown`, keep current content when no version is given, and answer an unknown version with a 400 naming the available range. Verify with tests for the latest view, a past version's rendered content, `is_current`, history ordering and counts, and the unknown-version error text.
- [x] 1.4 Update the existing web tests that pin the old single-page and `/api/files` shapes (`root_serves_the_embedded_page`, the file-listing tests, and any payload assertions), and run `cargo test web` to confirm the suite matches the new contract.

## 2. Client: router, screens, and live updates

- [x] 2.1 Build the client router in `app.js`: render Changes on `/`, Files on `/files`, Reader on `/file`, show a not-found screen for any other path, translate `/?path=…` to `/file?path=…` with `history.replaceState`, use real `<a href>` links with intercepted clicks and a `popstate` handler, and keep the header's current screen marked. Verify in the browser against a seeded root that each address renders its screen, reload and back/forward restore it, and `/?path=notes.md` lands on the Reader.
- [x] 2.2 Implement the Changes screen: day-grouped feed rows (time, path, cause, `+N -M`, version), timespan chips (`24h` default, `7d`, `all`), a path filter, both carried in the address, a clear empty state for a fresh machine and for filters that match nothing, and a "new changes" affordance when activity arrives while scrolled away from the top. Verify in the browser with a seeded store and update `tests/web_posture.rs::fresh_machine_shows_the_empty_state` to check the empty-state text where it now lives.
- [x] 2.3 Implement the Files screen: directory-grouped list in path order with line count, latest version, and last change, a text filter carried as `q` in the address, its empty state, and selection opening the Reader at the latest version. Verify in the browser on a root with nested directories and a file without history.
- [x] 2.4 Implement the Reader screen: rendered content at the requested or current version, the scrollable version strip built from `history` with the selection marked, a visible past-version marking with return to latest, the `from` back link (`changes` or `files`, defaulting to Changes), and the not-found/error screen for missing files and bad versions. Verify in the browser by opening a file from each screen, selecting a past version, reloading the versioned address, and opening a missing path.
- [x] 2.5 Wire one `EventSource` for the page's lifetime: apply pushed `{changes, files}` to the visible screen (feed and groups on Changes, rows on Files, an "updated — view vN" banner on the Reader without replacing the open content), and confirm navigation between screens reconnects nothing. Verify with the viewer open while a change is recorded through the MCP `memory` tool, and with a file created and deleted directly in the memory root.

## 3. Integration checks

- [x] 3.1 Run `cargo test` for the full suite and confirm the read-only, path-escape, and markdown-escaping tests still pass unchanged.
- [x] 3.2 Walk the three screens on a seeded root at desktop and phone widths, confirm no screen offers a write action (`grep` the handlers for mutating calls), and confirm the reader still refuses a symlink that points outside the memory root.

## Workflow follow-up

- Update `README.md`'s web viewer section: the three screens and their addresses, filters, version browsing, and the `/?path=` compatibility note.
- Bump the crate version in `Cargo.toml` (minor: new user-facing feature).
- Archive the change after review.
