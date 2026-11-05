//! The version record store (spec: `memory-versions`).
//!
//! Every mutation of a memory file appends one immutable record
//! `{ path, seq, content, cause, recorded_at }`, keyed per path with a
//! 1-based sequence number. Records are plain data — each stored record maps
//! 1:1 to a future database row — and live in a store directory beside the
//! memory root (`~/.memorija/versions`), never inside the visible memory tree.
//!
//! On-disk layout (design open question, resolved for this first cut): one
//! JSON Lines file per recorded path, one record per line. Appending writes a
//! single line to the end; no record is ever rewritten or removed.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One immutable version of one memory file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// Path of the memory file, relative to the memory root.
    pub path: String,
    /// Per-path 1-based sequence number.
    pub seq: u64,
    /// The file's full content after this change.
    pub content: String,
    /// The command that produced this version (`create`, `str_replace`,
    /// `insert`, or `restore`).
    pub cause: String,
    /// UTC timestamp of the change (`YYYY-MM-DDTHH:MM:SSZ`).
    pub recorded_at: String,
}

/// The version store directory for a memory root: a `versions` directory
/// beside the root (`~/.memorija/memories` → `~/.memorija/versions`).
pub fn store_dir(root: &Path) -> PathBuf {
    match root.parent() {
        Some(parent) => parent.join("versions"),
        // A root without a parent cannot have a sibling store.
        None => root.join(".versions"),
    }
}

/// Append one record for `rel_path` with the next per-path sequence number.
pub fn append(store: &Path, rel_path: &str, content: &str, cause: &str) -> Result<Record, String> {
    let file = record_file(store, rel_path)?;
    let seq = latest_seq(&file)? + 1;
    let record = Record {
        path: rel_path.to_string(),
        seq,
        content: content.to_string(),
        cause: cause.to_string(),
        recorded_at: now_utc(),
    };
    std::fs::create_dir_all(store).map_err(|e| {
        format!(
            "Error: cannot create the version store {}: {e}",
            store.display()
        )
    })?;
    let mut out = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
        .map_err(|e| {
            format!(
                "Error: cannot write the version store {}: {e}",
                file.display()
            )
        })?;
    serde_json::to_writer(&mut out, &record)
        .map_err(|e| format!("Error: cannot serialize the version record: {e}"))?;
    out.write_all(b"\n")
        .and_then(|_| out.flush())
        .map_err(|e| {
            format!(
                "Error: cannot write the version store {}: {e}",
                file.display()
            )
        })?;
    Ok(record)
}

/// All records for `rel_path`, oldest first; empty when the path has none.
pub fn read(store: &Path, rel_path: &str) -> Result<Vec<Record>, String> {
    let file = record_file(store, rel_path)?;
    let raw = match std::fs::read_to_string(&file) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(format!(
                "Error: cannot read the version store {}: {e}",
                file.display()
            ));
        }
    };
    raw.lines()
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(line).map_err(|e| {
                format!(
                    "Error: corrupt version record (line {}) in {}: {e}",
                    index + 1,
                    file.display()
                )
            })
        })
        .collect()
}

/// The store file holding `rel_path`'s records: the relative path,
/// percent-encoded onto a single file name.
fn record_file(store: &Path, rel_path: &str) -> Result<PathBuf, String> {
    if rel_path.is_empty() {
        return Err("Error: the memory root itself has no version records".to_string());
    }
    Ok(store.join(format!("{}.jsonl", encode_key(rel_path))))
}

/// Encode a relative path into one file-name component: `/` becomes `%2F`
/// and every byte outside `[A-Za-z0-9._-]` becomes `%XX`, so the mapping is
/// collision-free.
pub(crate) fn encode_key(rel_path: &str) -> String {
    let mut out = String::with_capacity(rel_path.len());
    for byte in rel_path.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'_' | b'-' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// The inverse of [`encode_key`]: decode a percent-encoded store file name
/// (without its `.jsonl` suffix) back to the recorded relative path.
pub fn decode_key(encoded: &str) -> String {
    let bytes = encoded.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(byte) = std::str::from_utf8(&bytes[index + 1..index + 3])
                .map_err(|_| ())
                .and_then(|hex| u8::from_str_radix(hex, 16).map_err(|_| ()))
            {
                out.push(byte);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The sequence number of a store file's last record, or 0 when it is new.
fn latest_seq(file: &Path) -> Result<u64, String> {
    let raw = match std::fs::read_to_string(file) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => {
            return Err(format!(
                "Error: cannot read the version store {}: {e}",
                file.display()
            ));
        }
    };
    match raw.lines().next_back() {
        None => Ok(0),
        Some(line) => serde_json::from_str::<Record>(line)
            .map(|record| record.seq)
            .map_err(|e| format!("Error: corrupt version record in {}: {e}", file.display())),
    }
}

/// Seconds since the Unix epoch.
pub(crate) fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A seconds-since-the-epoch instant as `YYYY-MM-DDTHH:MM:SSZ`.
pub(crate) fn format_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600,
        (rem % 3_600) / 60,
        rem % 60
    )
}

/// The current time as `YYYY-MM-DDTHH:MM:SSZ`.
fn now_utc() -> String {
    format_utc(now_secs())
}

/// Days since the Unix epoch to `(year, month, day)` in the proleptic
/// Gregorian calendar (Hinnant's `civil_from_days`).
pub(crate) fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::temp_dir;

    fn setup(label: &str) -> PathBuf {
        temp_dir(label)
    }

    #[test]
    fn store_sits_beside_the_memory_root() {
        assert_eq!(
            store_dir(Path::new("/home/tester/.memorija/memories")),
            PathBuf::from("/home/tester/.memorija/versions")
        );
    }

    #[test]
    fn append_then_read_round_trips_the_record() {
        let store = setup("versions-roundtrip");
        let record = append(&store, "notes.txt", "one\n", "create").unwrap();

        assert_eq!(record.path, "notes.txt");
        assert_eq!(record.seq, 1);
        assert_eq!(record.content, "one\n");
        assert_eq!(record.cause, "create");
        assert!(record.recorded_at.ends_with('Z'), "{record:?}");
        assert_eq!(record.recorded_at.len(), 20, "YYYY-MM-DDTHH:MM:SSZ form");

        let records = read(&store, "notes.txt").unwrap();
        assert_eq!(records, vec![record]);
    }

    #[test]
    fn sequences_are_per_path_and_one_based() {
        let store = setup("versions-sequences");
        append(&store, "a.txt", "a1\n", "create").unwrap();
        append(&store, "b.txt", "b1\n", "create").unwrap();
        let second = append(&store, "a.txt", "a2\n", "str_replace").unwrap();

        assert_eq!(second.seq, 2, "sequencing is per path, not global");
        let a = read(&store, "a.txt").unwrap();
        assert_eq!(a.iter().map(|r| r.seq).collect::<Vec<_>>(), [1, 2]);
        let b = read(&store, "b.txt").unwrap();
        assert_eq!(b.iter().map(|r| r.seq).collect::<Vec<_>>(), [1]);
    }

    #[test]
    fn earlier_records_are_immutable_after_later_appends() {
        let store = setup("versions-immutable");
        let first = append(&store, "notes.txt", "one\n", "create").unwrap();
        let second = append(&store, "notes.txt", "two\n", "str_replace").unwrap();
        let third = append(&store, "notes.txt", "three\n", "insert").unwrap();

        let records = read(&store, "notes.txt").unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(&records[0], &first, "first record unchanged");
        assert_eq!(&records[1], &second, "second record unchanged");
        assert_eq!(&records[2], &third);
        assert_ne!(records[0].content, records[2].content);
        assert_eq!(records[0].recorded_at, first.recorded_at);
    }

    #[test]
    fn read_of_unrecorded_path_is_empty() {
        let store = setup("versions-unrecorded");
        append(&store, "a.txt", "a1\n", "create").unwrap();

        assert!(read(&store, "b.txt").unwrap().is_empty());
        assert!(read(&store, "nested/b.txt").unwrap().is_empty());
    }

    #[test]
    fn nested_paths_map_to_distinct_files() {
        let store = setup("versions-nested");
        append(&store, "a/b.txt", "nested\n", "create").unwrap();
        append(&store, "a%2Fb.txt", "tricky\n", "create").unwrap();

        let nested = read(&store, "a/b.txt").unwrap();
        assert_eq!(nested[0].content, "nested\n");
        let tricky = read(&store, "a%2Fb.txt").unwrap();
        assert_eq!(tricky[0].content, "tricky\n");
    }

    #[test]
    fn decode_key_reverses_the_slash_escape() {
        assert_eq!(decode_key("a%2Fb.txt"), "a/b.txt");
        assert_eq!(
            decode_key("a%252Fb.txt"),
            "a%2Fb.txt",
            "a literal %2F in a path is double-escaped"
        );
        assert_eq!(
            decode_key("plain.txt"),
            "plain.txt",
            "unencoded names pass through"
        );
        assert_eq!(decode_key("a%20b.md"), "a b.md", "spaces are decoded");
        assert_eq!(
            decode_key("%C3%BCnicode.md"),
            "ünicode.md",
            "multi-byte UTF-8 reassembles"
        );
        assert_eq!(
            decode_key("broken%ZZ"),
            "broken%ZZ",
            "invalid escapes pass through"
        );
    }

    #[test]
    fn decode_key_round_trips_every_encoded_path() {
        for path in [
            "notes.txt",
            "a/b.txt",
            "a%2Fb.txt",
            "projects/alpha 2026/ünï code.md",
            "sp ace.txt",
            "…ellipsis",
        ] {
            assert_eq!(
                decode_key(&encode_key(path)),
                path,
                "round trip of {path:?}"
            );
        }
    }
}
