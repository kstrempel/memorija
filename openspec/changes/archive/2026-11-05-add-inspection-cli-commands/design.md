# Design

## Context

The binary today is serve-only: `cli.rs` has a single `Serve` subcommand and `main.rs` always starts the MCP server. The inspection machinery already exists underneath — `paths::resolve` for safe path handling, `versions::read`/`store_dir` for the record store, and the model-shaped outputs in `ops.rs` (see proposal.md for motivation). The version store records timestamps as `YYYY-MM-DDTHH:MM:SSZ` strings, which sort lexicographically in chronological order.

## Goals / Non-Goals

**Goals:**

- Four read-only inspection commands that share one thin human-facing formatting layer over the existing primitives
- Contain the `termimad` dependency to `show`'s render path only
- Keep the whole feature free of date-parsing and timezone logic

**Non-Goals:**

- Mutating CLI commands (`create`, `edit`, `delete`, `restore` from the shell)
- Calendar-based presets (`today`, `this week`) — they require local-timezone handling the codebase deliberately avoids
- Rendering for `list`, `versions`, or `changes` — plain text is the right shape for them
- Any change to the version store format or the MCP tool surface

## Decisions

### D1: Dedicated human-facing layer, not `ops::run_in` reuse

The CLI commands get their own formatting functions built on `paths::resolve` and `versions::read`, not the model-shaped `ops::run_in` output. `ops::view` prints tab-separated line numbers for model edit-referencing; `ops::history` prints diff excerpts a human scanning a log doesn't want. Forcing both audiences through one output shape makes both worse. The cost is some duplication of the walk/listing logic, mitigated by extracting shared pieces (directory walking, line counting, change-summary counting) where cheap.

*Alternative considered*: parse CLI args into `MemoryArgs` and call `ops::run_in`. Rejected: wrong output shape for humans; post-processing model output back into human form is more work than formatting directly.

### D2: `termimad` for markdown rendering, TTY-gated

`show` renders via `termimad` when stdout is a terminal; piped output is raw. TTY detection uses stdlib `std::io::IsTerminal` (no dep). Explicit `--raw`/`--render` flags override detection, `--raw` winning if both are passed (clap `conflicts_with` is unnecessary — last-wins is fine, but documenting `--raw` as the override keeps help text simple). Version-selected content renders the same way as live content.

*Alternative considered*: zero-dep raw passthrough (rejected: the user chose rendered output); `mdcat`/`glow` shelling (rejected: external binary dependency, glow is Go).

### D3: Timespans are compiled to one cutoff string, no date library

The timespan grammar (`<n>h|<n>d|<n>w|<n>m`, `YYYY-MM-DD`, `START..END`, `all`, default `7d`) resolves to a `[cutoff_start, cutoff_end)` pair of `YYYY-MM-DDTHH:MM:SSZ` strings. Duration windows compute `now` via the existing `versions::now_utc`-style math; absolute dates are already in the stored format, so filtering is a plain string comparison against `record.recorded_at`. Month suffix `m` is a rolling 30-day window, not calendar months. Format errors (bad suffix, bad date, reversed range) are clap-level usage errors.

*Alternative considered*: `jiff`/`chrono` for calendar semantics. Rejected: pulls timezone handling into a feature that doesn't need it; can be added later without format changes.

### D4: `changes` scans the whole store directory

`changes` iterates every `*.jsonl` in `versions::store_dir`, reads all records, filters by cutoff, and sorts by `recorded_at` (descending; stable tiebreak on path then seq for determinism). Store sizes are human-scale (thousands of records), so a full scan per invocation is fine. The `+N -M` summary reuses the line-diff counting logic from `ops::change_summary` (extracted to be shared). Decode percent-encoded store file names back to display paths via the existing encode scheme's inverse.

### D5: Dispatch shape in `main.rs`

`cli.rs` gains `List`, `Show`, `Versions`, `Changes` variants; `main.rs` matches: serve variants go to `serve::run_server`, inspection variants go to the new module, which resolves the root via `root::resolve()` and returns process exit codes. Inspection commands check for the memory root's existence *before* any creation attempt and never call `root::ensure` — the "no memories yet" path for `list`; the other three report a missing root the same way.

### D6: Error style matches existing conventions

Errors are `Error: <human message>` strings to stderr with non-zero exit, mirroring `ops.rs`'s message style (path-naming, version-range naming). A file with no records under `versions` errors like `ops::history` does.

## Risks / Trade-offs

- [termimad dependency weight] → confined to `show`'s render path; compile-time cost only, no runtime effect on the MCP server. If it ever needs to go, `--raw` behavior remains as the fallback shape.
- [Duplicate listing/diff logic between `ops.rs` and the new layer] → extract shared helpers (walk, line-diff counts) rather than copy-paste; keep the model outputs untouched.
- [`m` = 30 days may surprise] → document it in `--help`; calendar months would require the timezone machinery D3 avoids.
- [Global sort of records with identical second-resolution timestamps] → stable tiebreak on path, then seq, so output order is deterministic.

## Migration Plan

None needed: purely additive subcommands; bare invocation and existing client configs keep serving stdio. Rollback is reverting the change.

## Open Questions

None — the explore session settled scope, output shapes, timespan grammar, and dependency choice.
