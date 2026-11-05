# Spec Delta

## MODIFIED Requirements

### Requirement: Single unified memory tool
The server SHALL expose exactly one tool named `memory` whose input schema requires `command` and `path`, declares `command` as the enum `view`, `create`, `str_replace`, `insert`, `delete`, `rename`, `history`, `restore`, and declares `file_text`, `old_str`, `new_str`, `insert_line`, `insert_text`, `new_path`, `view_range`, and `version` as optional properties.

#### Scenario: Client discovers the tool
- **WHEN** an MCP client lists the server's tools
- **THEN** exactly one tool named `memory` is returned, with a description mentioning persistent memory files and an input schema whose `required` array is exactly `["command", "path"]`

#### Scenario: Tool description guides usage
- **WHEN** the tool list is returned
- **THEN** the `memory` tool's description explains that paths are relative to the memory root and what each command does

#### Scenario: Versioning commands are present in the schema
- **WHEN** the tool's input schema is inspected
- **THEN** `command` includes the values `history` and `restore`, and `version` is declared as an optional property

#### Scenario: Original commands are unchanged
- **WHEN** any of `view`, `create`, `str_replace`, `insert`, `delete`, or `rename` is called without `version`
- **THEN** the call behaves exactly as specified by the original command requirements

## ADDED Requirements

### Requirement: Version argument validation for history and restore
The server SHALL accept `history` with only `command` and `path`, SHALL require `version` for `restore`, and SHALL reject calls missing the fields their command needs with a tool result naming the missing field.

#### Scenario: Restore without version
- **WHEN** the tool is called with `command: "restore"` and no `version`
- **THEN** the result is an error result naming `version` as required for `restore`

#### Scenario: History needs no extra fields
- **WHEN** the tool is called with `command: "history"` and only `path`
- **THEN** the call is valid and the version timeline is returned

### Requirement: View reads a past version
The server SHALL return the content of the referenced version record, numbered with true line numbers, when `view` is called with `version` on a file, and SHALL return an error result naming the file's available version range when the version does not exist.

#### Scenario: View an old version
- **WHEN** `view` targets a file with `version: 2`
- **THEN** the result contains that version's recorded content with 1-based line numbers

#### Scenario: View a nonexistent version
- **WHEN** `view` targets a file with a `version` greater than its latest record
- **THEN** the result is an error result naming the file's available version range

#### Scenario: Version on a directory is rejected
- **WHEN** `view` targets a directory with `version: 2`
- **THEN** the result is an error result explaining `version` applies to files only

### Requirement: Tool description documents versioning
The `memory` tool's description SHALL explain that file changes are recorded as versions, that `history` shows a file's change timeline, that `view` with `version` reads past content, and that `restore` reverts a file to a past version by recording a new version.

#### Scenario: Versioning guidance present
- **WHEN** the tool list is returned
- **THEN** the `memory` tool's description explains `history`, `view` with `version`, and `restore`
