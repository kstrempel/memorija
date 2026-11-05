# Tasks

## 1. Shared structured change scan

- [x] 1.1 Extract the newest-first store scan from `inspect::changes` into a structured function (records with timestamp, path, seq, cause, added, removed) and rebuild the CLI `changes` output on top of it; verify all existing `changes` tests pass unchanged (`cargo test changes`).

## 2. Web subcommand and server skeleton

- [x] 2.1 Add the `web` subcommand to `src/cli.rs` (`--port` default 8081, `--host` default 127.0.0.1, `--no-open`) with clap tests mirroring the `serve` flag tests; verify `cargo test web` parses defaults and rejects nothing unexpectedly.
- [x] 2.2 Create `src/web/` with an axum app serving the embedded page at `/` and a JSON `GET /api/changes` backed by the shared scan; verify with a spawned-server test that `/api/changes` returns newest-first JSON records for a seeded store.

## 3. Content rendering and navigation

- [x] 3.1 Add `pulldown-cmark` and a `GET /api/file?path=…` handler that resolves the path via `paths::resolve`, reads the file, and returns rendered HTML plus path and latest-version metadata; verify a seeded-memory test renders headings, lists, and code, and that traversal and symlink-escape paths are refused with errors.
- [x] 3.2 Add a rendering test that a memory file containing `<script>` renders as escaped text with no live tag in the output.

## 4. Live updates

- [x] 4.1 Add a `GET /api/events` SSE endpoint backed by a ~1s background poll of the version store (mtime/size signal), emitting feed updates when the store moves; verify with a test that appends a record to a seeded store and observes an event within a few seconds.

## 5. Page and browser launch

- [x] 5.1 Write the embedded single page (inline CSS/JS, vanilla `EventSource` + fetch): feed on top, last changed file rendered below, click-to-file with `/?path=<rel>` URLs, empty-state message when the store is empty, clear not-found message for missing files; verify the served HTML exists and the page's JS references all three API endpoints (manual smoke check: run `memorija web` against a seeded root and click through).
- [x] 5.2 Implement browser auto-open (platform opener spawn, URL printed regardless, `--no-open` honored); verify the flag test and that a failed opener spawn does not stop the server.

## 6. Integration and posture

- [x] 6.1 Verify the end-to-end posture: default port 8081 runs alongside `serve --http` on 8080, non-loopback bind prints the warning, occupied port exits non-zero with a clear message, and a fresh machine (no memory root records) shows the empty state — via tests plus a manual run of both servers together.
- [x] 6.2 Update `README.md` Usage with the `web` subcommand, its flags, and the endpoints' behavior as user-facing output.

## Workflow follow-up

- On archive, bump the crate version in `Cargo.toml` (new feature → minor bump).
- On archive, fold any additional user-facing details surfaced during apply into `README.md` (already covered by task 6.2).
