# Spec Delta

## ADDED Requirements

### Requirement: Version flag
The binary SHALL accept a `--version` flag that prints the tool name, the crate version (`0.1.0` at the time of this change), and the build date, and exits with code zero without starting the server.

#### Scenario: Print version
- **WHEN** the binary is invoked as `memorija --version`
- **THEN** the output contains the tool name `memorija`, the version `0.1.0`, and a build date, and the exit code is zero

#### Scenario: Version does not start the server
- **WHEN** the binary is invoked as `memorija --version`
- **THEN** the process prints the version information and exits without starting the MCP server

### Requirement: Build date in version output
The binary SHALL embed the UTC date the binary was built at compile time and include it in the `--version` output, so different builds of the same version can be told apart.

#### Scenario: Build date is present
- **WHEN** the binary is invoked as `memorija --version`
- **THEN** the output includes a build date in `YYYY-MM-DD` form

## MODIFIED Requirements

### Requirement: Help subcommand
The binary SHALL accept a `help` subcommand that prints information about the tool to the standard output and exits with code zero: `memorija help` SHALL print the tool's purpose, its version and build date, and the overall usage listing the available subcommands; `memorija help <command>` SHALL print the usage of that specific command.

#### Scenario: Overall help
- **WHEN** the binary is invoked as `memorija help`
- **THEN** the output includes the tool's purpose, the version `0.1.0` with the build date, and the usage listing `serve` among the subcommands, and the exit code is zero

#### Scenario: Help for a command
- **WHEN** the binary is invoked as `memorija help serve`
- **THEN** the usage of `serve` is printed, including its `--stdio`, `--http`, `--host`, and `--port` options, and the exit code is zero

#### Scenario: Unknown help topic
- **WHEN** the binary is invoked as `memorija help flush`
- **THEN** the process exits non-zero with a message naming `flush` as an unknown command
