# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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

## REMOVED Requirements

### Requirement: Main page shows the last changed file rendered

**Reason**: The one-page stack is replaced by separate screens. The Changes screen no longer embeds a rendered file; the Reader is its own screen with its own address.

**Migration**: Open any change to reach the Reader at that version, or open any entry in the Files screen. The former in-page rendering is available at `/file?path=<file>`.
