# Proposal

## Why

The `memorija` binary is currently MCP-only: the only way to look at what the memory holds is through a running MCP session. A human on the machine has no way to ask "what memories do I have?", "what does this file say?", or "what changed lately?" without opening an MCP client. Four small read-only subcommands make the memory tree and its version store inspectable directly from the shell.

## What Changes

- New `memorija list` subcommand: recursive listing of every memory file with line counts and each file's available version range (`v1..vN`, `-` when the file was never changed through the tool), read from the version store without touching it. When the memory root does not exist yet, prints `no memories yet` and exits with code zero — it does not create or seed the root.
- New `memorija show <file> [version]` subcommand: prints one memory file (or the recorded content of one of its versions) to the console. On a terminal the file is rendered as styled markdown (via `termimad`); when stdout is piped the raw content is printed. `--render` forces rendering, `--raw` forces raw output.
- New `memorija versions <file>` subcommand: table of that file's version records — version number, UTC timestamp, and cause.
- New `memorija changes [timespan]` subcommand: a global change log across all files, newest first, each line showing date, time, path, cause, and a `+N -M` line-count summary. The timespan accepts `<n>h`/`<n>d`/`<n>w`/`<n>m` duration suffixes, an absolute `YYYY-MM-DD` date (or `START..END` range), or `all`; the default is `7d`.
- All four commands are read-only: they append no version records and never create the memory root or the version store.
- New dependency: `termimad` (with its transitive tree), used only by `show`'s render path.

## Capabilities

### New Capabilities

- `cli-inspection`: The binary's human-facing inspection commands — `list`, `show`, `versions`, and `changes` — their read-only contract, their output shapes, the `show` rendering/raw and TTY behavior, the `changes` timespan grammar, and their behavior on a machine with no memory yet.

### Modified Capabilities

None. The existing `cli` spec's requirements (serve surface, bare-invocation default, help, version) are unchanged by these additions; the inspection commands are new behavior with their own capability.

## Impact

- `src/cli.rs`: four new `Command` variants with their arguments and flags; `main.rs` dispatches them instead of always serving.
- New CLI formatting module over the existing primitives (`paths::resolve`, `versions::read`, `versions::store_dir`) rather than reusing the model-shaped `ops::run_in` output; `ops::change_summary`-style line counting is shared logic for `changes`.
- `Cargo.toml`: adds `termimad` (render path of `show` only; the MCP server path never touches it).
- The version store (`~/.memorija/versions`) gains no new files or formats; `changes` reads every existing per-path JSONL.
- Existing MCP clients invoking the bare binary are unaffected (bare invocation still means `serve --stdio`).
