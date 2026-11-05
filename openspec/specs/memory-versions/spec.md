# memory-versions Specification

## Purpose

Records the history of memory file changes as logical version records — path, content, cause, timestamp — appended on every mutation, so past states remain retrievable and the model can ask what changed and when. Records are storage-neutral data, kept out of the visible memory tree.

## Requirements

### Requirement: Mutations append version records
The server SHALL append a version record whenever a command changes the content of a memory file (`create`, `str_replace`, `insert`), capturing the path, the resulting full content, the causing command, and the time of the change. Each record SHALL be immutable after it is written.

#### Scenario: Edit history accumulates
- **WHEN** a file is created, then modified twice by `str_replace` and `insert`
- **THEN** the file has three version records: one per change, each with its own cause and timestamp

#### Scenario: Records survive later changes
- **WHEN** any later mutation or restore follows an earlier record
- **THEN** the earlier record's content, cause, and timestamp are unchanged

### Requirement: History shows a per-file version timeline
The server SHALL return, for `history` on an existing memory file, that file's version records as a timeline ordered from oldest to newest, each entry showing its version number, timestamp, cause, and a compact change summary against the previous version.

#### Scenario: Timeline for an edited file
- **WHEN** `history` targets a file with three version records
- **THEN** the result lists all three in chronological order with version number, timestamp, cause, and change summary

#### Scenario: History on a path without records
- **WHEN** `history` targets a path that does not exist or has never been recorded
- **THEN** the result is an error result naming the path

#### Scenario: History is per path
- **WHEN** a file is moved to a new path with `rename`
- **THEN** the new path has no version records; history does not follow the file across paths

### Requirement: Restore creates a new version from a past one
The server SHALL, for `restore` on an existing memory file with a `version` referencing one of that file's records, set the file's content to that record's content by appending a new version record whose cause is `restore`; existing records SHALL not be changed, removed, or reordered.

#### Scenario: Restoring to an earlier version
- **WHEN** `restore` targets a file's version 2 out of four records
- **THEN** the file's content equals version 2's content and `history` shows a fifth entry with cause `restore`

#### Scenario: Restore is itself undoable
- **WHEN** a restore to an earlier version is followed by a restore to the latest prior version
- **THEN** the file's content equals that version's content and both restores appear in the timeline

#### Scenario: Restore with an unknown version
- **WHEN** `restore` targets a version number the file has no record for
- **THEN** the file is unchanged and the result is an error result naming the file's available version range

### Requirement: Version records are invisible in the memory tree
The server SHALL store version records outside the model-visible memory tree, so that directory listings, recursive views, and containment rules of the memory root never surface them as ordinary files.

#### Scenario: Root listing hides version storage
- **WHEN** `view` targets the memory root after many recorded mutations
- **THEN** the listing shows only the user's memory files and directories, with no version storage entries

#### Scenario: Records are not editable through the tool
- **WHEN** any of the six original commands targets a path inside the version storage
- **THEN** the operation is refused as if the path did not exist

### Requirement: Version causes distinguish change origins
The server SHALL record as the cause of a version the command that produced it (`create`, `str_replace`, `insert`), or `restore` for versions created by restore, so history entries state what kind of change each version represents.

#### Scenario: Causes match their commands
- **WHEN** a file is created, edited with `str_replace`, and restored to its first version
- **THEN** the three timeline entries show causes `create`, `str_replace`, and `restore` respectively
