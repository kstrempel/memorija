# Proposal

## Why

`memorija` is installed today by building from source (`cargo install --path .`), and
nothing verifies that the crate even compiles outside the maintainer's macOS machine.
Making the binary available for macOS, Linux and Windows — and proving each build on
every push — is the missing step before the tool can be handed to anyone else.

The Windows build has a real gap that only shows up once there is a Windows job: the
memory root is resolved from `HOME`, which Windows does not normally set, and three
unit tests use Unix symlinks, so `cargo test` cannot compile on Windows.

## What Changes

- Add a GitHub Actions CI workflow that runs on every push to `main` and every pull
  request: formatting, linting and tests on Linux and macOS, a compiled build for each
  supported target including Windows.
- Add a GitHub Actions release workflow triggered by a `v*` tag: build optimized
  binaries for `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`,
  `x86_64-apple-darwin` and `x86_64-pc-windows-msvc`, package each with the README and
  LICENSE, generate checksums, and attach them to the GitHub Release for that tag.
- Resolve the user's home directory from `USERPROFILE` when `HOME` is not set, so the
  released Windows binary can find `~/.memorija/memories`.
- Gate the symlink-specific unit tests to Unix so the test suite compiles and runs on
  Windows.
- Document the install paths (source build and prebuilt release archives) in
  `README.md`.

## Capabilities

### New Capabilities

- `ci-release`: automated verification of the crate on macOS, Linux and Windows, and
  the release pipeline that publishes tagged, per-platform binaries.

### Modified Capabilities

- `memory-root`: how the user's home directory is found now covers Windows, where
  `HOME` is absent and `USERPROFILE` is the home location.

## Impact

- New files: `.github/workflows/ci.yml`, `.github/workflows/release.yml`.
- `src/root.rs`: home resolution gains the `USERPROFILE` fallback (behaviour change for
  Windows only; `HOME` keeps precedence, so macOS and Linux are untouched).
- `src/ops.rs`, `src/web/mod.rs`: three test functions gain `#[cfg(unix)]`.
- `README.md`: install section gains prebuilt binaries and states the supported targets.
- No new crate dependencies, no API or wire-protocol change.
- Repository settings the maintainer must provide: the `GITHUB_TOKEN` workflow
  permission `contents: write` for the release job. No secrets beyond that.
- Assumption: the release version is the `Cargo.toml` version, and the tag that starts
  a release is `v<that version>`; a mismatch fails the release instead of publishing a
  mislabelled binary.
