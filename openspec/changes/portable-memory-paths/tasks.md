# Tasks

## 1. Canonical path producers

- [x] 1.1 Confirm the canonical-relative-path helper (`paths::key_string`, which joins a path's `Normal` components with `/`) is used by both `rel_key` and `walk` (which keeps its trailing `/` on directory entries) and add the missing integration test that a nested temp tree yields `projects/alpha/notes.txt` from both; verify with `cargo test ops::tests` and the existing view/list tests.
- [x] 1.2 Confirm the change log and version tables still key recorded paths through `rel_key` (`src/inspect.rs`), so `history`, `versions` and `restore` read the canonical file; verify `cargo test inspect::` passes.

## 2. Version-store compatibility

- [x] 2.1 Normalise separators in `versions::decode_key` so a legacy `%5C` name decodes to `a/b.txt`; verify with a unit test beside the existing `decode_key` tests and `cargo test versions::decode_key` passes.
- [x] 2.2 Make the store-file lookup fall back to the legacy `\`-spelled name when the canonical file is absent, for both read and append; verify with a unit test that seeds `a%5Cb.txt.jsonl` with two records, reads `a/b.txt`, appends a third mutation and finds one three-record history in sequence order, and that a canonical file wins when both spellings exist.

## 3. Recorded output surfaces

- [x] 3.1 Normalise `record.path` when `inspect::scan_changes` builds a `ChangeRecord`; verify with a unit test that a seeded legacy record appears in `changes` and `scan_changes` with `/` segments, and that `cargo test inspect::tests::changes` passes.
- [x] 3.2 Add a web test that a legacy `%5C` store record still joins the Files index with its latest version and last-changed time; verify `cargo test web::tests::api_files` passes on macOS and that the existing `api_files_reports_metadata_for_recorded_and_unrecorded_files` expectation of `projects/alpha/notes.txt` is unchanged.

## 4. Whole-suite verification

- [x] 4.1 Run `cargo test` and confirm the suite is green locally with no ignored tests beyond those already ignored.
- [x] 4.2 Push the branch and confirm the Windows job on `add-github-release-workflow` passes the four previously failing tests (`inspect::tests::list_names_nested_files_with_line_counts`, `inspect::tests::end_to_end_inspection_through_the_real_layout`, `ops::tests::view_directory_lists_tree_with_line_counts`, `web::tests::api_files_reports_metadata_for_recorded_and_unrecorded_files`) and the whole job; the `key_string_keeps_a_backslash_in_a_file_name` test is Unix-only, because Windows parses that backslash as a separator.

## Workflow follow-up

- Archive the change once the tree is reviewed and CI is green (`openspec archive portable-memory-paths`).
- On archive, add a brief Development note to `README.md` that memory paths are canonical `/` paths and that pre-fix `\`-keyed store files are read through a fallback, and bump the crate patch version in `Cargo.toml` (`0.8.0` → `0.8.1`).
