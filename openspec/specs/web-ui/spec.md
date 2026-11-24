# web-ui Specification

## Purpose

Provides a local, read-only web viewer for the memory files: it turns the version store's change log into a live-updating page where each change and each memory file can be read as rendered HTML, without any ability to modify memories.

## Requirements

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

### Requirement: Viewer presents each screen at its own address

The viewer SHALL serve three screens at distinct addresses: Changes, Files, and the Reader (a single memory file). It SHALL show a shared navigation that marks the current screen, and `/` SHALL show the Changes screen.

#### Scenario: Each screen has its own address

- **WHEN** the user opens the Changes, Files, or Reader address
- **THEN** that screen is shown, and the shared navigation marks it as current

#### Scenario: Root address lands on Changes

- **WHEN** the user opens `/` without any other address detail
- **THEN** the Changes screen is shown

#### Scenario: Reload and history restore the screen

- **WHEN** the user reloads the browser, or moves back and forward in its history
- **THEN** the previously shown screen is shown again with the same filters and file selection

#### Scenario: Unknown screen address

- **WHEN** the user opens an address the viewer does not serve
- **THEN** a clear not-found screen is shown with a way back to Changes, not a bare error

#### Scenario: Reader returns to its origin

- **WHEN** the user follows the Reader's back link
- **THEN** the screen the file was opened from is shown again

### Requirement: Changes screen filters recorded activity

The Changes screen SHALL let the user restrict the feed to a rolling timespan and to a file-path filter, and SHALL carry the chosen filters in the address so a reload keeps them.

#### Scenario: Timespan filter

- **WHEN** the user chooses a timespan such as 24 hours, 7 days, or all
- **THEN** only changes recorded in that window are listed, and the address records the choice

#### Scenario: Path filter

- **WHEN** the user types part of a file path into the filter
- **THEN** only changes for paths containing that text are listed, and the address records the filter

#### Scenario: Filter matches nothing

- **WHEN** the active filters match no recorded change
- **THEN** the feed shows a clear empty state for the filters rather than an error

### Requirement: Files screen shows per-file metadata

Each entry in the Files screen SHALL show the file's line count, its latest recorded version, and the time of its last recorded change.

#### Scenario: Metadata for a recorded file

- **WHEN** the Files screen lists a file with recorded versions
- **THEN** its line count, latest version, and last change time are shown alongside its path

#### Scenario: Metadata for a file without history

- **WHEN** the Files screen lists a file with no recorded versions
- **THEN** it is listed with its line count and no version or change time

### Requirement: Files screen filters by path

The Files screen SHALL let the user narrow the list with a text filter matched against file paths.

#### Scenario: Filter narrows the list

- **WHEN** the user types part of a path into the Files filter
- **THEN** only files whose path contains that text are listed

#### Scenario: Clearing the filter restores the list

- **WHEN** the user clears the filter
- **THEN** every memory file is listed again

### Requirement: Path filters focus with a slash shortcut

The Changes and Files screens SHALL focus their path filter when the user presses `/` while no text field, select, or editable element has focus, and SHALL ignore the shortcut while Control, Alt, or Meta is held. The Changes screen SHALL show a visible hint that `/` focuses the filter, and pressing Escape in the focused filter SHALL release focus without changing the filter.

#### Scenario: Slash focuses the filter on Changes

- **WHEN** the Changes screen is open, no editable element has focus, and the user presses `/`
- **THEN** the path filter gains focus and subsequent typing enters text into it, while the slash itself is not entered

#### Scenario: Slash focuses the filter on Files

- **WHEN** the Files screen is open, no editable element has focus, and the user presses `/`
- **THEN** the Files path filter gains focus and subsequent typing narrows the list

#### Scenario: Typing a slash inside a field is untouched

- **WHEN** any text field, including the path filter itself, has focus and the user types `/`
- **THEN** the character is entered in that field and the shortcut does not fire

#### Scenario: Escape releases the focused filter

- **WHEN** the path filter has focus and the user presses Escape
- **THEN** the filter loses focus and its current text and the listed results are unchanged

#### Scenario: The hint names the shortcut

- **WHEN** the Changes screen is shown
- **THEN** a visible hint tells the user that pressing `/` focuses the filter

### Requirement: Reader browses a file's versions

The Reader SHALL render a file at its latest version by default and SHALL offer the file's recorded versions for selection. A selected past version SHALL be visibly marked as historical and SHALL offer a return to the latest version.

#### Scenario: Version strip from recorded history

- **WHEN** the Reader opens a file with recorded versions
- **THEN** its recorded versions are offered for selection, with the latest one selected

#### Scenario: Past version is rendered

- **WHEN** the user selects a past version
- **THEN** the file's content as of that version is rendered, and the screen marks it as a past version with a way back to the latest

#### Scenario: Returning to the latest version

- **WHEN** the user follows the return-to-latest action from a past version
- **THEN** the file's current content is rendered and no past-version marking remains

#### Scenario: Change row opens that version

- **WHEN** the user selects a change in the Changes screen
- **THEN** the Reader opens that file at the version the change recorded

#### Scenario: File without recorded history

- **WHEN** the Reader opens a file with no recorded versions
- **THEN** its current content is rendered and no version strip is offered

### Requirement: Main page shows the changes feed newest first

The Changes screen SHALL show a feed of recorded changes (time, file path, cause, and change size), ordered newest first and grouped by day, drawn from the version store.

#### Scenario: Feed reflects recorded changes

- **WHEN** the version store contains records for several files across several days
- **THEN** the Changes screen lists those changes newest first with time, path, cause, and change size, under day headings

#### Scenario: Fresh machine shows an empty state

- **WHEN** the version store contains no records
- **THEN** the Changes screen shows an empty-state message instead of a feed and no error

### Requirement: Each file has a stable URL

Selecting a change or a file SHALL show it in the Reader at its own address carrying the file path and, when one was chosen, the version, so reloading that address after a browser restart returns to the same file and version. The former `/?path=<file>` address SHALL keep resolving to the same Reader view.

#### Scenario: Navigation to a file and back

- **WHEN** the user opens a file from a list and then reloads the browser
- **THEN** the same file's rendered content is shown at the same address

#### Scenario: Versioned address is stable

- **WHEN** the user opens a past version of a file and reloads the browser
- **THEN** that same past version is shown again

#### Scenario: Legacy file address still works

- **WHEN** the user opens an existing `/?path=<file>` bookmark
- **THEN** the Reader shows that file, the same view as its current address

#### Scenario: Unknown or missing file

- **WHEN** the viewer is asked for a file that does not exist in the memory root
- **THEN** the page shows a clear not-found message rather than an error page or a wrong file

### Requirement: Changes appear without manual reload

While any screen is open, the viewer SHALL detect newly recorded changes in the version store and newly appearing or disappearing files in the memory root, and update the visible screen automatically without the user reloading or losing live updates when switching screens.

#### Scenario: Agent records a change while the page is open

- **WHEN** an agent session records a new version of a memory file while the Changes screen is open
- **THEN** the feed gains that change within a few seconds without user action

#### Scenario: Agent records a change while the Reader is open

- **WHEN** the file shown in the Reader gains a new version while it is open
- **THEN** the Reader shows that the file changed and offers the new version without a reload

#### Scenario: Switching screens keeps updates flowing

- **WHEN** the user moves from one screen to another without reloading the browser
- **THEN** live updates continue on the new screen without reconnecting or refreshing

### Requirement: File access stays inside the memory root

The viewer SHALL NOT serve any file outside the memory root: a requested path that escapes the root by traversal or by following a symbolic link outside it SHALL be refused with a clear error, reusing the same path-safety rules as the rest of the tool.

#### Scenario: Traversal outside the root is refused

- **WHEN** a request asks for a path such as `/memories/../secrets.txt` or a path whose symlink resolves outside the root
- **THEN** the request is refused with a clear error and no content outside the root is served

### Requirement: Rendered markdown cannot execute scripts

When rendering memory file content as HTML, the viewer SHALL escape embedded HTML so that content in a memory file cannot execute script in the browser. A drawn diagram SHALL inherit that guarantee even though drawing may add inline style to the page.

#### Scenario: Memory file contains raw HTML

- **WHEN** a memory file contains markup such as a `<script>` tag
- **THEN** the page displays it as text and no script runs

#### Scenario: Inline styles do not open a script path

- **WHEN** the page carries the inline style a drawn diagram produced
- **THEN** no inline script or event handler in the page can run, and memory content still cannot introduce markup of its own

### Requirement: All-files index shows every memory file

The Files screen SHALL list every memory file in the memory root, grouped by its directory and ordered by path, including every file regardless of whether it has recorded changes in the version store.

#### Scenario: Index lists files without change history

- **WHEN** the memory root contains files that have no records in the version store alongside files that do
- **THEN** the list shows all of those files by path, not only the ones with recorded changes

#### Scenario: Index is sorted and stable

- **WHEN** the list is rendered
- **THEN** files are grouped under their directories in path order, and reloading the page returns the same ordering

#### Scenario: Fresh machine shows an empty index

- **WHEN** the memory root contains no memory files (or does not exist)
- **THEN** the Files screen shows an empty-state message instead of an error

### Requirement: Index entries open the rendered file view

Selecting an entry in the Files screen SHALL open that file in the Reader at its latest recorded version, or its current content when it has no recorded version.

#### Scenario: Index navigation reaches the same view

- **WHEN** the user selects a file in the Files screen
- **THEN** the Reader shows the file's rendered content at its own address, indistinguishable from opening it through a change

#### Scenario: File without history opens its current content

- **WHEN** the user selects a file that has no recorded versions
- **THEN** the Reader shows its current content and offers no version strip

### Requirement: Index stays current without manual reload

While the Files screen is open, the viewer SHALL update the list when files appear in or disappear from the memory root, without the user reloading the page.

#### Scenario: New file appears while the page is open

- **WHEN** a memory file is created in the memory root while the Files screen is open
- **THEN** the list gains that file within a few seconds without user action

#### Scenario: Removed file disappears from the index

- **WHEN** a memory file is deleted from the memory root while the Files screen is open
- **THEN** the list no longer shows that file within a few seconds without user action

### Requirement: Viewer offers light and dark appearances

The viewer SHALL render every screen in either a light or a dark appearance. Text, backgrounds, borders, links, muted text, and the change-size colours SHALL stay legible and distinguishable in both appearances.

#### Scenario: Each screen renders in both appearances

- **WHEN** the viewer shows the Changes, Files, Reader, not-found, or empty state in the light appearance and again in the dark appearance
- **THEN** every screen is fully styled in both, with no unstyled or light-only region

#### Scenario: Rendered markdown follows the appearance

- **WHEN** the Reader shows markdown that contains a heading, a code block, and a table in the dark appearance
- **THEN** those elements use dark-appropriate text, background, and border colours rather than the light palette

#### Scenario: Change-size colours stay distinguishable

- **WHEN** the Changes screen lists an added and a removed line count in either appearance
- **THEN** both counts are visible against the row background and remain distinguishable from each other

### Requirement: Appearance follows the system preference by default

Without an explicit choice, the viewer SHALL match the operating system's or browser's light or dark preference, and SHALL follow a change to that preference while a screen is open without a reload.

#### Scenario: First visit on a dark system

- **WHEN** the viewer is opened in a browser whose preferred appearance is dark and no appearance was chosen in that browser
- **THEN** the viewer is shown in the dark appearance

#### Scenario: First visit on a light system

- **WHEN** the viewer is opened in a browser whose preferred appearance is light and no appearance was chosen in that browser
- **THEN** the viewer is shown in the light appearance

#### Scenario: System preference changes while the page is open

- **WHEN** the system preference switches from light to dark while a screen is open and no explicit choice was made in that browser
- **THEN** the open screen switches to the dark appearance without a reload

### Requirement: Appearance control offers Light, Dark, and System

The settings dialog SHALL present the three appearance choices — Light, Dark, and System — as labelled controls with accessible names and a pressed state. The control SHALL be available on every screen, and no separate status readout of the appearance in effect SHALL be shown.

#### Scenario: Control is present on every screen

- **WHEN** the user opens the Changes, Files, Reader, or not-found screen and then the settings dialog
- **THEN** the three labelled appearance choices are part of the dialog, each names its choice to assistive technology, and the active choice is marked

#### Scenario: Choosing an appearance applies immediately

- **WHEN** the user chooses Light or Dark in the dialog
- **THEN** the visible screen switches to that appearance at once, that choice is marked as active, and the dialog stays open

#### Scenario: Choosing System returns to the OS preference

- **WHEN** the user chooses System after having chosen Light or Dark
- **THEN** the viewer matches the browser's current preferred appearance, follows it from then on, and marks System as active

### Requirement: Explicit appearance choice is remembered per browser

An explicit appearance choice SHALL persist across reloads and later visits in the same browser. It SHALL be stored in the browser only and SHALL NOT be written to the server, the memory root, or the version store.

#### Scenario: Reload keeps the chosen appearance

- **WHEN** the user chooses Dark and then reloads the page or restarts the viewer
- **THEN** the viewer opens in the dark appearance without briefly using the other one

#### Scenario: Another browser starts from the system preference

- **WHEN** the viewer is opened in a browser or profile that has no stored choice
- **THEN** it follows the system preference rather than a choice made elsewhere

#### Scenario: Browser storage is unavailable

- **WHEN** the browser refuses or discards the stored choice
- **THEN** the viewer still offers the control, applies choices for the current page, and falls back to the system preference on the next visit

#### Scenario: The viewer stays read-only

- **WHEN** the user changes the appearance
- **THEN** no memory file or version record is created, changed, or deleted

### Requirement: Settings dialog holds the viewer's settings

The shared header SHALL present a settings control on every screen that opens a modal dialog containing the viewer's settings. The dialog SHALL dismiss on its close control, on Escape, or on a click outside it, and SHALL return focus to the control that opened it.

#### Scenario: Settings are reachable from every screen

- **WHEN** the user opens the Changes, Files, Reader, empty, or not-found screen
- **THEN** the settings control is part of the shared header, names what it opens for assistive technology, and opens the same dialog

#### Scenario: The dialog dismisses without changing anything

- **WHEN** the dialog is open and the user activates Close, presses Escape, or clicks outside the dialog
- **THEN** the dialog closes and the visible screen is unchanged except for the settings that were already applied

#### Scenario: Focus returns to the control that opened the dialog

- **WHEN** the user closes the dialog with the keyboard
- **THEN** focus is on the settings control in the header, not on the page body

### Requirement: Page width is adjustable

The settings dialog SHALL offer a page-width control that sets the width of the content shell as a continuous value between a comfortable minimum and the full window width, defaulting to a mid-range width. The chosen width SHALL be remembered per browser and SHALL never be sent to the server or written to the memory root or the version store.

#### Scenario: Adjusting the width applies at once

- **WHEN** the user moves the page-width control
- **THEN** the shell widens or narrows immediately, the dialog stays open, and the visible screen keeps its filters, scroll position, and selected file without a reload

#### Scenario: Width is remembered per browser

- **WHEN** the user sets a width and then reloads the page or restarts the viewer
- **THEN** the page opens at that width without first painting at another one

#### Scenario: Another browser starts from the default

- **WHEN** the viewer is opened in a browser or profile with no stored width
- **THEN** it uses the default mid-range width rather than a width chosen elsewhere

#### Scenario: Smallest setting stops at the layout minimum

- **WHEN** the user chooses the smallest width
- **THEN** the shell stops at a width where the change and file rows still keep their columns instead of narrowing further

#### Scenario: Rendered files keep a readable measure

- **WHEN** the user widens the shell and opens a file in the Reader
- **THEN** prose lines stop growing at a readable measure, while tables, code blocks, and the list screens use the available width

#### Scenario: The viewer stays read-only

- **WHEN** the user changes the width
- **THEN** no memory file or version record is created, changed, or deleted

### Requirement: Reader draws Mermaid diagrams

The Reader SHALL draw a fenced code block whose info string is `mermaid` as an SVG diagram wherever it renders a memory file's markdown — in the current content and in every recorded version. Every other code block SHALL render exactly as before, as escaped source text.

#### Scenario: A diagram in the current content

- **WHEN** the Reader renders a file containing a ` ```mermaid ` block with a valid flowchart
- **THEN** the block is shown as a drawn diagram, in place of its source text, inside the rendered file

#### Scenario: A diagram in a past version

- **WHEN** the user selects a recorded version whose content contains a Mermaid block
- **THEN** that version's diagram is drawn, and the screen still marks the version as historical

#### Scenario: Other code blocks are untouched

- **WHEN** a file contains a fenced block with a language other than `mermaid`, or a plain code block
- **THEN** it stays a code block with its language class and source text, and no diagram is drawn for it

#### Scenario: Inline code and prose are untouched

- **WHEN** a file mentions the word `mermaid` in prose or in inline code without a `mermaid` fence
- **THEN** nothing on the page is turned into a diagram

#### Scenario: A file with several diagrams

- **WHEN** one file contains two or more `mermaid` blocks with different diagram types
- **THEN** each block is drawn as its own diagram, in the order the blocks appear

### Requirement: An undrawable diagram keeps its source

When a Mermaid block's content is invalid, unsupported, or too large to draw, the Reader SHALL keep that block's source visible and readable and SHALL mark it as not drawn. A failed diagram SHALL NOT become an error page, SHALL NOT hide the rest of the file, and SHALL NOT stop the file's other diagrams from being drawn.

#### Scenario: Invalid diagram source

- **WHEN** the Reader renders a `mermaid` block whose source does not parse
- **THEN** the block's source remains visible with a note that the diagram could not be drawn, and the rest of the file still renders

#### Scenario: One bad diagram among good ones

- **WHEN** a file contains one invalid and one valid `mermaid` block
- **THEN** the valid block is drawn and the invalid one keeps its source with the note

#### Scenario: The fallback is readable

- **WHEN** a diagram has fallen back to its source
- **THEN** the source keeps the formatting of a code block in both appearances, rather than appearing as unstyled text

### Requirement: Diagrams follow the viewer appearance

A drawn diagram SHALL use colours and type legible in the appearance in effect, and SHALL be redrawn in the matching appearance when the user changes it, without reloading, rebuilding the Reader, or losing the shown file, version, scroll position, or the settings dialog's open state.

#### Scenario: Diagram in the light appearance

- **WHEN** the viewer is in the light appearance and the Reader shows a diagram
- **THEN** its text and shapes are drawn against light-appropriate colours

#### Scenario: Diagram in the dark appearance

- **WHEN** the viewer is in the dark appearance and the Reader shows a diagram
- **THEN** its text and shapes are drawn against dark-appropriate colours, not the light palette

#### Scenario: Appearance changes while a diagram is shown

- **WHEN** the user chooses the other appearance while the Reader shows a diagram
- **THEN** the diagram is redrawn to match, the shown version and scroll position stay as they were, and the settings dialog stays open

#### Scenario: System preference changes while a diagram is shown

- **WHEN** the viewer follows the system preference and that preference flips while a diagram is shown
- **THEN** the diagram is redrawn to match without a reload

### Requirement: Diagrams stay inside the page

A diagram SHALL NOT widen the content shell, push the page sideways, or overflow outside its file content area. A diagram wider than the available measure SHALL scale down to it and SHALL remain reachable at its full size inside its own scrolling frame. A diagram smaller than the measure SHALL be drawn at its natural size rather than stretched.

#### Scenario: A wide diagram at the smallest page width

- **WHEN** the user selects the smallest page width and opens a file with a wide diagram
- **THEN** the page keeps that width, no horizontal page scrollbar appears, and the diagram can be inspected by scrolling within its own frame

#### Scenario: A small diagram is not stretched

- **WHEN** a file's diagram is narrower than the reading measure
- **THEN** it is drawn at its natural size and left-aligned, not scaled up

#### Scenario: Diagram text stays legible

- **WHEN** a diagram is scaled down to fit the measure
- **THEN** it is not shrunk below the point where its labels become unreadable, and the frame offers the full-size view instead

### Requirement: Diagram support loads only when a diagram is present

The viewer SHALL request the diagram renderer only after it renders a file that actually contains a `mermaid` block, and SHALL take that renderer from its own origin. A screen or file without a diagram SHALL make no diagram-renderer request. If the renderer cannot be fetched or started, diagrams SHALL fall back to their source as a failed diagram does, and the rest of the page SHALL keep working.

#### Scenario: A file without a diagram

- **WHEN** the Reader opens a file with no `mermaid` block
- **THEN** no diagram-renderer request is made and the page behaves exactly as before

#### Scenario: The renderer comes from the viewer's origin

- **WHEN** a file with a diagram is opened
- **THEN** the renderer is fetched from the viewer's own address and nothing is requested from a third-party host

#### Scenario: The renderer is unavailable

- **WHEN** the renderer cannot be fetched or fails to start
- **THEN** every diagram on the page shows its source with the not-drawn note, and the rest of the file renders normally

#### Scenario: The viewer stays offline-capable

- **WHEN** the machine has no network access and a file with a diagram is opened
- **THEN** the diagram is drawn as usual

### Requirement: Diagram content cannot run code or reach the network

A diagram's source SHALL be treated as data, never as page markup or script: labels, titles, links, and directives in it SHALL NOT execute script, open a page, or navigate the viewer. Drawing a diagram SHALL NOT make any network request beyond the viewer's own renderer asset. The page's Content-Security-Policy SHALL continue to forbid inline and third-party script.

#### Scenario: Script in a diagram label

- **WHEN** a `mermaid` block contains markup such as `<script>` or an event-handler attribute in a node label
- **THEN** it is drawn as literal text or the diagram falls back to source, and no script runs

#### Scenario: Click directives and links

- **WHEN** a `mermaid` block contains `click` directives or `href` links in nodes
- **THEN** nodes are not clickable and nothing navigates away, opens a window, or loads a URL

#### Scenario: Remote resources in a diagram

- **WHEN** a `mermaid` block tries to reference a remote image, font, or stylesheet
- **THEN** nothing is fetched over the network and the diagram renders without it

#### Scenario: The policy stays script-strict

- **WHEN** any screen address is requested
- **THEN** the response's policy allows script only from the viewer's own origin, forbids inline script and inline event handlers, and permits inline styles only for drawing

### Requirement: Viewer paths use forward slashes

Every path the viewer reports for a memory file — an index entry, a change row, or a Reader address — SHALL use `/` between segments on every platform, and SHALL round-trip: taking a path the viewer shows and asking for that file SHALL open the same file and its history.

#### Scenario: The Files index on Windows

- **WHEN** the Files screen lists a root holding `projects/alpha/notes.txt` on Windows
- **THEN** the entry's path is `projects/alpha/notes.txt`, not a backslash spelling

#### Scenario: An index entry opens its file

- **WHEN** a person selects that nested entry
- **THEN** the Reader opens the file at its address and shows its content and recorded versions

#### Scenario: A change row names the same file

- **WHEN** the changes feed shows a change to that file
- **THEN** the row's path matches the index entry's path exactly
