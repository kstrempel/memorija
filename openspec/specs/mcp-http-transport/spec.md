# mcp-http-transport Specification

## Purpose

Defines serving the MCP server — and with it the existing `memory` tool and memory root — over Streamable HTTP, so MCP clients can connect over HTTP instead of only launching the process over stdio.

## Requirements

### Requirement: MCP over Streamable HTTP
The server SHALL expose the same MCP interface over Streamable HTTP as it does over stdio: an HTTP client SHALL be able to initialize a session, list tools, and call the `memory` tool with identical semantics and results.

#### Scenario: HTTP client reaches the memory tool
- **WHEN** an MCP client connects to the HTTP server's MCP endpoint and calls the `memory` tool with a valid `view` command
- **THEN** the result is identical to what a stdio client would receive for the same arguments

#### Scenario: Tool listing over HTTP
- **WHEN** an MCP client lists the server's tools over the HTTP transport
- **THEN** exactly one tool named `memory` is returned, with the same schema as over stdio

### Requirement: HTTP server binds to the configured address
The HTTP server SHALL bind to the host address and port configured through the command line (defaults `127.0.0.1` and 8080), SHALL print a warning to the standard error when the configured host is not a loopback address, and SHALL refuse to start with a non-zero exit code and a clear message when the address cannot be bound.

#### Scenario: Bind failure reported clearly
- **WHEN** the configured address is already in use by another process
- **THEN** the process exits non-zero with an error message naming the address, rather than hanging or failing silently

#### Scenario: Non-loopback bind warns
- **WHEN** the server starts with `--host 0.0.0.0`
- **THEN** a warning is printed to the standard error before serving, noting the server has no authentication

### Requirement: Memory root is shared across transports
The HTTP server SHALL operate on the same memory root as the stdio server, creating and seeding it on first launch under the same rules.

#### Scenario: First launch over HTTP
- **WHEN** the server starts with `--http` on a machine where the memory root does not exist
- **THEN** the memory root is created and seeded exactly as on first stdio launch
