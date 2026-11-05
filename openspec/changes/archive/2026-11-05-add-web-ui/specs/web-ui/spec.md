# Spec Delta

## Purpose

Provides a local, read-only web viewer for the memory files: it turns the version store's change log into a live-updating page where each change and each memory file can be read as rendered HTML, without any ability to modify memories.

## ADDED Requirements

### Requirement: Web subcommand starts a read-only viewer

The binary SHALL accept a `web` subcommand that starts a local web server serving a read-only viewer of the memory root and blocks until the server stops. The viewer SHALL offer no operation that modifies memory files or version records.

#### Scenario: Viewer starts and prints its address

- **WHEN** the user runs `memorija web`
- **THEN** a web server starts on the default local address, the standard error output shows the URL of the main page, and the server keeps running until interrupted

#### Scenario: Viewer is read-only over HTTP

- **WHEN** an HTTP request asks the server to change, create, or delete any memory file or version record
- **THEN** no such operation exists and no file under the memory root or version store is modified

### Requirement: Browser opens automatically

When the web server starts, it SHALL attempt to open the main page in the system's default web browser, and SHALL print the URL to the standard error output regardless of whether the browser could be opened.

#### Scenario: Browser opens on demand

- **WHEN** the server starts on a machine with a default browser available
- **THEN** the main page opens in that browser and the URL is also printed

#### Scenario: No-open flag

- **WHEN** the user passes `--no-open` to the `web` subcommand
- **THEN** no browser is launched and the URL is printed

### Requirement: Web server binding follows the MCP HTTP posture

The `web` subcommand SHALL bind to `127.0.0.1` by default and SHALL accept a `--port <number>` option defaulting to 8081 and a `--host <address>` option. It SHALL print a warning to the standard error output when the host is not a loopback address, and SHALL exit with a non-zero code and a clear message when the address cannot be bound.

#### Scenario: Default port does not clash with MCP HTTP

- **WHEN** `memorija serve --http` is already running on port 8080 and the user runs `memorija web` with defaults
- **THEN** the web viewer starts successfully on port 8081

#### Scenario: Non-loopback bind warns

- **WHEN** the user passes `--host 0.0.0.0`
- **THEN** the server starts and a warning about unauthenticated access from the network is printed

#### Scenario: Unbindable address fails clearly

- **WHEN** the configured port is already in use
- **THEN** the process exits non-zero with an error naming the address

### Requirement: Main page shows the changes feed newest first

The main page SHALL show a feed of recorded changes (time, file path, cause, and change size), ordered newest first, drawn from the version store.

#### Scenario: Feed reflects recorded changes

- **WHEN** the version store contains records for several files
- **THEN** the main page lists those changes newest first with time, path, cause, and change size

#### Scenario: Fresh machine shows an empty state

- **WHEN** the version store contains no records
- **THEN** the main page shows an empty-state message instead of a feed and no error

### Requirement: Main page shows the last changed file rendered

The main page SHALL display below the feed the content of the most recently changed file, rendered from markdown into styled HTML.

#### Scenario: Last changed file is rendered as HTML

- **WHEN** the main page loads and at least one change is recorded
- **THEN** the most recently changed file's content appears rendered as formatted HTML (headings, lists, code, emphasis) rather than as raw markdown text

### Requirement: Each file has a stable URL

Clicking a feed entry SHALL show that entry's file rendered the same way at its own address, so reloading that address after a browser restart returns to the same file.

#### Scenario: Navigation to a file and back

- **WHEN** the user clicks a feed entry and then reloads the browser
- **THEN** the same file's rendered content is shown at the same URL

#### Scenario: Unknown or missing file

- **WHEN** the viewer is asked for a file that does not exist in the memory root
- **THEN** the page shows a clear not-found message rather than an error page or a wrong file

### Requirement: Changes appear without manual reload

While the main page or a file view is open, the viewer SHALL detect newly recorded changes in the version store and update the feed and the displayed content automatically, without the user reloading the page.

#### Scenario: Agent records a change while the page is open

- **WHEN** an agent session records a new version of a memory file while the page is open
- **THEN** the feed gains that change and the displayed content updates within a few seconds without user action

### Requirement: File access stays inside the memory root

The viewer SHALL NOT serve any file outside the memory root: a requested path that escapes the root by traversal or by following a symbolic link outside it SHALL be refused with a clear error, reusing the same path-safety rules as the rest of the tool.

#### Scenario: Traversal outside the root is refused

- **WHEN** a request asks for a path such as `/memories/../secrets.txt` or a path whose symlink resolves outside the root
- **THEN** the request is refused with a clear error and no content outside the root is served

### Requirement: Rendered markdown cannot execute scripts

When rendering memory file content as HTML, the viewer SHALL escape embedded HTML so that content in a memory file cannot execute script in the browser.

#### Scenario: Memory file contains raw HTML

- **WHEN** a memory file contains markup such as a `<script>` tag
- **THEN** the page displays it as text and no script runs
