# Spec Delta

## ADDED Requirements

### Requirement: Viewer paths use forward slashes

Every path the viewer reports for a memory file — an index entry, a change row,
or a Reader address — SHALL use `/` between segments on every platform, and SHALL
round-trip: taking a path the viewer shows and asking for that file SHALL open
the same file and its history.

#### Scenario: The Files index on Windows

- **WHEN** the Files screen lists a root holding `projects/alpha/notes.txt` on
  Windows
- **THEN** the entry's path is `projects/alpha/notes.txt`, not a backslash
  spelling

#### Scenario: An index entry opens its file

- **WHEN** a person selects that nested entry
- **THEN** the Reader opens the file at its address and shows its content and
  recorded versions

#### Scenario: A change row names the same file

- **WHEN** the changes feed shows a change to that file
- **THEN** the row's path matches the index entry's path exactly
