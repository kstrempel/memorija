# Proposal

## Why

Every display path and every recorded version path is built with
`Path::display()`/`to_string_lossy()`, so a Windows build spells nested entries
with backslashes: `memorija list` prints `projects\alpha\notes.txt`, `view`
does the same, the web Files index returns `projects\\alpha\\notes.txt`, and the
version store files a nested path under `projects%5Calpha%5Cnotes.txt.jsonl`
with `path` records holding backslashes. That breaks the four Windows test
failures on `add-github-release-workflow`, and it is more than a formatting
problem: the version store's keys become machine- and platform-specific, the web
Reader's own `/file?path=…` addresses no longer round-trip against the store,
and a memory tree synced between macOS and Windows records the same file under
two different keys, so its history splits.

## What Changes

- Memory paths become one canonical form everywhere: relative to the memory
  root, segments joined with `/`, on every platform. Both producers of a
  relative path — the write path's key and the recursive listing walk — use the
  canonical form, so `list`, `view`, `history`, `changes`, the MCP tool results
  and the web APIs all name `projects/alpha/notes.txt`.
- The version store keeps working for stores written by the current Windows
  build: decoding a store file name normalises a legacy `\` to `/`, the change
  scan normalises the `path` stored in old records, and a store file that only
  exists under the legacy spelling is still read from and appended to, so an
  existing Windows history is neither lost nor duplicated. No file is rewritten
  and no store migration runs.
- The web UI's index, changes feed and Reader addresses therefore agree with the
  store and with a path a person types, on both platforms.
- Out of scope: rewriting legacy store files, migrating `path` fields in place,
  accepting `\` from CLI or MCP callers as an alias, and any change to the
  memory root layout.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `memory-versions`: recorded and decoded paths have a canonical `/` form on
  every platform, and a store written with `\` remains readable and appendable.
- `cli-inspection`: listings and the change log name paths with `/`, so the
  same tree prints identically on macOS, Linux and Windows.
- `web-ui`: the Files index, change rows and Reader addresses use `/` paths, so
  a path the page shows resolves back to the file and its history.

## Impact

- `src/paths.rs` — the canonical-relative-path helper (`key_string`) both
  producers use.
- `src/versions.rs` — `decode_key` normalises separators; the store-file lookup
  falls back to the legacy `\`-spelled name when the canonical file is absent.
- `src/inspect.rs` — the change scan normalises recorded paths on output.
- Tests: `src/ops.rs`, `src/versions.rs`, `src/inspect.rs`, `src/web/mod.rs`
  cover the canonical form and legacy readability; the four now-failing Windows
  tests pass unchanged because they already assert `/`.
- `README.md` (Development note) and `Cargo.toml` (patch bump) on archive, per
  the project's archive rules.
