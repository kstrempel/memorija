# cli-inspection Specification

## Purpose

Defines the binary's human-facing inspection commands — `list`, `show`, `versions`, and `changes` — which let a person examine the memory tree and its version history directly from the shell. All four are read-only: they report on the memory root and version store without ever modifying either.

## Requirements

### Requirement: List subcommand shows all memory files
The binary SHALL accept a `list` subcommand that prints a recursive listing of every file and directory in the memory root, each file shown with its line count and its available version range, without starting the MCP server.

#### Scenario: Listing a populated memory root
- **WHEN** the binary is invoked as `memorija list` and the memory root contains nested files
- **THEN** the output names every file, including files in subdirectories, each with its line count and the range of version numbers recorded for it, and the process exits with code zero

#### Scenario: Listing with no memory yet
- **WHEN** the binary is invoked as `memorija list` and the memory root does not exist
- **THEN** the process prints `no memories yet`, exits with code zero, and does not create or seed the memory root

### Requirement: Show subcommand prints a memory file
The binary SHALL accept a `show <file>` subcommand that prints one memory file's content, and SHALL accept an optional version number argument that selects the file's recorded content at that version instead of the live file.

#### Scenario: Showing the live file
- **WHEN** the binary is invoked as `memorija show notes.md` for an existing memory file
- **THEN** the file's current content is printed and the process exits with code zero

#### Scenario: Showing a past version
- **WHEN** the binary is invoked as `memorija show notes.md 2` and the file has a version 2 record
- **THEN** the content recorded for version 2 is printed, not the live content

#### Scenario: Showing an unknown version
- **WHEN** the binary is invoked with a version number the file has no record for
- **THEN** the process exits non-zero with an error naming the file's available version range

#### Scenario: Showing a missing file
- **WHEN** the binary is invoked as `memorija show` for a path that does not exist in the memory root
- **THEN** the process exits non-zero with an error naming the path

### Requirement: Show renders markdown on a terminal
The `show` subcommand SHALL render the file as styled markdown when standard output is a terminal, and SHALL print the file's raw content when standard output is piped. A `--raw` flag SHALL force raw output and a `--render` flag SHALL force rendered output regardless of stdout type.

#### Scenario: Rendering on a terminal
- **WHEN** `memorija show notes.md` runs with standard output connected to a terminal
- **THEN** the markdown is printed in rendered, styled form

#### Scenario: Piping falls back to raw
- **WHEN** `memorija show notes.md` runs with standard output piped to another process
- **THEN** the file's raw content is printed, containing no terminal styling

#### Scenario: Explicit flags override detection
- **WHEN** `show` is invoked with `--raw` on a terminal, or with `--render` into a pipe
- **THEN** the forced behavior is used: raw output in the first case, rendered output in the second

### Requirement: Versions subcommand shows a file's version timeline
The binary SHALL accept a `versions <file>` subcommand that prints the file's version records as a table of version number, UTC timestamp, and cause, ordered oldest first.

#### Scenario: Version table for an edited file
- **WHEN** `memorija versions notes.md` targets a file with three version records
- **THEN** the output lists all three with their version numbers, timestamps, and causes, oldest first, and exits with code zero

#### Scenario: Versions of a file without records
- **WHEN** `memorija versions` targets a path that exists but was never changed through the tool
- **THEN** the process exits non-zero with an error naming the path

### Requirement: Changes subcommand shows a global change log
The binary SHALL accept a `changes [timespan]` subcommand that prints one line per version record across all files in the version store, newest first, each line showing the date, time, file path, cause, and an added/removed line-count summary against the previous version.

#### Scenario: Recent changes across files
- **WHEN** the binary is invoked as `memorija changes 7d` and two different files were changed within the last seven days
- **THEN** the output shows both changes, newest first, each with date, time, path, cause, and change summary

#### Scenario: Timespan filters old records
- **WHEN** `memorija changes 24h` is invoked and the only recorded changes are older than 24 hours
- **THEN** no change lines are printed and the process exits with code zero

### Requirement: Changes timespan grammar
The `changes` subcommand SHALL accept a timespan of a duration suffix (`<n>h`, `<n>d`, `<n>w`, `<n>m`), an absolute `YYYY-MM-DD` date meaning since that day's start, a `START..END` date range, or `all`; a missing timespan SHALL default to the last 7 days. An unparseable timespan SHALL be rejected with a usage error.

#### Scenario: Duration suffixes
- **WHEN** `changes` is invoked with `24h`, `7d`, `2w`, or `3m`
- **THEN** only records whose timestamp falls within that rolling window before now are shown

#### Scenario: Absolute date and range
- **WHEN** `changes` is invoked with `2026-11-01`, only records from that day's start are shown; with `2026-10-01..2026-10-31`, only records within the range are shown

#### Scenario: Default timespan
- **WHEN** `changes` is invoked without a timespan
- **THEN** the result is identical to invoking it with `7d`

#### Scenario: Invalid timespan is rejected
- **WHEN** `changes` is invoked with a value matching none of the accepted forms
- **THEN** the process exits non-zero with a usage error naming the invalid value

### Requirement: Inspection commands are read-only
The inspection commands SHALL perform no filesystem changes: they SHALL not create or seed the memory root or the version store, and SHALL not append version records.

#### Scenario: Inspection leaves no trace on a fresh machine
- **WHEN** any inspection command is invoked on a machine where the memory root does not exist
- **THEN** after the process exits, neither the memory root nor the version store directory exists

### Requirement: Inspection output spells paths with forward slashes
Every path an inspection command prints for a memory file or directory SHALL use `/` between segments on every platform, whether the path comes from walking the memory tree or from a record the version store holds. A directory entry SHALL keep its trailing `/`.

#### Scenario: Listing a nested tree on Windows
- **WHEN** `memorija list` runs on Windows against a root holding `projects/alpha/notes.txt`
- **THEN** the output names `projects/`, `projects/alpha/` and `projects/alpha/notes.txt` with their line counts and version ranges

#### Scenario: Viewing a directory on Windows
- **WHEN** `memorija view /memories` runs on Windows against the same tree
- **THEN** each entry is spelled with `/`, so a path copied from the output addresses the same file on the same machine

#### Scenario: The change log normalises old records
- **WHEN** `memorija changes` reads a record an earlier Windows build wrote with `\` separators
- **THEN** the row still names the file with `/` segments
