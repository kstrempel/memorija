# memory-tool Specification

## Purpose

Defines the `memory` MCP tool: a single unified interface through which MCP clients store, read, and organize persistent memory files. The tool's schema and command semantics replicate Anthropic's memory tool design so the interface is familiar to models across clients.

## Requirements

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

### Requirement: Per-command argument validation
The server SHALL validate that the optional fields required by the selected command are present and SHALL reject the call with a tool result that explains which field is missing when validation fails.

#### Scenario: Create without file_text
- **WHEN** the tool is called with `command: "create"` and no `file_text`
- **THEN** the result is an error result whose message names `file_text` as required for `create`

#### Scenario: str_replace without old_str
- **WHEN** the tool is called with `command: "str_replace"` and no `old_str`
- **THEN** the result is an error result whose message names `old_str` as required for `str_replace`

#### Scenario: Rename without new_path
- **WHEN** the tool is called with `command: "rename"` and no `new_path`
- **THEN** the result is an error result whose message names `new_path` as required for `rename`

### Requirement: Unknown command values are rejected
The server SHALL return an error result when `command` is not one of the six supported values.

#### Scenario: Unsupported command
- **WHEN** the tool is called with `command: "copy"`
- **THEN** the result is an error result listing the supported commands

### Requirement: Path resolution against the memory root
The server SHALL interpret `path` relative to the memory root, SHALL accept a leading `/memories` prefix and strip it, and SHALL treat `/memories` alone (or an empty path) as the root directory itself.

#### Scenario: Prefixed path
- **WHEN** the tool is called with `path: "/memories/notes.txt"`
- **THEN** the operation targets `notes.txt` directly inside the memory root

#### Scenario: Bare root path
- **WHEN** the tool is called with `command: "view"` and `path: "/memories"`
- **THEN** the operation targets the memory root directory itself

### Requirement: View shows files with numbered lines
The server SHALL return file contents with each line prefixed by its 1-based line number when `view` targets a text file, and SHALL honor an optional `view_range` of two integers where a negative end value means "through the end of the file".

#### Scenario: View whole file
- **WHEN** `view` targets an existing text file without `view_range`
- **THEN** the result contains the full file content with 1-based line numbers

#### Scenario: View with range
- **WHEN** `view` targets a text file with `view_range: [5, -1]`
- **THEN** the result contains only lines 5 through the end of the file, numbered with their true line numbers

#### Scenario: Range beyond file end
- **WHEN** `view` targets a text file with `view_range` whose start exceeds the file's line count
- **THEN** the result is an error result reporting the file's actual line count

### Requirement: View shows directory listings
The server SHALL return a recursive listing of the directory when `view` targets a directory, annotating each file with its line count and each directory as a branch in the tree, and SHALL ignore `view_range` in this case.

#### Scenario: Directory listing
- **WHEN** `view` targets a directory containing two files and one subdirectory with a file
- **THEN** the result shows the tree of all three files and the subdirectory, with line counts for each file

#### Scenario: view_range ignored for directories
- **WHEN** `view` targets a directory with `view_range: [1, 5]`
- **THEN** the full recursive listing is returned without error

### Requirement: Create writes or overwrites files
The server SHALL write `file_text` to the target path when `create` is invoked, overwriting any existing file at that path, and SHALL create missing parent directories.

#### Scenario: Create in nested directory
- **WHEN** `create` is called with `path: "/memories/projects/alpha/notes.txt"` where `projects` does not exist
- **THEN** `notes.txt` exists with the given content and `projects/alpha` was created

#### Scenario: Create overwrites existing file
- **WHEN** `create` is called for a path that already exists as a file
- **THEN** the file's content is replaced entirely by `file_text` with no error

### Requirement: Targeted replacement requires a unique match
The server SHALL replace `old_str` with `new_str` when `str_replace` finds exactly one occurrence of `old_str` in the target file; SHALL fail with an error result reporting the match count when `old_str` occurs more than once; and SHALL fail with an error result when `old_str` occurs zero times. Omitting `new_str` SHALL delete the matched text.

#### Scenario: Unique replacement
- **WHEN** `str_replace` targets a file where `old_str` appears exactly once
- **THEN** the file contains `new_str` in place of `old_str`

#### Scenario: Ambiguous replacement fails
- **WHEN** `str_replace` targets a file where `old_str` appears twice and `new_str` is provided
- **THEN** the file is unchanged and the result is an error result reporting 2 matches and advising more surrounding context

#### Scenario: Replacement as deletion
- **WHEN** `str_replace` is called with an `old_str` that appears once and no `new_str`
- **THEN** the matched text is removed from the file

### Requirement: Insert places text after a line index
The server SHALL insert `insert_text` after line `insert_line` when `insert` is invoked, treating `insert_line: 0` as the very beginning of the file, and SHALL fail with an error result when `insert_line` is greater than the file's current line count.

#### Scenario: Insert at top of file
- **WHEN** `insert` is called with `insert_line: 0` on an existing file
- **THEN** the inserted text appears as the file's first line(s) and the previous first line follows it

#### Scenario: Insert after last line
- **WHEN** `insert` is called with `insert_line` equal to the file's current line count
- **THEN** the inserted text is appended after that last line

#### Scenario: Insert beyond end fails
- **WHEN** `insert` is called with `insert_line` greater than the file's line count
- **THEN** the file is unchanged and the result is an error result reporting the file's line count

### Requirement: Delete removes files and directories recursively
The server SHALL remove the target file, or the target directory with all of its contents, when `delete` is invoked on an existing path, and SHALL fail with an error result when the target does not exist.

#### Scenario: Delete a directory tree
- **WHEN** `delete` targets a directory containing nested files
- **THEN** the directory and all nested contents no longer exist

#### Scenario: Delete missing path fails
- **WHEN** `delete` targets a path that does not exist
- **THEN** the result is an error result naming the missing path

### Requirement: Rename moves or renames files and directories
The server SHALL move the target to `new_path` when `rename` is invoked, creating missing parent directories of the destination, and SHALL fail with an error result if the destination already exists.

#### Scenario: Rename a file
- **WHEN** `rename` is called with a file path and a `new_path` that does not exist
- **THEN** the file exists at `new_path` with unchanged content and no longer exists at the original path

#### Scenario: Rename onto existing destination fails
- **WHEN** `rename` is called with a `new_path` that already exists
- **THEN** neither path is modified and the result is an error result naming the existing destination

### Requirement: Recoverable failures are tool results, not protocol errors
The server SHALL report operation failures (missing paths, ambiguous replacements, rejected paths, validation problems) as tool results marked as errors with a human-readable explanation, reserving protocol-level errors for malformed requests the tool layer cannot interpret.

#### Scenario: Model can self-correct after failure
- **WHEN** any operation fails for a reason stated in its error result
- **THEN** the calling model receives the explanation as a normal tool result and can retry with corrected arguments in the next turn

### Requirement: Tool description instructs proactive recall
The `memory` tool's description SHALL instruct the model to view the memory root at the start of a session and read relevant memories before working on any non-trivial task, and SHALL direct recall of memories before answering questions about the user's past work, preferences, or projects.

#### Scenario: Recall guidance present
- **WHEN** an MCP client lists the server's tools
- **THEN** the `memory` tool's description instructs viewing the memory root for non-trivial tasks

#### Scenario: Trivial exchanges are excluded from recall
- **WHEN** the description is read
- **THEN** it scopes recall to non-trivial tasks rather than requiring it for every exchange

### Requirement: Tool description instructs silent capture
The `memory` tool's description SHALL instruct the model to continuously record durable, work-relevant facts about the user and their work — role, preferences and conventions, projects, decisions and their rationale, people, systems, and open threads — as a background action taken without asking the user and without announcing the record in its reply.

#### Scenario: Capture guidance is silent by design
- **WHEN** the description is read
- **THEN** it directs that recording a memory never becomes part of the reply to the user

#### Scenario: Durable fact in a trivial exchange
- **WHEN** the description is read
- **THEN** it directs that a durable fact is recorded even when it surfaces during a trivial exchange

### Requirement: Tool description excludes sensitive data
The `memory` tool's description SHALL instruct the model never to store secrets, credentials, or sensitive personal data, and to skip one-off task details.

#### Scenario: Secrets exclusion present
- **WHEN** the description is read
- **THEN** it explicitly forbids storing secrets and credentials

### Requirement: Tool description defines the memory layout
The `memory` tool's description SHALL specify a directory convention of `about/` for facts about the user, `projects/<name>/` for per-project knowledge, and `sessions/YYYY-MM.md` for a brief monthly running log of work done, and SHALL direct the model to update existing memories instead of duplicating them.

#### Scenario: Layout convention present
- **WHEN** the description is read
- **THEN** it names the `about/`, `projects/<name>/`, and `sessions/YYYY-MM.md` conventions

#### Scenario: Update over duplicate
- **WHEN** the description is read
- **THEN** it directs updating existing memory files rather than creating duplicates

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
