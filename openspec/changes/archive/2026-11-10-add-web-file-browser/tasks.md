# Tasks

## 1. File listing endpoint

- [x] 1.1 Add `GET /api/files` to `src/web/mod.rs`: walk the memory root (skipping the version-store directory and hidden entries), collect relative paths sorted by path, and return `{ "files": [...] }`; verify with a test that a seeded root with recorded and unrecorded files returns all paths sorted, and a nonexistent root returns an empty list (`cargo test web`)
- [x] 1.2 Verify listing safety with a test: a symlink inside the root pointing outside is refused by `/api/file` as before and the listing never leaks outside content; `cargo test web` passes

## 2. Live updates for the index

- [x] 2.1 Extend the poller's movement signal to also cover root shape (entry count + newest mtime at top level) so filesystem-only changes trigger a push, and have the poller include the fresh file list in its watch payload; verify with a test that creating a file without a store record still produces an `/api/events` update containing it (`cargo test web`)

## 3. Page UI

- [x] 3.1 Add an "All files" section to `src/web/page.html` (rows styled like the feed, sorted by path), wired to the existing `select()` navigation and selected-state highlighting; verify by serving `memorija web` on a seeded root and clicking through to a rendered file at its `/?path=…` URL
- [x] 3.2 Render the index from the live stream as well as initial fetch: a file created or removed while the page is open appears or disappears without reload; verify with `cargo test web` coverage of the payload shape and manually against a running viewer
- [x] 3.3 Handle the empty case: a fresh machine shows an empty-state message for the index and no error; verify in the browser on an empty root

## 4. Integration checks

- [x] 4.1 Run the full suite and confirm the existing web-ui scenarios still hold (`cargo test`, including feed ordering, path escape refusal, XSS escaping, SSE push)
- [x] 4.2 Confirm the viewer remains write-free: `grep` the new handler for any mutating call and re-run the read-only posture tests (`cargo test web`)

## Workflow follow-up

- Archive the change after review; on archive, update `README.md` (web viewer section: mention the all-files index) and bump the crate version in `Cargo.toml` (minor bump — new user-facing feature).
