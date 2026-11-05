//! Human-facing inspection commands (`list`, `show`, `versions`, `changes`)
//! (spec: `cli-inspection`).
//!
//! All four are read-only: they never create or seed the memory root, never
//! append version records, and start no server. The root is resolved via
//! `root::resolve` (never `root::ensure`); on a machine with no memory root
//! yet the commands report `no memories yet` and exit zero.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use crate::cli;
use crate::ops;
use crate::paths;
use crate::root;
use crate::versions;

/// Run one inspection command; `main` dispatches the non-serve variants here.
/// Errors come back as `Error: ...` strings for stderr with a non-zero exit,
/// mirroring `ops`' message style (design D6).
pub fn run(command: &cli::Command) -> Result<(), String> {
    match command {
        cli::Command::List => {
            let root = resolve_root()?;
            print!("{}", list_output(root.path())?);
            Ok(())
        }
        cli::Command::Show(args) => show(args),
        cli::Command::Versions { file } => {
            let root = resolve_root()?;
            print!("{}", versions_output(root.path(), file)?);
            Ok(())
        }
        cli::Command::Changes { timespan } => {
            let root = resolve_root()?;
            print!("{}", changes_output(root.path(), timespan.as_ref())?);
            Ok(())
        }
        cli::Command::Serve(_) => unreachable!("serve never reaches the inspection layer"),
        cli::Command::Web(_) => unreachable!("web never reaches the inspection layer"),
    }
}

/// The output of `list`: the recursive listing with line counts, or
/// `no memories yet` on a fresh machine.
fn list_output(root: &Path) -> Result<String, String> {
    match inspect_root(root) {
        MemoryRoot::Missing(_) => Ok("no memories yet\n".to_string()),
        MemoryRoot::Present(root) => list(&root),
    }
}

/// The `list` listing: every file and directory under the root, files with
/// their line count and available version range, directories with a trailing
/// slash.
fn list(root: &Path) -> Result<String, String> {
    let last_seqs = available_versions(root)?;
    let mut out = String::new();
    for (line_count, relative) in crate::ops::walk(root, root, root)? {
        if relative.ends_with('/') {
            out.push_str(&format!("{relative}\n"));
        } else {
            // Seq numbers are per-path and contiguous, so the last recorded
            // seq names every available version.
            let versions = last_seqs
                .get(&relative)
                .map(|&last| format_versions(last))
                .unwrap_or_else(|| "-".to_string());
            out.push_str(&format!("{relative}\t{line_count}\t{versions}\n"));
        }
    }
    Ok(out)
}

/// The available version range for one recorded path (`v1` for a single
/// version, `v1..vN` otherwise).
fn format_versions(last: u64) -> String {
    if last == 1 {
        "v1".to_string()
    } else {
        format!("v1..v{last}")
    }
}

/// The last recorded seq per store path, keyed by display path: one read per
/// store file, the same scan shape the `changes` log uses (design D4).
fn available_versions(root: &Path) -> Result<std::collections::HashMap<String, u64>, String> {
    let store = crate::versions::store_dir(root);
    let mut last_seqs = std::collections::HashMap::new();
    if store.is_dir() {
        let mut files: Vec<PathBuf> = std::fs::read_dir(&store)
            .map_err(|e| {
                format!(
                    "Error: cannot read the version store {}: {e}",
                    store.display()
                )
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                format!(
                    "Error: cannot read the version store {}: {e}",
                    store.display()
                )
            })?
            .into_iter()
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "jsonl"))
            .map(|entry| entry.path())
            .collect();
        files.sort();
        for file in files {
            let encoded = file
                .file_stem()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let rel = crate::versions::decode_key(&encoded);
            if let Some(last) = crate::versions::read(&store, &rel)?.last() {
                last_seqs.insert(rel, last.seq);
            }
        }
    }
    Ok(last_seqs)
}

/// The `show` subcommand: print one memory file (or one recorded version),
/// raw or rendered per the TTY gate (design D2).
fn show(args: &cli::ShowArgs) -> Result<(), String> {
    let root = resolve_root()?;
    let root = match root {
        MemoryRoot::Missing(_) => {
            print!("no memories yet");
            return Ok(());
        }
        MemoryRoot::Present(path) => path,
    };
    let content = show_content(&root, args)?;
    if force_raw(args.raw, args.render, std::io::stdout().is_terminal()) {
        print!("{content}");
        if !content.is_empty() && !content.ends_with('\n') {
            println!();
        }
    } else {
        termimad::print_text(&content);
    }
    Ok(())
}

/// Whether `show` prints raw content: `--raw` forces it, `--render` forces
/// rendering, otherwise stdout decides (raw when piped). `--raw` wins when
/// both flags are set (design D2).
fn force_raw(raw: bool, render: bool, stdout_is_terminal: bool) -> bool {
    raw || !(render || stdout_is_terminal)
}

/// The content `show` prints: the live file's content, or the content recorded
/// for the selected version.
fn show_content(root: &Path, args: &cli::ShowArgs) -> Result<String, String> {
    let target = paths::resolve(root, &args.file)?;
    if target.is_root {
        return Err(format!("Error: {} is a directory", args.file));
    }
    let meta = std::fs::symlink_metadata(&target.path).map_err(|_| missing(&args.file))?;
    if meta.is_dir() {
        return Err(format!("Error: The path {} is a directory", args.file));
    }
    match args.version {
        None => std::fs::read_to_string(&target.path).map_err(|e| {
            format!(
                "Error: The path {} is not a readable text file: {e}",
                args.file
            )
        }),
        Some(version) => {
            let store = versions::store_dir(root);
            let record = ops::record_at(&store, root, &args.file, &target, version as i64)?;
            Ok(record.content)
        }
    }
}

fn missing(display: &str) -> String {
    format!("Error: The path {display} does not exist. Please provide a valid path.")
}

/// A parsed `changes` timespan (design D3): no date library, the grammar
/// compiles against the stored `YYYY-MM-DDTHH:MM:SSZ` timestamp format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Timespan {
    /// Every recorded change.
    All,
    /// A rolling window of this many seconds before now.
    Window(u64),
    /// Every change from the start of this day (days since the epoch) on.
    Since(i64),
    /// Changes within `start..=end`, both days inclusive (days since epoch).
    Range { start: i64, end: i64 },
}

/// Seconds in an hour / day / week; `m` is a rolling 30-day window, not a
/// calendar month (design D3).
const HOUR: u64 = 3_600;
const DAY: u64 = 86_400;
const WEEK: u64 = 7 * DAY;
const MONTH: u64 = 30 * DAY;

/// Parse a timespan per the grammar: `<n>h`, `<n>d`, `<n>w`, `<n>m`,
/// `YYYY-MM-DD`, `START..END`, or `all`. Used as the clap value parser for
/// `changes`, so an error text becomes a usage error naming the value.
pub fn parse_timespan(spec: &str) -> Result<Timespan, String> {
    if spec == "all" {
        return Ok(Timespan::All);
    }
    if let Ok(seconds) = parse_duration(spec) {
        return Ok(Timespan::Window(seconds));
    }
    if let Some((start, end)) = spec.split_once("..") {
        let (start, end) = match (parse_date(start), parse_date(end)) {
            (Ok(start), Ok(end)) => (start, end),
            _ => {
                return Err(format!(
                    "invalid timespan `{spec}`; expected <n>h, <n>d, <n>w, <n>m, YYYY-MM-DD, START..END, or all"
                ));
            }
        };
        if start > end {
            return Err(format!(
                "invalid timespan `{spec}`: the range start is after its end"
            ));
        }
        return Ok(Timespan::Range { start, end });
    }
    if let Ok(days) = parse_date(spec) {
        return Ok(Timespan::Since(days));
    }
    Err(format!(
        "invalid timespan `{spec}`; expected <n>h, <n>d, <n>w, <n>m, YYYY-MM-DD, START..END, or all"
    ))
}

/// `<n>` followed by one of the duration suffixes, to seconds.
fn parse_duration(spec: &str) -> Result<u64, ()> {
    let (digits, suffix) = spec.split_at(spec.len().checked_sub(1).ok_or(())?);
    let unit = match suffix {
        "h" => HOUR,
        "d" => DAY,
        "w" => WEEK,
        "m" => MONTH,
        _ => return Err(()),
    };
    let count: u64 = digits.parse().map_err(|_| ())?;
    if count == 0 {
        return Err(());
    }
    Ok(count * unit)
}

/// A `YYYY-MM-DD` date, as days since the Unix epoch.
fn parse_date(spec: &str) -> Result<i64, ()> {
    let bytes = spec.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(());
    }
    let year: i64 = spec[..4].parse().map_err(|_| ())?;
    let month: u32 = spec[5..7].parse().map_err(|_| ())?;
    let day: u32 = spec[8..10].parse().map_err(|_| ())?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(());
    }
    Ok(days_from_civil(year, month, day))
}

/// `(year, month, day)` in the proleptic Gregorian calendar to days since the
/// Unix epoch (Hinnant's `days_from_civil`, inverse of `civil_from_days`).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64; // [0, 399]
    let mp = if m > 2 { m - 3 } else { m + 9 } as u64; // [0, 11]
    let doy = (153 * mp + 2) / 5 + u64::from(d) - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe as i64 - 719_468
}

/// The `[start, end)` cutoff pair of `YYYY-MM-DDTHH:MM:SSZ` strings for a
/// timespan, computed against `now` (seconds since the epoch). Stored
/// timestamps sort lexicographically in chronological order, so filtering is
/// a plain string comparison.
fn cutoffs(timespan: &Timespan, now: u64) -> (String, String) {
    const DAWN: &str = "0000-01-01T00:00:00Z";
    const FAR_FUTURE: &str = "9999-12-31T23:59:59Z";
    match timespan {
        Timespan::All => (DAWN.to_string(), FAR_FUTURE.to_string()),
        Timespan::Window(seconds) => (
            crate::versions::format_utc(now.saturating_sub(*seconds)),
            // Timestamps have second resolution, so the exclusive upper bound
            // of `now + 1` keeps changes made in the query's own second
            // visible.
            crate::versions::format_utc(now.saturating_add(1)),
        ),
        Timespan::Since(days) => (day_start(*days), FAR_FUTURE.to_string()),
        Timespan::Range { start, end } => (day_start(*start), day_start(end + 1)),
    }
}

/// The start of a day (days since the epoch) as a stored timestamp.
fn day_start(days: i64) -> String {
    let (year, month, day) = crate::versions::civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T00:00:00Z")
}

/// The output of `changes`: one line per recorded change across all files,
/// newest first; `no memories yet` on a fresh machine.
fn changes_output(root: &Path, timespan: Option<&Timespan>) -> Result<String, String> {
    match inspect_root(root) {
        MemoryRoot::Missing(_) => Ok("no memories yet\n".to_string()),
        MemoryRoot::Present(root) => changes(&root, timespan),
    }
}

/// One recorded change in structured form (design D5): the version record
/// plus its `+N -M` line-count summary against the previous version. Shared
/// by the CLI `changes` text output and the web JSON API.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ChangeRecord {
    /// UTC timestamp of the change (`YYYY-MM-DDTHH:MM:SSZ`).
    pub recorded_at: String,
    /// Path of the memory file, relative to the memory root.
    pub path: String,
    /// Per-path 1-based sequence number.
    pub seq: u64,
    /// The command that produced this version.
    pub cause: String,
    /// Lines added against the previous version.
    pub added: usize,
    /// Lines removed against the previous version.
    pub removed: usize,
}

/// The structured store scan behind `changes` (design D5): every recorded
/// change across all files, filtered by the timespan cutoff, newest first
/// with a path/seq tiebreak. `None` timespan means the CLI default (7d); the
/// web feed passes `Some(&Timespan::All)`.
pub fn scan_changes(root: &Path, timespan: Option<&Timespan>) -> Result<Vec<ChangeRecord>, String> {
    const DEFAULT: Timespan = Timespan::Window(7 * DAY);
    let (start, end) = cutoffs(timespan.unwrap_or(&DEFAULT), crate::versions::now_secs());

    let store = crate::versions::store_dir(root);
    let mut per_path: Vec<Vec<crate::versions::Record>> = Vec::new();
    if store.is_dir() {
        let mut files: Vec<PathBuf> = std::fs::read_dir(&store)
            .map_err(|e| {
                format!(
                    "Error: cannot read the version store {}: {e}",
                    store.display()
                )
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                format!(
                    "Error: cannot read the version store {}: {e}",
                    store.display()
                )
            })?
            .into_iter()
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "jsonl"))
            .map(|entry| entry.path())
            .collect();
        files.sort();
        for file in files {
            // Decode the store file name back to the recorded path.
            let encoded = file
                .file_stem()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let rel = crate::versions::decode_key(&encoded);
            per_path.push(crate::versions::read(&store, &rel)?);
        }
    }

    let mut records: Vec<ChangeRecord> = Vec::new();
    for path_records in &per_path {
        for (index, record) in path_records.iter().enumerate() {
            if record.recorded_at.as_str() < start.as_str()
                || record.recorded_at.as_str() >= end.as_str()
            {
                continue;
            }
            let previous = if index == 0 {
                ""
            } else {
                path_records[index - 1].content.as_str()
            };
            let (added, removed) = ops::diff_counts(previous, &record.content);
            records.push(ChangeRecord {
                recorded_at: record.recorded_at.clone(),
                path: record.path.clone(),
                seq: record.seq,
                cause: record.cause.clone(),
                added,
                removed,
            });
        }
    }
    // Newest first; ties are stable on path, then seq (design D4).
    records.sort_by(|a, b| {
        b.recorded_at
            .cmp(&a.recorded_at)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.seq.cmp(&b.seq))
    });
    Ok(records)
}

/// The `changes` global log: the shared structured scan (design D5) printed
/// as date, time, path, cause, and the `+N -M` line-count summary.
fn changes(root: &Path, timespan: Option<&Timespan>) -> Result<String, String> {
    let mut out = String::new();
    for record in scan_changes(root, timespan)? {
        out.push_str(&format!(
            "{} {}\t{}\t{}\t+{} -{}\n",
            &record.recorded_at[..10],
            &record.recorded_at[11..19],
            record.path,
            record.cause,
            record.added,
            record.removed
        ));
    }
    Ok(out)
}

/// The output of `versions`: the file's version records as a table of
/// version, UTC timestamp, and cause, oldest first; `no memories yet` on a
/// fresh machine.
fn versions_output(root: &Path, file: &str) -> Result<String, String> {
    match inspect_root(root) {
        MemoryRoot::Missing(_) => Ok("no memories yet\n".to_string()),
        MemoryRoot::Present(root) => versions_table(&root, file),
    }
}

/// The `versions` table for one memory file.
fn versions_table(root: &Path, file: &str) -> Result<String, String> {
    let target = paths::resolve(root, file)?;
    if target.is_root {
        return Err("Error: the memory root itself has no version records".to_string());
    }
    std::fs::symlink_metadata(&target.path).map_err(|_| missing(file))?;
    let rel = ops::rel_key(root, &target)?;
    let records = versions::read(&versions::store_dir(root), &rel)?;
    if records.is_empty() {
        return Err(format!(
            "Error: {file} has no version history (it was never changed through this tool)."
        ));
    }
    let mut out = String::new();
    for record in &records {
        out.push_str(&format!(
            "v{}\t{}\t{}\n",
            record.seq, record.recorded_at, record.cause
        ));
    }
    Ok(out)
}

/// What an inspection command finds at the memory root: a usable root, or the
/// fresh-machine condition reported as `no memories yet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryRoot {
    /// The memory root exists; run the command against it.
    Present(PathBuf),
    /// No memory root: report `no memories yet`, exit zero, change nothing.
    Missing(PathBuf),
}

impl MemoryRoot {
    /// The root path, whether present or not.
    pub fn path(&self) -> &Path {
        match self {
            MemoryRoot::Present(path) | MemoryRoot::Missing(path) => path,
        }
    }
}

/// Resolve the memory root without ever creating it: `root::resolve` only
/// computes the path, and existence decides the fresh-machine condition.
pub fn resolve_root() -> Result<MemoryRoot, String> {
    let root = root::resolve().map_err(|e| format!("Error: cannot locate the memory root: {e}"))?;
    Ok(inspect_root(&root))
}

fn inspect_root(root: &Path) -> MemoryRoot {
    if root.exists() {
        MemoryRoot::Present(root.to_path_buf())
    } else {
        MemoryRoot::Missing(root.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::temp_dir;

    #[test]
    fn fresh_machine_reports_missing_root_without_creating_anything() {
        let home = temp_dir("inspect-fresh");
        let root = root::root_in(&home);
        assert!(!root.exists(), "the test needs a machine with no memory");

        assert_eq!(inspect_root(&root), MemoryRoot::Missing(root.clone()));
        assert!(!root.exists(), "resolving must not create the root");
        assert!(
            !crate::versions::store_dir(&root).exists(),
            "resolving must not create the version store"
        );
    }

    #[test]
    fn existing_root_is_present() {
        let home = temp_dir("inspect-existing");
        let root = root::root_in(&home);
        std::fs::create_dir_all(&root).unwrap();

        assert_eq!(inspect_root(&root), MemoryRoot::Present(root));
    }

    #[test]
    fn list_names_nested_files_with_line_counts() {
        let root = root::root_in(&temp_dir("inspect-list"));
        std::fs::create_dir_all(root.join("projects/alpha")).unwrap();
        std::fs::write(root.join("MEMORY.md"), "one\ntwo\nthree\n").unwrap();
        std::fs::write(root.join("projects/alpha/notes.txt"), "x\n").unwrap();

        let out = list_output(&root).unwrap();
        assert!(out.contains("MEMORY.md\t3"), "{out}");
        assert!(out.contains("projects/\n"), "{out}");
        assert!(out.contains("projects/alpha/notes.txt\t1"), "{out}");
    }

    #[test]
    fn list_shows_available_version_ranges() {
        let home = temp_dir("inspect-list-versions");
        let root = root::root_in(&home);
        root::ensure(&root).unwrap();
        ops::run(
            &root,
            &memory_args("create", "/memories/edited.md", "one\n"),
        )
        .unwrap();
        ops::run(
            &root,
            &memory_args("create", "/memories/edited.md", "two\n"),
        )
        .unwrap();
        ops::run(
            &root,
            &memory_args("create", "/memories/edited.md", "three\n"),
        )
        .unwrap();
        ops::run(&root, &memory_args("create", "/memories/once.md", "only\n")).unwrap();
        std::fs::write(root.join("by-hand.md"), "never recorded\n").unwrap();

        let out = list_output(&root).unwrap();
        assert!(out.contains("edited.md\t1\tv1..v3"), "{out}");
        assert!(out.contains("once.md\t1\tv1"), "{out}");
        assert!(
            out.contains("by-hand.md\t1\t-"),
            "unrecorded files show no versions: {out}"
        );
    }

    #[test]
    fn list_on_a_fresh_machine_prints_no_memories_yet_and_creates_nothing() {
        let root = root::root_in(&temp_dir("inspect-list-fresh"));
        assert!(!root.exists());

        let out = list_output(&root).unwrap();
        assert_eq!(out, "no memories yet\n");
        assert!(!root.exists(), "listing must not create the root");
        assert!(
            !crate::versions::store_dir(&root).exists(),
            "listing must not create the version store"
        );
    }

    fn show_args(file: &str, version: Option<u64>) -> cli::ShowArgs {
        cli::ShowArgs {
            file: file.to_string(),
            version,
            raw: false,
            render: false,
        }
    }

    fn memory_args(command: &str, path: &str, file_text: &str) -> crate::args::MemoryArgs {
        crate::args::MemoryArgs {
            command: serde_json::from_str(&format!("\"{command}\"")).unwrap(),
            path: path.to_string(),
            file_text: Some(file_text.to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
            insert_text: None,
            new_path: None,
            view_range: None,
            version: None,
        }
    }

    #[test]
    fn show_prints_live_and_recorded_content() {
        let home = temp_dir("inspect-show");
        let root = root::root_in(&home);
        root::ensure(&root).unwrap();
        ops::run(
            &root,
            &memory_args("create", "/memories/notes.md", "first draft\n"),
        )
        .unwrap();
        ops::run(
            &root,
            &memory_args("create", "/memories/notes.md", "second draft\n"),
        )
        .unwrap();

        let live = show_content(&root, &show_args("notes.md", None)).unwrap();
        assert_eq!(live, "second draft\n");
        let past = show_content(&root, &show_args("notes.md", Some(1))).unwrap();
        assert_eq!(past, "first draft\n", "version 1 is the recorded content");
    }

    #[test]
    fn show_unknown_version_names_the_available_range() {
        let home = temp_dir("inspect-show-range");
        let root = root::root_in(&home);
        root::ensure(&root).unwrap();
        ops::run(&root, &memory_args("create", "/memories/notes.md", "one\n")).unwrap();

        let err = show_content(&root, &show_args("notes.md", Some(5))).unwrap_err();
        assert!(err.contains("no version 5"), "{err}");
        assert!(err.contains("1..=1"), "{err}");
        assert!(err.contains("notes.md"), "{err}");
    }

    #[test]
    fn show_missing_file_names_the_path() {
        let home = temp_dir("inspect-show-missing");
        let root = root::root_in(&home);
        root::ensure(&root).unwrap();

        let err = show_content(&root, &show_args("gone.md", None)).unwrap_err();
        assert!(err.contains("gone.md"), "{err}");
        let err = show_content(&root, &show_args("gone.md", Some(1))).unwrap_err();
        assert!(err.contains("gone.md"), "{err}");
    }

    #[test]
    fn show_gate_forces_raw_over_render_and_follows_stdout() {
        // `--raw` always forces raw, even with `--render` (design D2).
        assert!(force_raw(true, true, true));
        assert!(force_raw(true, false, false));
        // `--render` forces rendering, even into a pipe.
        assert!(!force_raw(false, true, false));
        // Default: follow stdout.
        assert!(force_raw(false, false, false), "piped stdout prints raw");
        assert!(!force_raw(false, false, true), "terminal stdout renders");
    }

    #[test]
    fn versions_lists_three_records_oldest_first() {
        let home = temp_dir("inspect-versions");
        let root = root::root_in(&home);
        root::ensure(&root).unwrap();
        ops::run(&root, &memory_args("create", "/memories/notes.md", "one\n")).unwrap();
        ops::run(&root, &memory_args("create", "/memories/notes.md", "two\n")).unwrap();
        ops::run(
            &root,
            &memory_args("create", "/memories/notes.md", "three\n"),
        )
        .unwrap();

        let out = versions_table(&root, "notes.md").unwrap();
        let v1 = out.find("v1").expect("v1 row");
        let v2 = out.find("v2").expect("v2 row");
        let v3 = out.find("v3").expect("v3 row");
        assert!(v1 < v2 && v2 < v3, "oldest first: {out}");
        // Version, UTC timestamp, cause per row.
        assert!(out.contains("v1\t") && out.contains("v3\t"), "{out}");
        let v1_row = &out[v1..v2];
        assert!(v1_row.contains("create"), "shows the cause: {v1_row}");
        assert!(
            v1_row[3..].starts_with("20") && v1_row.contains('T') && v1_row.contains('Z'),
            "shows a UTC timestamp: {v1_row}"
        );
    }

    #[test]
    fn versions_without_records_names_the_path() {
        let home = temp_dir("inspect-versions-empty");
        let root = root::root_in(&home);
        root::ensure(&root).unwrap();
        std::fs::write(root.join("by-hand.md"), "edited outside the tool\n").unwrap();

        let err = versions_table(&root, "by-hand.md").unwrap_err();
        assert!(err.contains("by-hand.md"), "names the path: {err}");
        assert!(err.contains("no version history"), "{err}");
    }

    const NOW: u64 = 1_800_000_000; // 2027-01-15T08:00:00Z, a fixed "now"

    #[test]
    fn timespan_grammar_is_table_driven() {
        use Timespan::*;
        // Duration suffixes.
        assert_eq!(parse_timespan("24h"), Ok(Window(24 * HOUR)));
        assert_eq!(parse_timespan("7d"), Ok(Window(7 * DAY)));
        assert_eq!(parse_timespan("2w"), Ok(Window(2 * WEEK)));
        assert_eq!(
            parse_timespan("3m"),
            Ok(Window(3 * MONTH)),
            "`m` is a rolling 30-day window"
        );
        // Absolute date and range.
        let first = days_from_civil(2026, 11, 1);
        let last = days_from_civil(2026, 10, 31);
        assert_eq!(parse_timespan("2026-11-01"), Ok(Since(first)));
        assert_eq!(
            parse_timespan("2026-10-01..2026-10-31"),
            Ok(Range {
                start: days_from_civil(2026, 10, 1),
                end: last
            })
        );
        assert_eq!(parse_timespan("all"), Ok(All));
    }

    #[test]
    fn invalid_timespans_name_the_value() {
        for spec in [
            "",
            "7",
            "7x",
            "x",
            "d7",
            "0d",
            "2026-13-01",
            "2026-00-05",
            "2026-2-3",
            "01-02-2026",
            "bogus",
        ] {
            let err = parse_timespan(spec).unwrap_err();
            assert!(
                err.contains(spec),
                "names the invalid value {spec:?}: {err}"
            );
        }
    }

    #[test]
    fn reversed_range_is_rejected() {
        let err = parse_timespan("2026-10-31..2026-10-01").unwrap_err();
        assert!(err.contains("after its end"), "{err}");
        assert!(err.contains("2026-10-31..2026-10-01"), "{err}");
    }

    #[test]
    fn cutoffs_are_half_open_timestamp_strings() {
        use Timespan::*;
        // A rolling window ends at now, starts window seconds before.
        let (start, end) = cutoffs(&Window(2 * DAY), NOW);
        assert_eq!(start, "2027-01-13T08:00:00Z");
        assert_eq!(
            end, "2027-01-15T08:00:01Z",
            "second resolution keeps the query second visible"
        );
        // The 30-day `m` semantics: 3m is exactly 90 days.
        let (start, _) = cutoffs(&Window(3 * MONTH), NOW);
        assert_eq!(start, "2026-10-17T08:00:00Z");
        // An absolute date starts at that day's 00:00:00Z.
        let (start, end) = cutoffs(&Since(days_from_civil(2026, 11, 1)), NOW);
        assert_eq!(start, "2026-11-01T00:00:00Z");
        assert_eq!(end, FAR_FUTURE_END);
        // A range includes its end day: cutoff_end is the day after.
        let (start, end) = cutoffs(
            &Range {
                start: days_from_civil(2026, 10, 1),
                end: days_from_civil(2026, 10, 31),
            },
            NOW,
        );
        assert_eq!(start, "2026-10-01T00:00:00Z");
        assert_eq!(end, "2026-11-01T00:00:00Z");
        // `all` spans everything.
        assert_eq!(cutoffs(&All, NOW).0, DAWN_START);
    }

    const DAWN_START: &str = "0000-01-01T00:00:00Z";
    const FAR_FUTURE_END: &str = "9999-12-31T23:59:59Z";

    #[test]
    fn changes_orders_multi_file_records_newest_first() {
        let home = temp_dir("inspect-changes");
        let root = root::root_in(&home);
        root::ensure(&root).unwrap();
        let store = crate::versions::store_dir(&root);
        std::fs::create_dir_all(&store).unwrap();
        // Store files written directly, with controlled timestamps: a.txt
        // edited on 01-05, b.txt created on 01-03, a.txt created on 01-01,
        // and two same-second records whose order is the path/seq tiebreak.
        let record = |path: &str, seq: u64, content: &str, cause: &str, at: &str| {
            serde_json::json!({
                "path": path, "seq": seq, "content": content,
                "cause": cause, "recorded_at": at,
            })
            .to_string()
        };
        std::fs::write(
            store.join(format!("{}.jsonl", crate::versions::encode_key("a.txt"))),
            format!(
                "{}\n{}\n",
                record("a.txt", 1, "a1\na2\na3\n", "create", "2027-01-01T10:00:00Z"),
                record(
                    "a.txt",
                    2,
                    "a1\na2\na3b\n",
                    "str_replace",
                    "2027-01-05T12:30:00Z"
                ),
            ),
        )
        .unwrap();
        std::fs::write(
            store.join(format!("{}.jsonl", crate::versions::encode_key("b.txt"))),
            record("b.txt", 1, "b1\n", "create", "2027-01-03T08:00:00Z") + "\n",
        )
        .unwrap();
        std::fs::write(
            store.join(format!("{}.jsonl", crate::versions::encode_key("c1.txt"))),
            record("c1.txt", 1, "same\n", "create", "2027-01-04T09:00:00Z") + "\n",
        )
        .unwrap();
        std::fs::write(
            store.join(format!("{}.jsonl", crate::versions::encode_key("c2.txt"))),
            record("c2.txt", 1, "same\n", "create", "2027-01-04T09:00:00Z") + "\n",
        )
        .unwrap();

        let out = changes_output(&root, Some(&Timespan::All)).unwrap();
        let a_edit = out
            .find("a.txt\tstr_replace\t+1 -1")
            .expect("a.txt edit row");
        let c1 = out.find("c1.txt\tcreate\t+1 -0").expect("c1 row");
        let c2 = out.find("c2.txt\tcreate\t+1 -0").expect("c2 row");
        let b = out.find("b.txt\tcreate\t+1 -0").expect("b.txt row");
        let a_create = out.find("a.txt\tcreate\t+3 -0").expect("a.txt create row");
        // Newest first, with same-second records stable on path then seq.
        let order = [a_edit, c1, c2, b, a_create];
        assert!(order.windows(2).all(|w| w[0] < w[1]), "ordered rows: {out}");
        // Date, time, path, cause, summary columns on every line.
        for line in out.lines() {
            let columns: Vec<&str> = line.split('\t').collect();
            assert_eq!(
                columns.len(),
                4,
                "date time / path / cause / summary: {line}"
            );
            assert!(columns[0].contains(' ') && columns[0].len() == 19, "{line}");
        }
    }

    #[test]
    fn changes_filters_by_the_timespan() {
        let home = temp_dir("inspect-changes-filter");
        let root = root::root_in(&home);
        root::ensure(&root).unwrap();
        ops::run(&root, &memory_args("create", "/memories/old.txt", "old\n")).unwrap();
        // The record is seconds old; a zero-length window from the past shows nothing.
        let past = Timespan::Range {
            start: days_from_civil(2020, 1, 1),
            end: days_from_civil(2020, 1, 2),
        };
        let out = changes_output(&root, Some(&past)).unwrap();
        assert_eq!(out, "", "no change lines within the window");
    }

    #[test]
    fn changes_default_window_is_seven_days() {
        let home = temp_dir("inspect-changes-default");
        let root = root::root_in(&home);
        root::ensure(&root).unwrap();
        ops::run(
            &root,
            &memory_args("create", "/memories/new.txt", "fresh\n"),
        )
        .unwrap();

        let default = changes_output(&root, None).unwrap();
        let explicit = changes_output(&root, Some(&Timespan::Window(7 * DAY))).unwrap();
        assert_eq!(default, explicit, "default timespan behaves like 7d");
        assert!(default.contains("new.txt\tcreate\t+1 -0"), "{default}");
    }

    #[test]
    fn changes_on_a_fresh_machine_is_empty_and_creates_nothing() {
        let root = root::root_in(&temp_dir("inspect-changes-fresh"));
        let out = changes_output(&root, None).unwrap();
        assert_eq!(out, "no memories yet\n");
        assert!(!root.exists(), "must not create the root");
        assert!(
            !crate::versions::store_dir(&root).exists(),
            "must not create the store"
        );
    }

    /// The inspection commands against the real layout (`HOME/.memorija`
    /// with the store beside the root), driven through `ops::run` for the
    /// mutating side, asserting the spec scenarios end to end.
    #[test]
    fn end_to_end_inspection_through_the_real_layout() {
        let home = temp_dir("inspect-e2e");
        let root = root::root_in(&home);
        root::ensure(&root).unwrap();

        // create + two edits through the tool: three recorded versions.
        ops::run(
            &root,
            &memory_args("create", "/memories/demo/notes.md", "first draft\n"),
        )
        .unwrap();
        ops::run(
            &root,
            &memory_args("create", "/memories/demo/notes.md", "second draft\nmore\n"),
        )
        .unwrap();
        ops::run(
            &root,
            &memory_args(
                "create",
                "/memories/demo/notes.md",
                "second draft\nmore\nand more\n",
            ),
        )
        .unwrap();
        ops::run(
            &root,
            &memory_args("create", "/memories/MEMORY.md", "index\n"),
        )
        .unwrap();

        // list: nested files with line counts and version ranges.
        let out = list_output(&root).unwrap();
        assert!(out.contains("MEMORY.md\t1\tv1"), "{out}");
        assert!(out.contains("demo/"), "{out}");
        assert!(out.contains("demo/notes.md\t3\tv1..v3"), "{out}");

        // show: the live file.
        let out = show_content(&root, &show_args("demo/notes.md", None)).unwrap();
        assert_eq!(out, "second draft\nmore\nand more\n");

        // show <file> <version>: the recorded content, not the live file.
        let out = show_content(&root, &show_args("demo/notes.md", Some(1))).unwrap();
        assert_eq!(out, "first draft\n");

        // versions: all three records, oldest first, with causes.
        let out = versions_table(&root, "demo/notes.md").unwrap();
        let v1 = out.find("v1").expect("v1");
        let v2 = out.find("v2").expect("v2");
        let v3 = out.find("v3").expect("v3");
        assert!(v1 < v2 && v2 < v3, "oldest first: {out}");
        assert!(out.contains("create"), "{out}");

        // changes: every record across files, newest first, with summaries.
        let out = changes_output(&root, Some(&Timespan::All)).unwrap();
        assert!(out.contains("demo/notes.md\tcreate\t+1 -0"), "{out}");
        assert!(out.contains("MEMORY.md\tcreate\t+1 -0"), "{out}");
        let notes_last = out.find("demo/notes.md\tcreate\t+2 -1").unwrap();
        let memory = out.find("MEMORY.md\tcreate\t+1 -0").unwrap();
        assert!(memory < notes_last, "newest first: {out}");

        // unknown version: non-zero error naming the range.
        let err = show_content(&root, &show_args("demo/notes.md", Some(9))).unwrap_err();
        assert!(err.contains("1..=3"), "{err}");
    }
}
