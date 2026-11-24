# Design

## Context

See `proposal.md` — Why. Facts that shape the approach:

- The repository has no `.github/` directory yet; this change creates the first
  workflows. Remote is `github.com/kstrempel/memorija` and the default branch is `main`.
- The crate is edition 2024 (`Cargo.toml`), so it needs Rust ≥ 1.85; there is no
  `rust-toolchain.toml`, and the local toolchain is 1.99.1.
- `Cargo.lock` is committed, so every workflow can and should use `--locked`.
- `build.rs` stamps `BUILD_DATE` into `--version`; it honours `SOURCE_DATE_EPOCH`, but
  this change does not set it, so a release binary reports the day it was built.
- Dependencies are pure Rust plus `axum`/`tokio` and `rmcp` over stdio and HTTP; nothing
  is platform-specific, so all four targets compile from a single codebase.
- `src/root.rs::resolve()` reads only `HOME`. The integration tests in `tests/` pass
  `HOME` explicitly to the spawned binary, so they already work on Windows; only the
  default resolution is Windows-hostile.
- Three unit tests create Unix symlinks (`src/ops.rs`, `src/web/mod.rs`), which makes
  `cargo test` fail to compile on Windows.
- Rust unit tests in `src/ops.rs` and `src/web/mod.rs` treat the memory root as a plain
  temp directory, so no test needs the real `~/.memorija`.

## Goals / Non-Goals

**Goals:**

- One CI workflow that fails a change when formatting, lints, tests, or any supported
  target's build break — with the failing platform identified by job name.
- One release workflow where pushing `v<version>` produces four archives and a
  `SHA256SUMS` file on a GitHub Release, and a tag that disagrees with `Cargo.toml`
  fails loudly and publishes nothing.
- The released Windows binary runs on a stock Windows install, and the Windows job runs
  the test suite rather than only compiling it.

**Non-Goals:**

- Publishing to crates.io, Homebrew, winget, or an apt repository.
- Code signing, notarization, or Windows Authenticode — archives are unsigned and the
  README says so.
- Additional targets (Linux aarch64/musl, macOS universal binaries), Docker images,
  SBOMs, or coverage reporting.
- Pinning an exact Rust version or adding `rust-toolchain.toml`; `stable` is tracked.
- A release-draft approval gate: the release is created and filled in one job.

## Decisions

### D1: Two workflows, `ci.yml` and `release.yml`

Verification and publishing have different triggers (`push`/`pull_request` vs `tag`),
different permissions (`contents: read` vs `contents: write`), and different failure
consequences. Splitting them keeps the write permission off the path that runs on every
pull request, which is the point of least privilege.

Alternatives considered: one workflow branching on `startsWith(github.ref, 'refs/tags/')`
(one file, but the release permissions and tag-only steps sit in the same graph as PR
code); a reusable workflow called from both (adds a third file and an abstraction for two
consumers).

Both files carry `concurrency` groups that cancel superseded runs: `ci-${{ github.ref }}`
with `cancel-in-progress: true`, and `release-${{ github.ref }}` with
`cancel-in-progress: false` so a partially uploaded release is never abandoned mid-flight.

### D2: `ubuntu-22.04` for the Linux artifact

The Linux binary links glibc, so the runner's glibc version becomes the minimum users
need. `ubuntu-22.04` gives glibc 2.35, which runs on every current mainstream
distribution; `ubuntu-latest` (24.04, glibc 2.39) would lock out Debian 12, Ubuntu 22.04
and RHEL 9. Static musl builds are the usual alternative and are explicitly out of scope.

Alternatives considered: `ubuntu-latest` (simpler, but a higher glibc floor);
`cargo-zigbuild`/musl (statically linked, but a new toolchain dependency and a second
build path to debug).

### D3: One matrix of four targets, shared by CI and release

Targets, runners and cross-compilation:

| Target | Runner | Notes |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | `ubuntu-22.04` | native |
| `aarch64-apple-darwin` | `macos-latest` | native (arm64 runner) |
| `x86_64-apple-darwin` | `macos-latest` | cross-built; the Apple SDK ships both slices |
| `x86_64-pc-windows-msvc` | `windows-latest` | native; MSVC linker preinstalled |

Keeping the matrix identical in both workflows means CI builds exactly what a release
will build, so a target cannot rot between releases. Each matrix entry is its own job, so
GitHub names the failing platform in the checks list (spec: "a platform-only break is
caught").

Alternatives considered: `macos-13` for the Intel build (a whole extra runner generation
for a cross-compile `rustup target add` already handles); a `fail-fast: false` plus
`continue-on-error` on some targets (would let a broken platform pass).

### D4: Checks split by cost — `fmt` and `clippy` once, `test` on three platforms

- `fmt` job (ubuntu): `cargo fmt --all -- --check`.
- `clippy` job (ubuntu): `cargo clippy --all-targets --locked -- -D warnings`.
- `test` matrix over `ubuntu-22.04`, `macos-latest`, `windows-latest`: `cargo test --locked`.
- `build` matrix (D3): `cargo build --release --locked --target <target>`.

Formatting and linting are platform-independent, so running them three times buys
nothing. Tests, however, are where platform differences surface, and the Windows test
run is a spec requirement, so they run on all three.

Alternatives considered: everything in one matrix job per platform with `if:` guards per
step (fewer jobs, longer critical path, and a failure reads less clearly); running clippy
per target (catches `cfg`-specific lints but triples the slowest job for little gain).

### D5: `dtolnay/rust-toolchain` and `Swatinem/rust-cache`

`dtolnay/rust-toolchain@stable` with a `targets:` input installs the toolchain and any
cross target in one step; `Swatinem/rust-cache@v2` caches `~/.cargo` and `target` keyed
on the lockfile and target triple, which is the difference between a three-minute and a
ten-minute matrix.

These are third-party actions (this is a github.com repository, not GHES, so the usual
internal action restrictions do not apply). Both are widely used and version-tagged.

Alternatives considered: `actions-rust-lang/setup-rust-toolchain` (comparable; also
installs cargo-binstall, unused here); hand-rolled `actions/cache@v4` with explicit
paths and a `Cargo.lock`-hash key (no third party, but more YAML to keep correct,
including the Windows `target`-path quirks); no cache (acceptable for a small crate but
the dependency tree is ~200 crates).

### D6: `gh` CLI for the release, not a release action

The release job runs on `ubuntu-latest`, downloads all four build artifacts into one
directory, writes `SHA256SUMS`, then uses the preinstalled GitHub CLI:
`gh release view "$TAG" || gh release create "$TAG" --title … --notes …`, followed by
`gh release upload "$TAG" … --clobber`.

`gh` is first-party and already on the runner, so there is nothing to pin; `--clobber`
makes a re-run of the job idempotent, which is what a maintainer retrying a failed
upload needs. Checksums are computed by one `sha256sum` step instead of three
platform-specific ones, because every archive is in one place by then.

Alternatives considered: `softprops/action-gh-release` (concise, but a third-party
action with write permission and its own glob semantics); `gh api` with hand-built JSON
(more code for the same result).

### D7: Archives are `tar.gz`, except Windows which is `zip`

`tar -czf memorija-<version>-<target>.tar.gz -C <dir> memorija README.md LICENSE` for the
three Unix targets; `Compress-Archive` on Windows producing the same layout as a `.zip`,
because Windows Explorer extracts zip natively (spec: "extractable with built-in tools").
Each archive extracts to files at the top level, not a nested directory, so the binary
is immediately usable after extraction.

Alternatives considered: `.tar.gz` everywhere (a Windows user needs 7-Zip or `tar.exe`,
which exists on Windows 10+ but is not what a double-click gets); per-target `.zip`
(consistent but second-class on Unix); a bare binary (no README, no LICENSE, no
versioning in the name).

### D8: Home resolution and test gating in the crate

`src/root.rs::resolve()` becomes:

```rust
let home = std::env::var_os("HOME")
    .or_else(|| std::env::var_os("USERPROFILE"))
    .ok_or_else(|| std::io::Error::other("neither HOME nor USERPROFILE is set"))?;
```

`HOME` keeps precedence on purpose: macOS and Linux behaviour must not change, and the
integration tests rely on `HOME` pointing at their temp directory even if `USERPROFILE`
happens to be set. The error message names both variables so a user with an unusual
environment knows what to set. No new dependency (`dirs`/`home`) is warranted for two
`var_os` calls.

The three symlink tests get `#[cfg(unix)]`: they assert Unix symlink containment, which
has no Windows equivalent here, so excluding them is correct rather than a workaround.
Any further Windows-only test failure found in the first run is fixed as a test
assumption (for example file-lock release timing) and never by weakening product
behaviour.

Alternatives considered: the `home` crate (handles edge cases such as Windows roaming
profiles, but adds a dependency for a two-line rule); `#[cfg(windows)]`-only fallback
(same behaviour, more code paths to read).

## Risks / Trade-offs

- **Windows surfaces test failures that cannot be reproduced locally** → the design
  keeps the product behaviour fixed and treats the first Windows run as a debugging
  task with its own task item, not a green-build precondition. If a failure proves to be
  a genuine platform limitation of a feature, split it out into its own change rather
  than disabling the test.
- **A tag pushed from a commit that failed CI publishes anyway** → the release workflow
  builds every target itself and fails on mismatch, so a broken tag produces no release.
- **Runner image changes (glibc, MSVC, Xcode) silently alter artifacts** → pin
  `ubuntu-22.04` and accept floating `-latest` for macOS/Windows; note in the README
  that archives are unsigned and built on hosted runners.
- **Cross-compiled Intel macOS binary is never executed by CI** → it is exercised by the
  arm64 test job and by the x86_64 build; running it would need an Intel runner. Accepted.
- **`macos-latest` is arm64, so a change relying on an Intel-only quirk would slip** →
  the target matrix still builds `x86_64-apple-darwin`; accepted residual risk.
- **Tag/version drift** (`v0.8.1` tag on a `0.8.0` crate) → D6's release job compares
  `cargo metadata`'s version with `${GITHUB_REF_NAME#v}` and fails before creating a
  release.
- **Release notes claim targets that are not attached** → the upload step names every
  archive explicitly and runs after all build jobs succeed (`needs: build`), so a
  partial matrix never reaches the upload.
- **Windows `USERPROFILE` fallback could surprise a Unix user who sets it** → `HOME`
  wins, and the new scenario in `memory-root` pins that precedence.
- **Caches grow and go stale** → `Swatinem/rust-cache` prunes by default and keys on
  `Cargo.lock`; a poisoned cache is recoverable by changing the key prefix.

## Migration Plan

Nothing to migrate: the workflows are additive files, `HOME` behaviour is unchanged, and
the symlink tests are unaffected on Unix.

Rollback: delete the two workflow files (and revert the `root.rs` fallback) with no data
or state to unwind. Tags created by a failed release attempt can be deleted and re-pushed
once fixed; `gh release view`/`--clobber` make a re-run safe on a release that already
exists.

After the first successful tagged release, `README.md` gains the download table; per
`AGENTS.md`, the same step bumps `Cargo.toml` (minor, for the new capability) and folds
the install/CI notes into README's Usage and Development sections at archive time.
