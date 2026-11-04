# Design

## Context

`src/main.rs` currently hardcodes the whole startup: resolve root → ensure root → `Memorija::new(root).serve(rmcp::transport::stdio())`. There is no argument parsing anywhere; `src/args.rs` is the memory tool's *input* schema, unrelated to CLI arguments. The crate has no CLI dependency yet. The transport surface of `rmcp` 3.8 is enabled feature-by-feature; only `transport-io` (stdio) is on today.

## Goals / Non-Goals

**Goals:**
- A CLI structure that scales to more commands later without rework (subcommand enum, per-command options).
- `serve` with stdio (default) and Streamable HTTP transports, sharing one startup path.
- Bare invocation stays byte-for-byte compatible with today's behavior.

**Non-Goals:**
- No TLS and no authentication on HTTP (localhost by default; a non-loopback `--host` warns at startup).
- No config file, environment variables, or logging flags.
- No changes to the `memory` tool or memory root semantics.
- No new subcommands beyond `serve` (reserved for future changes).

## Decisions

**D1: Parse arguments with `clap` (derive API).**
Alternative: hand-rolled parsing over `std::env::args()`. Rejected: the request explicitly expects more commands over time; clap gives usage/help text, mutually-exclusive flags, value validation, and non-zero exits on usage errors for free, and it never touches the filesystem on parse failure (spec: usage errors don't create the memory root). `clap` with `default-features = false, features = ["std", "help", "usage", "error-context", "derive"]` keeps the binary lean.

Structure:

```rust
#[derive(Parser)]
struct Cli { #[command(subcommand)] command: Option<Command> } // None → Serve::default()

#[derive(Subcommand)]
enum Command { Serve(ServeArgs) }

#[derive(Args)]
struct ServeArgs {
    #[arg(long)] http: bool,
    #[arg(long, requires = "http")] host: Option<IpAddr>, // default 127.0.0.1
    #[arg(long, requires = "http")] port: Option<u16>,
}
```

- clap auto-registers the `help` subcommand (with `help <command>`) whenever subcommands exist, which covers the help requirement as long as it is not disabled via `disable_help_subcommand`; keep it enabled and cover it in tests.

- `--stdio` is the absence of `--http`, so it needs no field; it exists only in help/usage. Model it as a `#[arg(long, conflicts_with = "http")] stdio: bool` that is parsed-and-ignored, or via clap's value semantics — implementation may pick either as long as `--stdio --http` errors. Recommend the explicit flag with `conflicts_with` so the error message names the conflict itself.
- `requires = "http"` on `--port` yields the "`--port` applies to `--http`" usage error.

**D2: Transport selection in `main`, one startup function.**
Extract a `run_server(root, transport)` (new `src/serve.rs` or in `main.rs`) that resolves + ensures the root and dispatches to the transport. stdio path is today's code unchanged; HTTP path constructs the rmcp HTTP service. Transport as a plain enum `Transport::{Stdio, Http { port }}` derived from `ServeArgs`.

**D3: HTTP via rmcp's Streamable HTTP server support.**
Enable the `transport-streamable-http-server` feature of `rmcp` 3.8 (pulls in `axum` as a transitive dependency). Use `StreamableHttpService::new(|| Ok(Memorija::new(root.clone())), …)` mounted in a minimal axum router at `/mcp`, bound to `host:port` from the CLI (default `127.0.0.1:8080`). Alternatives considered: hand-writing a JSON-RPC-over-HTTP bridge (rejected — duplicates protocol work rmcp already ships, including session management and SSE streaming); bare-tcp custom HTTP (rejected outright). A per-connection `Memorija::new` is fine: the struct is a stateless `PathBuf` wrapper, matching how stdio serves.

**D4: Startup order preserved — resolve/ensure root after argument parsing, before serving.**
Parsing happens first and only touches memory (satisfies the cli spec's "usage errors do not touch the memory root"); root resolution/ensuring stays where it is now, so both transports share identical root behavior (satisfies the mcp-http-transport spec's shared-root requirement).

**D5: Default subcommand via `Option<Command>`.**
`None` maps to `Command::Serve(ServeArgs::default())` (stdio). This keeps bare `memorija` equivalent to `serve --stdio` without hidden aliases, and unknown subcommands still fail with a normal clap usage error.

## Risks / Trade-offs

- [HTTP is a new network surface] → Bind to `127.0.0.1` by default, no auth yet; `--host` can widen the bind (e.g. `0.0.0.0`), so the server prints a stderr warning whenever it binds a non-loopback address; TLS/auth explicitly non-goals.
- [`transport-streamable-http-server` pulls axum and its tree] → Compile-time cost only; features are additive and stdio builds gain nothing at runtime.
- [rmcp 3.8 Streamable HTTP API shape may differ slightly from the sketch above] → Tasks treat the exact constructor/router wiring as the first implementation step to verify; the behavior contract lives in the spec, not the API sketch.
- [Conflict error UX depends on clap] → `conflicts_with` produces a named-conflict message; verify its wording in the tests for `--stdio --http`.

## Migration Plan

No data or config migration. Ship as one release: bare invocation and existing stdio client configs are unchanged; `--http` is opt-in. Rollback is a plain revert.

## Open Questions

None.
