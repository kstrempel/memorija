# AGENTS.md

## What this is

Fresh Rust binary crate (`memorija`, edition 2024, no dependencies yet). Entry point: `src/main.rs`. No tests, CI, or README yet.

## OpenSpec workflow

This repo uses OpenSpec (`openspec/config.yaml`, schema `spec-driven`); skills live in `.agents/skills/`.

- Non-trivial changes go through the OpenSpec flow: `openspec-propose` (plan only) → user approval → `openspec-apply-change` → `openspec-archive-change`.
- When running the propose workflow, do **not** edit project code in the same response — planning artifacts only. Wait for the user to start apply.
- Don't hand-create files under `openspec/`; use the `openspec` CLI.

## Build & verify

- `cargo build` / `cargo run` / `cargo test` — standard, nothing custom.
- Edition 2024 requires a current toolchain; if the build rejects the edition, `rustup update` before anything else.
- `target/` is gitignored — never commit build artifacts.

## Git

- Repo has no commits yet (fresh `master`). Keep changes small and self-contained; commit messages are unconstrained so far.
