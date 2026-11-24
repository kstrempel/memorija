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
  overwritten. The home directory comes from `$HOME`, or `%USERPROFILE%` on
  Windows where `HOME` is normally unset.
- **Versioned edits** — every change to a memory file is recorded, with
  `history`, `view` at a past version, and `restore` to undo any change
  (see [Versioning](#versioning)).
- **Sandboxed paths** — every path is resolved inside the memory root;
  `..` traversal and symlinks that lead outside are refused.
- **Web viewer** — `memorija web` opens a local, read-only viewer with three
  screens: a live Changes feed, a Files index with per-file metadata, and a
  Reader that can show any recorded version. It renders markdown, including
  Mermaid diagrams, and updates itself.
- **Two transports** — stdio for locally launched clients, Streamable HTTP for
  shared or remote connections.
- **Tool-level errors** — recoverable failures come back as `isError` results
  so the calling model can self-correct.

## Install

Prebuilt binaries for macOS, Linux and Windows are attached to every
[release](https://github.com/kstrempel/memorija/releases). Download the archive
for your platform, extract it, and put `memorija` on your `PATH`.

| Platform | Archive |
| --- | --- |
| macOS, Apple silicon | `memorija-<version>-aarch64-apple-darwin.tar.gz` |
| macOS, Intel | `memorija-<version>-x86_64-apple-darwin.tar.gz` |
| Linux, x86_64 | `memorija-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| Windows, x86_64 | `memorija-<version>-x86_64-pc-windows-msvc.zip` |

```sh
# macOS / Linux: <version> is the release's version and <target> is from the table
curl -LO https://github.com/kstrempel/memorija/releases/latest/download/memorija-<version>-<target>.tar.gz
tar -xzf memorija-<version>-<target>.tar.gz
sudo mv memorija /usr/local/bin/
```

```powershell
# Windows
Expand-Archive memorija-<version>-x86_64-pc-windows-msvc.zip -DestinationPath .
```

Every release also publishes `SHA256SUMS`; check a download with
`sha256sum -c SHA256SUMS --ignore-missing` (Linux) or
`shasum -a 256 -c SHA256SUMS` (macOS).

The archives are unsigned — no code signing, notarization, or Authenticode. The
Linux binary needs glibc 2.35 or newer (it is built on Ubuntu 22.04); on macOS,
if Gatekeeper refuses the binary, clear the download's quarantine flag with
`xattr -d com.apple.quarantine ./memorija`.

From source, with a current Rust toolchain (edition 2024):

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
memorija web [--host <address>] [--port <number>] [--no-open]
memorija list
memorija show <file> [version] [--raw | --render]
memorija versions <file>
memorija changes [timespan]
memorija help [command]
```

- `--version` — print the tool name, version, and build date (e.g. `memorija 0.1.1 (built 2026-11-05)`), then exit without starting the server
- `--stdio` — serve over stdio (default; also the bare-invocation default)
- `--http` — serve over Streamable HTTP (default port 8080, default host `127.0.0.1`)
- `--host` / `--port` only apply to `--http`; mixing them with `--stdio` is a usage error
- `list` — print a recursive listing of every memory file, with line counts and version ranges
- `show <file> [version]` — print one memory file's content, optionally as of a past version; renders as styled markdown on a terminal, raw when piped, overridable with `--raw` or `--render`
- `versions <file>` — print a file's version timeline (number, UTC timestamp, cause), oldest first
- `changes [timespan]` — print a global change log across all files, newest first; `timespan` is `24h`-style durations, a `YYYY-MM-DD` date, a `START..END` date range, or `all` (default: last 7 days)
- All four inspection commands are read-only: they never create or seed the memory root or the version store
- `memorija help` shows the tool's purpose, version, build date, and usage
- Invalid invocations exit non-zero and never create the memory root

### Web viewer

```sh
memorija web                                # http://127.0.0.1:8081/
memorija web --host 0.0.0.0 --port 9000
memorija web --no-open
```

`web` starts a local, read-only web viewer of your memory files and tries to
open the main page in the system browser (the URL is printed either way;
`--no-open` skips the browser launch). It has **no authentication** and warns
on startup when binding to a non-loopback address. Defaults: port `8081`
(beside `serve --http`'s 8080), host `127.0.0.1`.

The viewer offers no way to modify memories — it only reads the memory root
and the version store, and never creates or seeds either. It uses a monospace,
square-edged terminal layout; the `/` key focuses the path filter on the
Changes and Files screens, and `Escape` releases it.

The viewer follows your system's light or dark appearance, and how wide the page
is on your screen is yours to set. Both live in one settings dialog: the gear at
the right of the header opens it, and it holds the three appearance choices —
**Light**, **Dark**, and **System**, as labelled segments — and a page-width
slider that runs from a comfortable minimum to the full width of the window.
Everything applies immediately, so the page reflows behind the dialog as you
drag; the dialog itself only opens and closes. `Escape`, a click outside it, or
**Close** dismisses it and returns focus to the gear.

Both choices are remembered in the browser (two `localStorage` entries scoped to
the viewer's address, so a different port or host starts from the defaults
again) and are never sent to the server. A remembered width is applied before
the page paints, so it never shows at another width first. Choosing **System**
returns to the operating system preference, and an open page follows it without
a reload. Rendered files keep a readable line length however wide the shell
gets, while change and file rows, code blocks, and tables use the room. Neither
setting touches a memory file or version record.

The header navigates three screens, each at its own address:

- **Changes** (`/`) lists recorded changes newest first, grouped by day: time,
  file path, cause, `+N -M`, and the version. Filter by timespan (`24h`
  default, `7d`, `all`) and by path; both are carried in the address, so a
  reload or bookmark keeps them. Selecting a change opens the Reader at that
  recorded version.
- **Files** (`/files?q=`) lists every memory file in the memory root, grouped
  by directory in path order, with its line count, latest version, and last
  change time; files without recorded changes appear too. The intro reports
  how many files are listed (`N of M` while filtering). Selecting one opens
  the Reader at its latest version.
- **Reader** (`/file?path=<file>&version=<n>&from=changes|files`) renders the
  file from markdown in a `:: path` bar with the shown line count and its
  recorded versions as links, ending in `HEAD` (the current content); a past
  version is marked as historical with a return to the latest. Without
  `version`, the current content is shown. Files with no recorded versions
  show just their content. A fenced ` ```mermaid ` block is drawn as a
  diagram in the appearance in effect, and redrawn when you change it; an
  invalid diagram leaves its source visible with a note instead.

Old `/?path=<file>` bookmarks still resolve to the same Reader view. An
unknown address shows a not-found screen with a way back.

The viewer **updates on its own**: the server polls the version store and
the memory root's top level every second and pushes the feed and file list
over Server-Sent Events to whichever screen is open. The Changes screen shows
an "N new changes" indicator when you are scrolled away from the top, and the
Reader offers an "updated — view vN" link instead of replacing the content
you are reading.

Requesting a path that does not exist shows a clear not-found screen; paths
that escape the memory root (by `..` traversal or a symlink pointing outside)
are refused, exactly as in the CLI and MCP surfaces. Rendered markdown never
executes scripts: HTML inside a memory file displays as text, and a diagram's
source is treated as data — its labels stay text and its links and directives
are not made live.

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

The web viewer ships no front-end build step: `src/web/index.html`,
`app.css`, `prefs.js`, and `app.js` are embedded in the binary, served from the
same origin with `Cache-Control: no-cache`. `prefs.js` is a tiny
parser-blocking bootstrap in `<head>`: it applies a stored appearance choice
and page width before the body paints, which is why it exists as its own asset
instead of an inline script — the width is written to the `--shell` custom
property through CSSOM, which needs no inline `style` attribute. The terminal
appearance lives entirely in `app.css`: semantic custom properties (`--page`,
`--panel`, `--rule`, `--ink`, `--accent`, …) declared once for light and once
for dark, repeated for the explicit `data-theme` selectors. The shell width is
`clamp(40rem, calc(var(--shell) * 1vw), 100%)`, so the `40rem` floor keeps the
row grids intact at any window size. The settings dialog is a native `<dialog>`
opened with `showModal()`, so modality, the focus trap, `Escape` and the
backdrop come from the platform rather than from script.

Diagrams come from a vendored Mermaid build,
`src/web/vendor/mermaid.min.js` (version, checksum and MIT licence are in
[`src/web/vendor/README.md`](src/web/vendor/README.md)), embedded in the binary
and served at `/mermaid.js`. The client asks for it only when a rendered file
contains a ` ```mermaid ` block, draws each block from the escaped source the
server sent, and leaves the source in place whenever a diagram cannot be
drawn. The SVG Mermaid returns is scrubbed before it reaches the page (anchors
unwrapped, scripts and `on*` attributes dropped), so a diagram cannot navigate
or run code. One CSP exception exists for that drawing: `style-src` allows
`'unsafe-inline'`, because Mermaid writes its SVG styling as a `<style>` element
and style attributes; `script-src` stays `'self'` with no inline script and no
`'unsafe-eval'`, and nothing is fetched from another origin. A diagram upgrade
must keep those three properties — the commands that check them are in the
vendor README.

Specs and change history live under [`openspec/`](openspec/). See
[AGENTS.md](AGENTS.md) for the OpenSpec workflow used in this repository.

### CI and releases

`.github/workflows/ci.yml` runs on every push to `main` and every pull request:
`cargo fmt --all -- --check`, `cargo clippy --all-targets --locked -- -D warnings`,
the test suite on Ubuntu, macOS and Windows, and a release build for each
supported target — `x86_64-unknown-linux-gnu` (on `ubuntu-22.04`, for the glibc
floor), `aarch64-apple-darwin`, `x86_64-apple-darwin` (cross-built on the arm64
runner), and `x86_64-pc-windows-msvc`. Tests that need Unix symlinks are
`#[cfg(unix)]`, so the Windows run compiles and runs everything else. The memory
root is found through `$HOME`, falling back to `%USERPROFILE%` on Windows.

To release, bump the version in `Cargo.toml`, commit, and push a tag with that
version:

```sh
git tag v0.9.0
git push origin v0.9.0
```

`.github/workflows/release.yml` refuses a tag that disagrees with the crate
version. Otherwise it builds one archive per target with the binary, `README.md`
and `LICENSE` at the top level, writes `SHA256SUMS`, smoke-tests the Linux
binary, and attaches everything to the GitHub Release for that tag. Re-running a
failed release is safe: `gh workflow run release.yml -f tag=v0.9.0` creates the
release if it is missing and uploads with `--clobber`.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
