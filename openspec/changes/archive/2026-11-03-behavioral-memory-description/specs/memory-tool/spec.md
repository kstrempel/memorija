# Spec Delta

## ADDED Requirements

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
