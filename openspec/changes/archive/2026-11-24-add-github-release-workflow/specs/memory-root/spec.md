# Spec Delta

## ADDED Requirements

### Requirement: Home directory resolution across platforms

The server SHALL locate the user's home directory from `HOME` when that variable is set, and otherwise from `USERPROFILE`, so the memory root is found on Windows where `HOME` is not normally defined. When neither variable is set, the server SHALL fail to start with an error result naming the missing home directory.

#### Scenario: HOME takes precedence

- **WHEN** the server starts with `HOME` set and `USERPROFILE` also set
- **THEN** the memory root is under `HOME`, as on macOS and Linux today

#### Scenario: Windows home without HOME

- **WHEN** the server starts on Windows with `HOME` unset and `USERPROFILE` set
- **THEN** the memory root is `<USERPROFILE>/.memorija/memories` and the server starts normally

#### Scenario: No home variable at all

- **WHEN** the server starts with neither `HOME` nor `USERPROFILE` set
- **THEN** it exits with an error naming the missing home directory and creates no memory root
