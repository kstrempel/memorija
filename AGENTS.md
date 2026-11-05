# AGENTS.md

## What this is

`memorija` — an MCP server giving AI agents persistent file-based memory under `~/.memorija`, with a versioned store. Rust binary crate (edition 2024); entry point `src/main.rs`, CLI in `src/cli.rs`, command implementations in `src/ops.rs`, human-facing inspection commands in `src/inspect.rs`.

## OpenSpec workflow

This repo uses OpenSpec (`openspec/config.yaml`, schema `spec-driven`); skills live in `.agents/skills/`.

- Non-trivial changes go through the OpenSpec flow: `openspec-propose` (plan only) → user approval → `openspec-apply-change` → `openspec-archive-change`.
- When running the propose workflow, do **not** edit project code in the same response — planning artifacts only. Wait for the user to start apply.
- Don't hand-create files under `openspec/`; use the `openspec` CLI.
- On every archive, update `README.md` in the same step: user-facing changes (commands, flags, output shapes, behavior) go into the Usage/Features sections; anything a developer would find interesting (internals, conventions, gotchas) goes into Development. Keep additions brief — `openspec/` holds the full change history.
- On every archive, also bump the crate version in `Cargo.toml`: new features bump the minor version (`0.1.1` → `0.2.0`), bug fixes and other small changes bump the patch (`0.1.1` → `0.1.2`).

## Build & verify

- `cargo build` / `cargo run` / `cargo test` — standard, nothing custom.
- Edition 2024 requires a current toolchain; if the build rejects the edition, `rustup update` before anything else.
- `target/` is gitignored — never commit build artifacts.

## Git

- Keep changes small and self-contained; commit messages are unconstrained so far.
