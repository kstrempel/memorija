# Tasks

## 1. Shared inspection layer

- [x] 1.1 Add a `src/inspect.rs` module skeleton: resolve the memory root via `root::resolve()` without calling `root::ensure`, return a distinguishable "no memory root" condition, and verify unit tests for the fresh-machine path pass (`cargo test`)
- [x] 1.2 Extract the shared line-diff counting from `ops::change_summary` into a helper usable by both `ops.rs` and the new module, and verify existing `ops` tests still pass unchanged (`cargo test`)
- [x] 1.3 Implement a percent-encoded store-name decoder (inverse of `versions::encode_key`) with round-trip tests including the `a%2Fb.txt` case, verified by `cargo test`

## 2. `list` subcommand

- [x] 2.1 Add the `List` variant to `cli.rs` and dispatch it in `main.rs` to `inspect.rs` (no server start), verifying `memorija list --help` prints the command and `cargo test` passes for the new parse tests
- [x] 2.2 Implement the recursive listing with line counts over `paths::resolve`/directory walking, and add tests: populated root lists nested files with counts; fresh machine prints `no memories yet`, exits zero, and creates neither root nor store (`cargo test`)

## 3. `show` subcommand with rendering

- [x] 3.1 Add `termimad` to `Cargo.toml` and verify `cargo build` succeeds with the dependency tree
- [x] 3.2 Add the `Show` variant with `<file>`, optional version positional, and `--raw`/`--render` flags, with clap parse tests for the flags and conflict behavior (`cargo test`)
- [x] 3.3 Implement live-file and version-selected printing (raw path) reusing `paths::resolve` and `versions::read`, with tests for live content, past version, unknown version error naming the range, and missing-file error (`cargo test`)
- [x] 3.4 Wire the TTY gate (`std::io::IsTerminal`): render via termimad when stdout is a terminal and `--render` is set or flags are default; raw when piped or `--raw`; verify by piping output through a file and asserting no ANSI codes, and rendering to a pty manually

## 4. `versions` subcommand

- [x] 4.1 Add the `Versions` variant to `cli.rs`/`main.rs` with parse tests (`cargo test`)
- [x] 4.2 Implement the version/date/cause table from `versions::read` (oldest first) with tests for a three-record file and a no-records error naming the path (`cargo test`)

## 5. `changes` subcommand

- [x] 5.1 Implement the timespan parser per design D3 (`<n>h|<n>d|<n>w|<n>m`, `YYYY-MM-DD`, `START..END`, `all`, default `7d`) producing a cutoff pair of timestamp strings, with table-driven tests including invalid values, reversed range, and the 30-day `m` semantics (`cargo test`)
- [x] 5.2 Add the `Changes` variant to `cli.rs`/`main.rs`, rejecting unparseable timespans as usage errors (parse test, `cargo test`)
- [x] 5.3 Implement the global log: scan `versions::store_dir` JSONL files, filter by cutoff, sort newest-first with path/seq tiebreak, print date/time/path/cause/`+N -M` lines; tests cover multi-file ordering, empty-result zero exit, and summary counts (`cargo test`)

## 6. Integration verification

- [x] 6.1 End-to-end test through the real layout (`~/.memorija`-style temp home): create and edit files via `ops::run`, then assert `list`, `show`, `show <file> <version>`, `versions`, and `changes` outputs against the spec scenarios (`cargo test`)
- [x] 6.2 Verify the bare invocation still serves stdio and the MCP server path compiles without `termimad` in scope: existing `cli.rs` tests pass and `cargo build` warns nothing new (`cargo test`, `cargo build`)
- [x] 6.3 Run `openspec validate add-inspection-cli-commands --strict` and resolve any reported issues

## 7. Follow-up: version numbers in `list`

- [x] 7.1 Extend the `list` output with each file's available version range (`v1..vN`, `v1` for a single version, `-` when never recorded) via a store scan per design D4's shape, with tests for recorded, once-recorded, and unrecorded files, and the e2e assertions updated (`cargo test`)
