# Tasks

## 1. Build date plumbing

- [x] 1.1 Add a dependency-free `build.rs` that emits `cargo:rustc-env=BUILD_DATE=YYYY-MM-DD` (UTC, honoring `SOURCE_DATE_EPOCH` when set) and verify `cargo build` succeeds with `env!("BUILD_DATE")` resolvable from `src/cli.rs`

## 2. Version flag

- [x] 2.1 In `src/cli.rs`, add the version attribute to `Cli` assembling name, `CARGO_PKG_VERSION` (`0.1.0`), and `BUILD_DATE` (design D2); verify `cargo run -- --version` prints `memorija 0.1.0 (built <date>)` and exits zero without starting the server
- [x] 2.2 Add CLI tests: `--version` produces a `DisplayVersion` error containing `0.1.0` and a `YYYY-MM-DD` substring, and the bare invocation still maps to stdio (`cargo test` passes)

## 3. Help output info

- [x] 3.1 In `src/cli.rs`, add an `after_help` line to the root command with the version and build date (design D3); verify `cargo run -- help` shows the purpose, the usage listing `serve`, and the version/build-date line
- [x] 3.2 Add a CLI test that `memorija help` output contains the version and a `YYYY-MM-DD` substring, and that `memorija help serve` still lists `--stdio`, `--http`, `--host`, `--port` (`cargo test` passes)

## 4. Integration verification

- [x] 4.1 Run `cargo build && cargo test` and confirm the full suite passes with no regressions in existing CLI tests
- [x] 4.2 Run `cargo run -- --version`, `cargo run -- help`, and `cargo run` (bare, verify the MCP server still starts over stdio and exits on EOF) and confirm the observable behaviors from the spec delta
