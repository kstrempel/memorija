# Tasks

## 1. Version store

- [x] 1.1 Create a version store module implementing records `{ path, seq, content, cause, recorded_at }` with append and read-by-path operations, laid out so each record maps to a future database row, and located outside the memories root; verify with unit tests covering append, read, per-path sequencing, and record immutability
- [x] 1.2 Thread the store path (root's parent) through `ops::run` and confirm `paths::resolve` never resolves into the store; verify with a test that a path attempting to reach the store is refused or treated as nonexistent

## 2. Record mutations

- [x] 2.1 Append a record on `create`, `str_replace` (with and without `new_str`), and `insert`, capturing resulting content and the command as cause; verify with tests that each mutating command produces exactly one record with the right cause and content
- [x] 2.2 Verify the six original commands are unchanged when no `version` is passed; verify by running the existing ops test suite unchanged

## 3. History command

- [x] 3.1 Implement `history` returning the per-path timeline (version number, timestamp, cause, compact diff summary) ordered oldest to newest, with error results for paths without records; verify with unit tests covering a three-record timeline, per-path scoping after `rename`, and the no-records error
- [x] 3.2 Add `history` to the `Command` enum, argument parsing, and tool schema/description text; verify the enum test lists all eight commands and the schema test passes

## 4. View and restore past versions

- [x] 4.1 Honor an optional `version` on `view`, returning the recorded content with true line numbers and erroring with the available version range for unknown versions or directories; verify with unit tests for old-version read, out-of-range version, and directory rejection
- [x] 4.2 Implement `restore` as a write through the create path appending a `restore`-cause record, requiring `version` and an existing live file; verify with unit tests for content equal to the target version, the appended timeline entry, restore-then-restore-back, missing `version`, unknown version, and restore of a deleted file being refused
- [x] 4.3 Add `restore` to the `Command` enum and per-command validation (missing-`version` error naming the field); verify with argument-validation tests mirroring the existing per-command cases

## 5. Tool description and integration

- [x] 5.1 Update the tool description to document versioning, `history`, `view` with `version`, and `restore`; verify the description tests assert the versioning guidance
- [x] 5.2 Add an end-to-end test through `serve`/`ops::run`: create, edit, insert, history, view at version, restore, history again, and a root listing that shows no version-store entries; verify the whole sequence in one test
- [x] 5.3 Run `cargo test` and `cargo build` and confirm everything passes clean
