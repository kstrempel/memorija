# Proposal

## Why

The binary currently offers no way to see which version is installed: there is no `--version` flag, and the `help` output only carries a one-line about string. Users (and bug reports) cannot tell which build of `memorija` they are running.

## What Changes

- Add a `--version` flag to the binary that prints the tool name, version `0.1.0` (taken from the crate version), and the build date, then exits with code zero. It works on the bare invocation and alongside `serve`.
- Record the build date at compile time (UTC date the binary was built, e.g. `2026-11-04`) and include it in the `--version` output.
- Enrich the `memorija help` output so it presents info about the tool, not just usage: the tool's purpose (the existing about text), the version and build date, and the usage listing `serve`.
- Enrich `memorija help serve` context so the serve usage stays consistent with the added flag surface.
- No breaking changes: the bare invocation still starts the MCP server over stdio, and all existing flags behave unchanged.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `cli`: add a `--version` flag requirement (name, version, build date; exit code zero) and a requirement that the help output presents tool info (purpose, version, build date) alongside usage.

## Impact

- `src/cli.rs`: new `--version` flag wiring (clap `version` action), help/about text, build-date constant exposure.
- `Cargo.toml` or a small `build.rs`: a compile-time `BUILD_DATE` environment variable captured via `env!`.
- `src/main.rs`: no change expected if clap handles `--version` and help via the `Cli` command definition.
- Tests in `src/cli.rs`: new cases for `--version` and enriched help output.
