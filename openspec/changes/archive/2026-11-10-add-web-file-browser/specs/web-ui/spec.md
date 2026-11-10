# Spec Delta

## ADDED Requirements

### Requirement: All-files index shows every memory file

The main page SHALL show, alongside the changes feed, an index of every memory file in the memory root, listed by its relative path and sorted by path. The index SHALL include every file regardless of whether it has recorded changes in the version store.

#### Scenario: Index lists files without change history

- **WHEN** the memory root contains files that have no records in the version store alongside files that do
- **THEN** the index lists all of those files by path, not only the ones with recorded changes

#### Scenario: Index is sorted and stable

- **WHEN** the index is rendered
- **THEN** the files appear ordered by their relative path, and reloading the page returns the same ordering

#### Scenario: Fresh machine shows an empty index

- **WHEN** the memory root contains no memory files (or does not exist)
- **THEN** the index shows an empty-state message instead of an error

### Requirement: Index entries open the rendered file view

Clicking an entry in the all-files index SHALL open that file in the same rendered per-file view used by the feed, at the same stable per-file address.

#### Scenario: Index navigation reaches the same view

- **WHEN** the user clicks a file in the all-files index
- **THEN** the file's rendered content is shown at its stable per-file address, indistinguishable from opening it through the feed

### Requirement: Index stays current without manual reload

While the main page is open, the viewer SHALL update the all-files index when files appear in or disappear from the memory root, without the user reloading the page.

#### Scenario: New file appears while the page is open

- **WHEN** a memory file is created in the memory root while the main page is open
- **THEN** the index gains that file within a few seconds without user action

#### Scenario: Removed file disappears from the index

- **WHEN** a memory file is deleted from the memory root while the main page is open
- **THEN** the index no longer lists that file within a few seconds without user action
