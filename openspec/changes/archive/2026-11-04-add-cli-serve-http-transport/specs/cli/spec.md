# Spec Delta

## Purpose

Defines the `memorija` binary's command-line interface: the `serve` subcommand that starts the MCP server, its transport flags, the host and port options, the bare-invocation default, and how invalid invocations are reported. Designed so further commands can be added later.

## ADDED Requirements

### Requirement: Serve subcommand starts the MCP server
The binary SHALL accept a `serve` subcommand that starts the MCP server and blocks until the server stops.

#### Scenario: Serve with explicit stdio
- **WHEN** the binary is invoked as `memorija serve --stdio`
- **THEN** the MCP server starts over stdio and runs until stdin closes or the process is stopped

#### Scenario: Serve with no transport flag
- **WHEN** the binary is invoked as `memorija serve` with no transport flag
- **THEN** the server starts over stdio, identically to `memorija serve --stdio`

### Requirement: Bare invocation keeps stdio behavior
The binary SHALL treat an invocation with no subcommand and no arguments as `serve --stdio`, so existing MCP client configurations that launch the binary directly continue to work.

#### Scenario: Existing client config unaffected
- **WHEN** an MCP client launches the bare binary with no arguments over stdio
- **THEN** the server starts over stdio exactly as before this interface existed

#### Scenario: Unknown subcommand is rejected
- **WHEN** the binary is invoked with an unrecognized subcommand, such as `memorija flush`
- **THEN** the process exits non-zero with a usage error naming the invalid subcommand, and no server starts

### Requirement: HTTP transport flag
The binary SHALL accept a `--http` flag on `serve` that starts the MCP server over Streamable HTTP instead of stdio, and SHALL accept a `--port <number>` option that sets the port the HTTP server listens on, defaulting to 8080.

#### Scenario: Start HTTP server
- **WHEN** the binary is invoked as `memorija serve --http`
- **THEN** the MCP server starts over Streamable HTTP listening on the default port 8080 and runs until stopped

#### Scenario: Custom port
- **WHEN** the binary is invoked as `memorija serve --http --port 9000`
- **THEN** the MCP HTTP server listens on port 9000

#### Scenario: Port has no effect on stdio
- **WHEN** the binary is invoked as `memorija serve --stdio --port 9000`
- **THEN** the invocation is a usage error reported with a message explaining `--port` applies to `--http`

### Requirement: HTTP host option
The binary SHALL accept a `--host <address>` option on `serve` that sets the network address the HTTP server binds to, defaulting to `127.0.0.1`, and SHALL reject `--host` when `--http` is not given.

#### Scenario: Custom bind address
- **WHEN** the binary is invoked as `memorija serve --http --host 0.0.0.0`
- **THEN** the MCP HTTP server listens on all interfaces on the configured port and a warning is printed explaining the server has no authentication

#### Scenario: Host has no effect on stdio
- **WHEN** the binary is invoked as `memorija serve --stdio --host 127.0.0.1`
- **THEN** the invocation is a usage error reported with a message explaining `--host` applies to `--http`

#### Scenario: Invalid host is rejected
- **WHEN** the binary is invoked with a `--host` value that is not an IP address
- **THEN** the process exits non-zero with a usage error naming the invalid value, and no server starts

### Requirement: Transport flags are mutually exclusive
The binary SHALL reject an invocation that passes both `--stdio` and `--http` with a usage error and non-zero exit code.

#### Scenario: Both transport flags
- **WHEN** the binary is invoked as `memorija serve --stdio --http`
- **THEN** the process exits non-zero with an error message explaining that `--stdio` and `--http` cannot be combined, and no server starts

### Requirement: Help subcommand
The binary SHALL accept a `help` subcommand that prints usage information to the standard output and exits with code zero: `memorija help` SHALL print the overall usage listing the available subcommands, and `memorija help <command>` SHALL print the usage of that specific command.

#### Scenario: Overall help
- **WHEN** the binary is invoked as `memorija help`
- **THEN** the usage is printed listing `serve` among the subcommands, and the exit code is zero

#### Scenario: Help for a command
- **WHEN** the binary is invoked as `memorija help serve`
- **THEN** the usage of `serve` is printed, including its `--stdio`, `--http`, `--host`, and `--port` options, and the exit code is zero

#### Scenario: Unknown help topic
- **WHEN** the binary is invoked as `memorija help flush`
- **THEN** the process exits non-zero with a message naming `flush` as an unknown command

### Requirement: Invalid port is rejected
The binary SHALL reject a `--port` value that is not a valid TCP port number (1–65535) with a usage error and non-zero exit code.

#### Scenario: Out-of-range port
- **WHEN** the binary is invoked as `memorija serve --http --port 70000`
- **THEN** the process exits non-zero with an error naming the invalid port, and no server starts

### Requirement: Usage errors do not touch the memory root
The binary SHALL perform no filesystem changes — in particular, no memory root creation or seeding — when argument parsing fails.

#### Scenario: Usage error leaves memory root alone
- **WHEN** the binary is invoked with invalid arguments on a machine where the memory root does not exist
- **THEN** the process exits non-zero and the memory root still does not exist
