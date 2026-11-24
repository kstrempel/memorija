# Spec Delta

## ADDED Requirements

### Requirement: Inspection output spells paths with forward slashes

Every path an inspection command prints for a memory file or directory SHALL use
`/` between segments on every platform, whether the path comes from walking the
memory tree or from a record the version store holds. A directory entry SHALL
keep its trailing `/`.

#### Scenario: Listing a nested tree on Windows

- **WHEN** `memorija list` runs on Windows against a root holding
  `projects/alpha/notes.txt`
- **THEN** the output names `projects/`, `projects/alpha/` and
  `projects/alpha/notes.txt` with their line counts and version ranges

#### Scenario: Viewing a directory on Windows

- **WHEN** `memorija view /memories` runs on Windows against the same tree
- **THEN** each entry is spelled with `/`, so a path copied from the output
  addresses the same file on the same machine

#### Scenario: The change log normalises old records

- **WHEN** `memorija changes` reads a record an earlier Windows build wrote with
  `\` separators
- **THEN** the row still names the file with `/` segments
