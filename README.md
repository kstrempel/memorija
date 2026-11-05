# memorija

An MCP server that gives AI agents persistent memory: a plain directory of
markdown and text files under `~/.memorija/memories`, exposed through a single
`memory` tool.

memorija implements Anthropic's [memory tool](https://docs.claude.com/en/docs/agents-and-tools/tool-use/memory-tool)
interface (`memory_20250818`): one tool with eight file commands, designed so the
model manages its own memory without extra prompting logic on your side. The
memory is just files, so it stays readable, greppable, versionable, and safe to
back up or edit by hand.

## Features

- **Single `memory` tool** with the eight commands `view`, `create`,
  `str_replace`, `insert`, `delete`, `rename`, `history`, and `restore`, an
  additive extension of Anthropic's reference schema.
- **Files as storage** — the memory root is `~/.memorija/memories`, created and
  seeded with a `MEMORY.md` on first launch. An existing root is never
  overwritten.
- **Versioned edits** — every change to a memory file is recorded, with
  `history`, `view` at a past version, and `restore` to undo any change
  (see [Versioning](#versioning)).
- **Sandboxed paths** — every path is resolved inside the memory root;
  `..` traversal and symlinks that lead outside are refused.
- **Two transports** — stdio for locally launched clients, Streamable HTTP for
  shared or remote connections.
- **Tool-level errors** — recoverable failures come back as `isError` results
  so the calling model can self-correct.

## Install

Requires a current Rust toolchain (edition 2024):

```sh
cargo install --path .
```

## Usage

### stdio

Launch the server as a subprocess from an MCP client configuration:

```json
{
  "mcpServers": {
    "memorija": {
      "command": "/path/to/memorija"
    }
  }
}
```

A bare invocation with no arguments behaves as `memorija serve --stdio`.

### Streamable HTTP

```sh
memorija serve --http                       # listens on 127.0.0.1:8080
memorija serve --http --host 0.0.0.0 --port 9000
```

The MCP endpoint is `http://<host>:<port>/mcp`. The server has **no
authentication**; it warns on startup when binding to a non-loopback address.

### CLI

```
memorija [--version]
memorija serve [--stdio | --http] [--host <address>] [--port <number>]
memorija help [command]
```

- `--version` — print the tool name, version, and build date (e.g. `memorija 0.1.1 (built 2026-11-05)`), then exit without starting the server
- `--stdio` — serve over stdio (default; also the bare-invocation default)
- `--http` — serve over Streamable HTTP (default port 8080, default host `127.0.0.1`)
- `--host` / `--port` only apply to `--http`; mixing them with `--stdio` is a usage error
- `memorija help` shows the tool's purpose, version, build date, and usage
- Invalid invocations exit non-zero and never create the memory root

## The memory tool

All paths are relative to the memory root; the `/memories` prefix is accepted
and optional.

| Command       | Required fields | Effect |
|---------------|-----------------|--------|
| `view`        | `path`          | List a directory (recursively, with line counts) or read a file, optionally limited by `view_range`; with `version`, read a past version instead |
| `create`      | `path`, `file_text` | Write or overwrite a file (missing parent directories are created) |
| `str_replace` | `path`, `old_str`, `new_str` | Replace a unique occurrence (omit `new_str` to delete the matched text) |
| `insert`      | `path`, `insert_line`, `insert_text` | Insert text after a line number (0 prepends) |
| `delete`      | `path`          | Delete a file or directory |
| `rename`      | `path`, `new_path` | Move or rename |
| `history`     | `path`          | Show a file's version timeline |
| `restore`     | `path`, `version` | Revert a file to a past version by recording a new one |

## Versioning

Every change to a memory file (`create`, `str_replace`, `insert`, and
`restore` itself) appends an immutable version record capturing the path, the
full resulting content, the causing command, and a timestamp. Records live in
a store beside the memory root, `~/.memorija/versions/` — never inside the
visible memory tree, so listings and containment rules are unaffected. The
store is JSON Lines, one version per line per path, retained even after the
file is deleted or renamed.

- **`history`** returns a file's timeline, oldest first: version number,
  timestamp, cause (`create`, `str_replace`, `insert`, or `restore`), and a
  compact change summary against the previous version.
- **`view`** with `version: N` reads the file's content as of version N, with
  true line numbers; unknown versions name the available range.
- **`restore`** with `version: N` reverts the file to version N's content by
  recording a new version — history is append-only and never rewritten, so a
  restore is itself undoable.

Undoing a deletion of the file itself is not covered yet: `restore` requires
the file to exist.

## Development

```sh
cargo build
cargo test
```

Specs and change history live under [`openspec/`](openspec/). See
[AGENTS.md](AGENTS.md) for the OpenSpec workflow used in this repository.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
