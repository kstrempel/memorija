# Spec Delta

## ADDED Requirements

### Requirement: Recorded paths use a canonical form

A version record's path SHALL be the memory file's path relative to the memory
root with `/` separating segments, whatever platform wrote it. Decoding a store
file name SHALL yield that form, including a name an earlier Windows build wrote
with `\` separators, and a store file that exists only under the legacy `\`
spelling SHALL still be read from and appended to, so history recorded before
this rule stays available.

#### Scenario: A nested mutation records forward slashes

- **WHEN** a nested file such as `projects/alpha/notes.txt` is created on any
  platform
- **THEN** the appended record's path is `projects/alpha/notes.txt` and its store
  key encodes the same `/` separators

#### Scenario: A legacy Windows store stays readable

- **WHEN** the store holds records under a `%5C`-encoded name written by an
  earlier Windows build and history is read for the canonical `a/b.txt` path
- **THEN** those records are returned with their content, causes and timestamps
  unchanged

#### Scenario: Appending continues a legacy sequence

- **WHEN** a further mutation follows a legacy `\`-keyed record for the same file
- **THEN** the new record is appended beside the old ones with the next sequence
  number, and reading the canonical path returns the whole history in order

#### Scenario: Two spellings do not split a history

- **WHEN** a file under a nested path is recorded by an earlier Windows build and
  then edited after this rule takes effect
- **THEN** one history exists for the file, not one per separator spelling
