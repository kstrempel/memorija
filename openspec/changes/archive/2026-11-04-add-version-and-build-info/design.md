# Design

## Context

`src/cli.rs` builds the CLI with clap's derive API: a `Cli` struct with an optional `serve` subcommand, an about line, and a `parse()` entry that exits on usage errors. `Cargo.toml` already carries `version = "0.1.0"`. There is no build script and no time-formatting dependency in the project.

## Goals / Non-Goals

**Goals:**
- `memorija --version` prints name, version, build date; exit code zero; no server start.
- Build date embedded at compile time, reproducibility-friendly.
- `memorija help` shows tool info (purpose, version, build date) above/below the usage.
- Existing invocations (`memorija`, `memorija serve ...`, `memorija help ...`) behave exactly as before.

**Non-Goals:**
- Full build metadata (git SHA, profile, target triple).
- A runtime `-V` on the `serve` subcommand itself (the top-level flag suffices; avoid ambiguity with `memorija --version`).
- Automatic version bumping or release tooling.

## Decisions

- **D1 — Build date via a dependency-free `build.rs`.** A small build script computes the current UTC date and emits `cargo:rustc-env=BUILD_DATE=YYYY-MM-DD`, read at compile time with `env!("BUILD_DATE")`. Converting days-since-epoch to Y-M-D is ~15 lines of `std` code, so no `time`/`chrono` build-dependency is added.
  - *Alternatives:* the `vergen`/`time` build-dependencies (rejected: extra dependency for one date); `SOURCE_DATE_EPOCH` respected when set (adopted for free: when the env var exists, derive the date from it so reproducible builds stay reproducible).
- **D2 — Version string assembled with `concat!`.** `#[command(version = VERSION_STR)]` on `Cli`, where `VERSION_STR = concat!(env!("CARGO_PKG_VERSION"), " (built ", env!("BUILD_DATE"), ")")`. Clap's `--version` action then prints `memorija <VERSION_STR>` and exits zero before any transport logic runs, so `main.rs` needs no changes.
  - *Alternatives:* handling `--version` manually in `parse()` (rejected: duplicates clap behavior); clapping's plain `#[command(version)]` (rejected: prints only the number, not the build date).
- **D3 — Tool info in help via `after_help`.** A static `after_help` line (`memorija 0.1.0 (built YYYY-MM-DD)`) on the root command makes `memorija help` show purpose + usage + version/build date. The existing `about` string stays as the purpose. The `serve` subcommand keeps its current help text.
  - *Alternatives:* a longer `long_about` (rejected: duplicates the tool description the MCP layer already carries); `before_help` (placement is arbitrary; after_help keeps usage at top).
- **D4 — Flag placement: top-level only.** `--version` is defined on `Cli`, not on `ServeArgs`, matching clap conventions and keeping `ServeArgs::validate` untouched.

## Risks / Trade-offs

- [Build date changes on every rebuild, making builds non-reproducible] → Honor `SOURCE_DATE_EPOCH` in `build.rs`; cargo also caches build scripts, so ordinary incremental rebuilds do not churn it.
- [`env!("BUILD_DATE")` fails to compile if `build.rs` never runs] → Not a realistic path with cargo; the build script emits the variable unconditionally.
- [Tests assert on an output containing a date] → Tests assert on version format and the presence of a `YYYY-MM-DD` substring, not a fixed date.

## Migration Plan

Purely additive CLI surface; no data, protocol, or config changes. Rollback is reverting the commit.
