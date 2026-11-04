# memorija

An MCP server that gives AI agents persistent memory: a plain directory of
markdown and text files under `~/.memorija/memories`, exposed through a single
`memory` tool.

memorija implements Anthropic's [memory tool](https://docs.claude.com/en/docs/agents-and-tools/tool-use/memory-tool)
interface (`memory_20250818`): one tool with six file commands, designed so the
model manages its own memory without extra prompting logic on your side. The
memory is just files, so it stays readable, greppable, versionable, and safe to
back up or edit by hand.

## Features

- **Single `memory` tool** with the six commands `view`, `create`,
  `str_replace`, `insert`, `delete`, and `rename`, matching Anthropic's
  reference schema.
- **Files as storage** — the memory root is `~/.memorija/memories`, created and
  seeded with a `MEMORY.md` on first launch. An existing root is never
  overwritten.
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
memorija serve [--stdio | --http] [--host <address>] [--port <number>]
memorija help [command]
```

- `--stdio` — serve over stdio (default; also the bare-invocation default)
- `--http` — serve over Streamable HTTP (default port 8080, default host `127.0.0.1`)
- `--host` / `--port` only apply to `--http`; mixing them with `--stdio` is a usage error
- Invalid invocations exit non-zero and never create the memory root

## The memory tool

All paths are relative to the memory root; the `/memories` prefix is accepted
and optional.

| Command       | Required fields | Effect |
|---------------|-----------------|--------|
| `view`        | `path`          | List a directory (recursively, with line counts) or read a file, optionally limited by `view_range` |
| `create`      | `path`, `file_text` | Create a new file (fails if it exists) |
| `str_replace` | `path`, `old_str`, `new_str` | Replace a unique occurrence |
| `insert`      | `path`, `insert_line`, `insert_text` | Insert text after a line number (0 prepends) |
| `delete`      | `path`          | Delete a file or directory |
| `rename`      | `path`, `new_path` | Move or rename |

## Development

```sh
cargo build
cargo test
```

Specs and change history live under [`openspec/`](openspec/). See
[AGENTS.md](AGENTS.md) for the OpenSpec workflow used in this repository.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
