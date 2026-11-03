# Spec Delta

## Purpose

Defines the memory root: the local directory the `memory` tool operates on, how it is created and seeded, and the containment rules that keep every operation inside it.

## ADDED Requirements

### Requirement: Memory root location and creation
The server SHALL use `~/.memorija/memories` as its memory root and SHALL create that directory at startup when it does not exist.

#### Scenario: First launch
- **WHEN** the server starts and `~/.memorija/memories` does not exist
- **THEN** the directory is created and the server starts normally

#### Scenario: Existing root is reused
- **WHEN** the server starts and `~/.memorija/memories` already exists with files in it
- **THEN** the existing files are left untouched and remain visible to the `memory` tool

### Requirement: First-launch seeding
The server SHALL create a starter `MEMORY.md` in the memory root when it creates the root for the first time, containing short guidance on what the memory directory is for and how to organize it, and SHALL NOT overwrite an existing `MEMORY.md`.

#### Scenario: Seeding on first launch
- **WHEN** the server creates the memory root because it did not exist
- **THEN** the root contains a `MEMORY.md` with usage guidance

#### Scenario: Existing memory is never reseeded
- **WHEN** the server starts with an existing memory root containing a `MEMORY.md`
- **THEN** that `MEMORY.md` is unchanged

### Requirement: Path containment
The server SHALL refuse, with an error result, any operation whose resolved target would land outside the memory root, including paths containing `..` traversal segments and paths that resolve to the filesystem root or outside the user's home.

#### Scenario: Traversal attempt rejected
- **WHEN** any command is called with a path that resolves outside the memory root, such as `/memories/../../../etc/passwd`
- **THEN** no filesystem change occurs and the result is an error result explaining the path must stay within the memory root

### Requirement: Symlink containment
The server SHALL treat a symbolic link inside the memory root that resolves outside it as an out-of-bounds path and refuse operations that follow it with an error result.

#### Scenario: Escaping symlink rejected
- **WHEN** a symlink inside the memory root points to a file outside it and a command targets that symlink
- **THEN** the operation is refused with an error result and the outside file is untouched

### Requirement: Root itself is protected
The server SHALL refuse `delete` and `rename` operations that target the memory root directory itself with an error result, while still allowing `view` on it.

#### Scenario: Deleting the root fails
- **WHEN** `delete` is called with `path: "/memories"`
- **THEN** the memory root and its contents still exist and the result is an error result

#### Scenario: Renaming the root fails
- **WHEN** `rename` is called with `path: "/memories"` and any `new_path`
- **THEN** the memory root still exists at its original location and the result is an error result
