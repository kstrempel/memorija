# ci-release Specification

## Purpose

Defines how the project proves that it builds and works on every platform it ships for, and how a tagged version becomes downloadable, one-archive-per-platform release binaries.

## Requirements

### Requirement: Every change is verified automatically

The project SHALL run its automated checks on every push to the default branch and on every pull request, and SHALL report the result as a status check on the change.

#### Scenario: Pull request is checked

- **WHEN** a pull request is opened or updated
- **THEN** the automated checks run and their result is reported on that pull request

#### Scenario: Push to the default branch is checked

- **WHEN** a commit is pushed to the default branch
- **THEN** the same automated checks run for that commit

### Requirement: Checks cover formatting, lints, and tests

The automated checks SHALL verify that the code is formatted, that it produces no lint warnings, and that its tests pass.

#### Scenario: Unformatted code fails

- **WHEN** a change leaves the code unformatted
- **THEN** the check fails and names the offending files

#### Scenario: Lint warning fails

- **WHEN** a change introduces a lint warning
- **THEN** the check fails and reports the warning

#### Scenario: Failing test fails

- **WHEN** a change breaks a test
- **THEN** the check fails and reports the failing test

### Requirement: Every supported target is built on every change

The automated checks SHALL compile the crate for every target the project ships, so a change that breaks one platform is caught before release.

#### Scenario: All supported targets compile

- **WHEN** the automated checks run for a change
- **THEN** an optimized binary is built for each supported target and each build is reported separately

#### Scenario: A platform-only break is caught

- **WHEN** a change compiles on one platform but not on another
- **THEN** the check for the failing target fails while the passing targets still report success

### Requirement: Supported targets

The project SHALL ship binaries for `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, and `x86_64-pc-windows-msvc`.

#### Scenario: Target list is complete

- **WHEN** the release artifacts for a version are listed
- **THEN** there is exactly one archive for each supported target, and no other platform is claimed as supported

### Requirement: Windows builds are also tested

The automated tests SHALL run on Windows as well as on macOS and Linux, with only the tests that depend on Unix-only filesystem features excluded from the Windows run.

#### Scenario: Windows test run

- **WHEN** the automated checks run on Windows
- **THEN** the full test suite except the Unix-only tests runs, and the Unix-only tests are skipped rather than failing

#### Scenario: Released Windows binary finds the memory root

- **WHEN** the Windows binary starts on a machine where `HOME` is not set but `USERPROFILE` is
- **THEN** it uses the memory root under `USERPROFILE` instead of failing

### Requirement: A version tag publishes a release

The project SHALL publish a GitHub Release when a tag of the form `v<version>` is pushed, where `<version>` is the crate version, and SHALL refuse to publish when the tag and the crate version disagree.

#### Scenario: Tag matching the crate version releases

- **WHEN** a tag whose version equals the crate version is pushed
- **THEN** a GitHub Release for that tag is created with the built archives attached

#### Scenario: Mismatched tag is refused

- **WHEN** a tag is pushed whose version differs from the crate version
- **THEN** the release job fails without creating a release or attaching artifacts

#### Scenario: Release is complete once the job succeeds

- **WHEN** the release job finishes building
- **THEN** the release exists with every archive and a checksums file attached, and the notes name the version and the supported targets

### Requirement: Release archives are runnable without a build

Each release archive SHALL contain the optimized binary for its target alongside the project's README and LICENSE, and the release SHALL publish a checksums file covering every archive.

#### Scenario: Archive contents

- **WHEN** a user extracts an archive for their platform
- **THEN** the binary is at the top level of the extracted directory and the README and LICENSE are beside it

#### Scenario: Checksums let a download be verified

- **WHEN** the user downloads an archive and the checksums file
- **THEN** the recorded digest for that archive matches the downloaded file

#### Scenario: Windows archive is extractable with built-in tools

- **WHEN** a Windows user downloads the Windows archive
- **THEN** it is a format that Windows Explorer can extract without additional software
