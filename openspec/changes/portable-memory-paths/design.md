# Design

## Context

See `proposal.md` — Why, and `specs/` for the contract.

Two places produce a path relative to the memory root, and both now go through
`paths::key_string`, which joins the path's components with `/`: `ops::rel_key`
(the key the version store records under) and `ops::walk` (the tree `list` and
`view` print). On Windows that used to be `projects\alpha\notes.txt`; on macOS,
Linux and in every test, spec and URL it is `projects/alpha/notes.txt`. That
producer side is in place, so this change is about everything read back from a
store written before it.

Three places consume a path that was recorded: `versions::decode_key` turns a
store file name back into a path for `list`/`changes`, `inspect::scan_changes`
copies each record's own `path` into a `ChangeRecord`, and
`versions::record_file` encodes a path into the store file name for read and
append. The store layout is one `%`-encoded `.jsonl` file per path, so the
spelling a producer chose is baked into the file name; a Windows store written
before the producer fix already holds `%5C` names whose records carry
backslashes.

`paths::resolve` is not a producer: it splits incoming text with
`Path::components`, which treats both `/` and `\` as separators on Windows, so a
canonical `/` path addresses the same file there without a change.

## Goals / Non-Goals

**Goals:**

- One canonical path form — relative, `/`-separated — from every producer, on
  every platform, without touching `paths::resolve` or the memory root layout.
- A Windows store written before this change stays readable and appendable, with
  no file rewrite and no migration step.
- The four failing Windows tests pass unchanged, because they assert the form the
  specs now require.

**Non-Goals:**

- Rewriting legacy store files or their `path` fields in place.
- Accepting `\` from CLI, MCP or web callers as an alias for `/` (Windows already
  accepts it through `Path::components`).
- Changing how non-ASCII names or spaces are percent-encoded.
- Making the version store shareable between machines; only the key spelling
  becomes stable.

## Decisions

### D1 — Canonicalise at the two producers, in one helper

`paths::key_string`, beside path resolution, turns a path relative to a base
into its `/`-joined spelling from `Path::components` (`Normal` segments only,
`/` between them), and both producers — `rel_key` and `walk` — use it. That is
already in place; this change verifies it end to end (a nested temp tree yields
the same key from both) and leaves the conversion where it is. Building from
components rather than `replace('\\', "/")` on a display string keeps the
conversion explicit and leaves a segment that contains a literal backslash (a
legal Unix file name) as that file's own name instead of pretending it is a
directory boundary.

Alternative rejected: normalising inside `encode_key`. It would make `a/b` and a
Unix file literally named `a\b` encode to the same store file and destroy the
collision-free mapping `encode_key` is built for; the canonical spelling belongs
to the path, not to the encoder.

### D2 — Normalise recorded paths on output, never on disk

`decode_key` normalises a decoded `\` to `/`, and `scan_changes` normalises
`record.path` when it builds a `ChangeRecord`. That makes `list`, `changes`, the
web changes feed and the web file index agree on one spelling for records an
earlier build wrote, while respecting the store's append-only, immutable-record
rule: a record's stored text is never rewritten.

Alternative rejected: a startup migration rewriting every legacy file name and
the `path` inside each record. It buys nothing visible that D2 does not, and it
writes to a store the spec calls immutable.

### D3 — A legacy store file is still the file for its path

`record_file` looks for the canonical name first. When that file does not exist
but the legacy `\`-spelled name does, it returns the legacy path, so `read`
returns the old records and `append` continues that file's sequence instead of
starting a second history at seq 1. The fallback is a lookup only: once the
canonical file exists (a fresh store, or a file first recorded after this
change), it always wins, so a Unix file whose real name contains a backslash
keeps its own store file rather than being shadowed by a Windows artefact.

Alternative rejected: hiding legacy records behind "history starts at this
version" behaviour. Splitting a history is the bug the canonical form removes;
reproducing it for old Windows stores would only move the failure.

### D4 — The web layer changes only through the shared helpers

`file_entries` and `api_file` already call `rel_key` and `versions::read`, and
the changes feed already calls `scan_changes`, so D1–D3 fix the viewer. No route,
template or address format changes: the Reader's `/file?path=…` already carries
`/` paths and resolves them on Windows. One web test is added for the legacy
case, because that is where a mistaken "canonical only" store lookup would show
up as missing version metadata rather than as a wrong string.

### D5 — Verification is local for the conversion, CI for the platform

A macOS test run cannot reproduce the Windows separator, so the unit tests assert
the conversion itself: a nested temp tree yields `/` from `walk`/`rel_key`, a
legacy `%5C` file is read and appended with its sequence continuing, and a
decoded legacy name and a scanned legacy record come out with `/`. The four
Windows tests that failed are the end-to-end check and are left as they are.

## Risks / Trade-offs

- **Rollback leaves the old Windows build unable to see nested history** → the
  old build looks for `%5C` names only. Accepted: the alternative is keeping the
  broken key spelling, and new Windows history from this build would be invisible
  to an older binary in either direction unless a migration ran.
- **A legacy lookup costs one extra existence check when the canonical file is
  missing** → it runs only on a miss, once per read/append of a path with no
  canonical store file, not per listing entry.
- **A tree synced between platforms could hold both spellings of one file's
  store** → the canonical file wins and the legacy one is ignored; the spec's
  "one history per file" holds for the path the tree actually has. Merging two
  divergent stores is out of scope.
- **Records keep their original `path` text on disk** → anything reading the
  JSONL directly sees the old spelling. Records are internal data and every
  supported surface normalises; documenting it in README's Development section
  keeps the next reader from "fixing" it into the store.
- **A Unix file whose name contains a literal backslash** → its canonical
  spelling keeps the backslash inside one segment and its store key is `%5C`,
  which collides with a legacy Windows key only if both stores are merged; that
  collision is inherent to the existing encoding and unchanged by this fix.

## Migration Plan

No data migration and no route or file-format change. Deploy the fix and new
keys are canonical; old keys stay readable and appendable. Rollback restores the
old spelling for new writes and hides nested history written after the fix from a
pre-fix Windows binary.

## Open Questions

None that would change the specs, the approach or the task breakdown.
