# Tasks

## 1. Cross-platform home resolution

- [x] 1.1 In `src/root.rs::resolve()`, fall back to `USERPROFILE` when `HOME` is unset, and word the error as `neither HOME nor USERPROFILE is set`; verify with `cargo test root::` and by running `env -u HOME USERPROFILE=/tmp/fake-home cargo run -- list` (macOS/Linux) that the root resolves under the fallback and that unsetting both exits with the new message.
- [x] 1.2 Add a unit test asserting `HOME` wins when both variables are set, keeping the existing temp-root helpers; verify with `cargo test root::`.
- [x] 1.3 Update `README.md`'s environment notes (memory root paragraph) to state that the home directory comes from `HOME`, or `USERPROFILE` on Windows; verify the wording matches the behaviour exercised in 1.1.

## 2. Windows-testable test suite

- [x] 2.1 Add `#[cfg(unix)]` to the three symlink tests (`escaping_symlink_is_refused_and_outside_file_untouched`, `symlink_within_root_is_followed` in `src/ops.rs`; the symlink containment test in `src/web/mod.rs`) and to any helper only they use; verify on macOS with `cargo test` that the suite still runs them (`cargo test -- --list | grep symlink`).
- [x] 2.2 Audit `src/**/tests` and `tests/` for other Unix-only construction (path separators, permissions, rename-over-open-file), fixing assumptions without weakening product behaviour; verify with `cargo test --locked` locally and record anything that can only be confirmed on Windows in the change notes.

Notes: the audit found no Unix-only construction beyond symlinks (no permission bits, no shell
commands, `rename` onto an existing target is refused before `fs::rename`, and integration tests
pass `HOME` explicitly). Two extra web tests touch symlinks: `api_files_never_leaks_content_or_paths_outside_the_root`
is Unix-gated whole, and `api_file_refuses_paths_that_escape_the_root` keeps its traversal cases on
every platform while the symlink case is Unix-only. Windows-only verification that remains for CI:
the full suite on `windows-latest`, and the Windows `USERPROFILE` resolution exercised by the
released binary.

## 3. CI workflow

- [x] 3.1 Create `.github/workflows/ci.yml` triggered by `push` to `main`, `pull_request`, and `workflow_dispatch`, with `permissions: contents: read` and a `ci-${{ github.ref }}` concurrency group that cancels superseded runs; verify with `python3 -c "import yaml,sys;yaml.safe_load(open('.github/workflows/ci.yml'))"` (or `actionlint`) that the file parses and the triggers and permissions are as described.
- [x] 3.2 Add the `fmt` and `clippy` jobs (`cargo fmt --all -- --check`; `cargo clippy --all-targets --locked -- -D warnings`) using `dtolnay/rust-toolchain@stable` and `Swatinem/rust-cache@v2`; verify locally that both commands pass on a clean checkout (`cargo fmt --all -- --check && cargo clippy --all-targets --locked -- -D warnings`).
- [x] 3.3 Add the `test` matrix over `ubuntu-22.04`, `macos-latest`, `windows-latest` running `cargo test --locked`; verify the matrix keys and `runs-on` values expand to one job per platform with `actionlint`, and that the local `cargo test --locked` run passes.
- [x] 3.4 Add the `build` matrix for `x86_64-unknown-linux-gnu` (`ubuntu-22.04`), `aarch64-apple-darwin` and `x86_64-apple-darwin` (`macos-latest`), `x86_64-pc-windows-msvc` (`windows-latest`), running `cargo build --release --locked --target <target>` with the cross target installed via the toolchain action's `targets:` input; verify the target triples match the release workflow list exactly and that `actionlint` reports no unknown keys.
- [ ] 3.5 Push the branch and confirm on GitHub that all checks appear with per-platform names, that the Windows job runs the test suite, and that a deliberately introduced formatting error turns the run red; revert the error afterwards.

## 4. Release workflow

- [x] 4.1 Create `.github/workflows/release.yml` triggered by tags matching `v*` and `workflow_dispatch`, with `permissions: contents: write`, a `release-${{ github.ref }}` concurrency group that does not cancel in progress, and a first step that compares `${GITHUB_REF_NAME#v}` with `cargo metadata --no-deps --format-version 1`'s version, failing with a clear message on mismatch; verify with `actionlint` and by reading the step order.
- [x] 4.2 Add the build job for the four targets (same matrix as 3.4) with `cargo build --release --locked --target <target>`, then package: `tar.gz` containing the binary, `README.md` and `LICENSE` at the archive root for the three Unix targets, `Compress-Archive` zip with the same layout for Windows, named `memorija-<version>-<target>.<ext>`; verify by running the equivalent packaging commands locally on a `cargo build --release` output and extracting the archive to confirm the files sit at the top level.
- [x] 4.3 Upload each archive with `actions/upload-artifact`, then add a release job (`needs: build`, `ubuntu-latest`) that downloads all artifacts into one directory, writes `SHA256SUMS` with `sha256sum`, and runs `gh release view "$TAG" || gh release create "$TAG" --title … --notes …` followed by `gh release upload "$TAG" … --clobber` using `GH_TOKEN: ${{ github.token }}`; verify with `actionlint` and by re-reading the job to confirm the upload names every archive.
- [ ] 4.4 Write release notes that name the version, list the four supported targets with their archive filenames, and state that the archives are unsigned; verify the rendered body of a dry-run release (`gh release create --draft` on a scratch tag) reads correctly, then delete the draft.
- [ ] 4.5 Rehearse end to end on a scratch tag (for example `v<version>-rc1` after temporarily relaxing the version check, or a real `v<version>` once `Cargo.toml` is bumped), confirming four archives plus `SHA256SUMS` are attached, that each Unix archive runs `--version`, and that the Windows archive extracts with Explorer's built-in zip support; record the result in the change and remove any scratch tag.

## 5. Documentation

- [x] 5.1 Rewrite `README.md`'s Install section: keep the source build, add a table of the four release archives with the one-line download-and-extract command per platform, and state the Linux glibc floor and that archives are unsigned; verify each documented command against a downloaded artifact (or the rehearsal archive from 4.5).
- [x] 5.2 Add a Development note to `README.md` covering what CI checks, that `main` and pull requests are verified on all four targets, and how a release is made (bump `Cargo.toml`, tag `v<version>`, push the tag); verify the described steps against `.github/workflows/release.yml`.

## 6. Integration pass

- [ ] 6.1 Run the full local suite (`cargo fmt --all -- --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`) and confirm the pushed branch has every CI job green including Windows tests and all four builds.

## Workflow follow-up

- Bump `Cargo.toml` (minor for the new `ci-release` capability) and fold this change's user-facing notes into the README sections, then archive the change per `AGENTS.md`.
- Verify the archived `openspec/specs/ci-release/spec.md` exists with its Purpose and that `memory-root` carries the home-resolution requirement.
