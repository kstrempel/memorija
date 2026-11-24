//! The memory commands (specs: `memory-tool`, `memory-versions`).
//!
//! `run` executes one tool call against the memory root. Recoverable failures
//! come back as `Err(message)` — the handler turns that into an `isError`
//! tool result so the calling model can self-correct. Every mutation of a
//! memory file's content appends a version record to the version store.

use crate::args::{Command, CommandArg, MemoryArgs};
use crate::paths;
use crate::versions;
use std::path::Path;

/// Execute one tool call against the memory root, with the version store
/// beside the root (`~/.memorija/versions`). See [`run_in`] for dispatch.
pub fn run(root: &Path, args: &MemoryArgs) -> Result<String, String> {
    run_in(&versions::store_dir(root), root, args)
}

/// Execute one tool call against the memory root and version store.
/// Recoverable failures come back as `Err(message)` — the handler turns that
/// into an `isError` tool result so the calling model can self-correct.
pub fn run_in(store: &Path, root: &Path, args: &MemoryArgs) -> Result<String, String> {
    let command = match &args.command {
        CommandArg::Known(command) => command.clone(),
        CommandArg::Unknown(value) => {
            let supported = Command::ALL.map(|c| c.as_str()).join(", ");
            return Err(format!(
                "Error: unknown command `{value}`. Supported commands are: {supported}"
            ));
        }
    };

    match command {
        Command::View => view(store, root, args),
        Command::Create => create(store, root, args),
        Command::StrReplace => str_replace(store, root, args),
        Command::Insert => insert(store, root, args),
        Command::Delete => delete(root, args),
        Command::Rename => rename(root, args),
        Command::History => history(store, root, args),
        Command::Restore => restore(store, root, args),
    }
}

fn require<'a>(
    value: &'a Option<String>,
    field: &str,
    command: Command,
) -> Result<&'a str, String> {
    value.as_deref().ok_or_else(|| {
        format!(
            "Error: `{field}` is required for the `{}` command",
            command.as_str()
        )
    })
}

fn view(store: &Path, root: &Path, args: &MemoryArgs) -> Result<String, String> {
    let target = paths::resolve(root, &args.path)?;

    if let Some(version) = args.version {
        return view_version(store, root, &args.path, &target, version, args.view_range);
    }

    let meta = std::fs::symlink_metadata(&target.path).map_err(|_| missing(&args.path))?;

    if meta.is_dir() {
        // `view_range` is ignored for directory listings.
        return list_dir(&target.path, &args.path);
    }

    let content = std::fs::read_to_string(&target.path)
        .map_err(|_| format!("Error: The path {} is not a readable text file", args.path))?;
    view_file(&args.path, &content, args.view_range)
}

/// `view` with an explicit `version`: return the recorded content instead of
/// the live file.
fn view_version(
    store: &Path,
    root: &Path,
    display: &str,
    target: &paths::Target,
    version: i64,
    range: Option<[i64; 2]>,
) -> Result<String, String> {
    let meta = std::fs::symlink_metadata(&target.path).map_err(|_| missing(display))?;
    if meta.is_dir() {
        return Err(format!(
            "Error: `version` applies to files only; {display} is a directory"
        ));
    }
    let record = record_at(store, root, display, target, version)?;
    view_file(display, &record.content, range)
}

/// The record for `version` on `rel_path`, or an error naming the file's
/// available version range. Shared with the CLI inspection layer.
pub(crate) fn record_at(
    store: &Path,
    root: &Path,
    display: &str,
    target: &paths::Target,
    version: i64,
) -> Result<versions::Record, String> {
    let rel = rel_key(root, target)?;
    let records = versions::read(store, &rel)?;
    if version >= 1
        && let Some(record) = records.iter().find(|record| record.seq == version as u64)
    {
        return Ok(record.clone());
    }
    match records.last() {
        None => Err(format!("Error: {display} has no recorded versions")),
        Some(last) => Err(format!(
            "Error: {display} has no version {version}; available versions are 1..={}",
            last.seq
        )),
    }
}

fn view_file(display: &str, content: &str, range: Option<[i64; 2]>) -> Result<String, String> {
    let lines: Vec<&str> = if content.is_empty() {
        Vec::new()
    } else {
        content.lines().collect()
    };
    let count = lines.len() as i64;

    let (start, end) = match range {
        None => (1, count),
        Some([start, end]) => {
            if start > count {
                return Err(format!(
                    "Error: view_range start {start} is beyond the end of {display}, which has {count} lines"
                ));
            }
            let end = if end < 0 { count } else { end.min(count) };
            (start.max(1), end)
        }
    };

    let mut out = String::new();
    for (index, line) in lines.iter().enumerate() {
        let number = (index + 1) as i64;
        if number >= start && number <= end {
            out.push_str(&format!("{number:>6}\t{line}\n"));
        }
    }
    Ok(out)
}

fn list_dir(path: &Path, display: &str) -> Result<String, String> {
    let mut out = format!("Here're the files and directories in {display}:\n");
    let entries = walk(path, path)?;
    if entries.is_empty() {
        out.push_str("(empty)\n");
    }
    for (line_count, relative) in entries {
        out.push_str(&format!("{line_count}\t{relative}\n"));
    }
    Ok(out)
}

/// Recursively collect `(line_count, display_path)` for every file under
/// `dir`, with display paths relative to the listing root. Shared with the
/// CLI inspection layer.
pub(crate) fn walk(dir: &Path, display_root: &Path) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(dir)
        .map_err(|e| format!("Error: cannot read directory {}: {e}", dir.display()))?
        .collect::<Result<_, _>>()
        .map_err(|e| format!("Error: cannot read directory {}: {e}", dir.display()))?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let entry_path = entry.path();
        let relative = entry_path
            .strip_prefix(display_root)
            .map(paths::key_string)
            .unwrap_or_else(|_| paths::key_string(&entry_path));
        let meta = std::fs::symlink_metadata(&entry_path)
            .map_err(|e| format!("Error: cannot inspect {}: {e}", entry_path.display()))?;
        if meta.is_dir() {
            out.push(("-".to_string(), format!("{relative}/")));
            out.extend(walk(&entry_path, display_root)?);
        } else {
            let line_count = std::fs::read_to_string(&entry_path)
                .map(|content| content.lines().count().to_string())
                .unwrap_or_else(|_| "-".to_string());
            out.push((line_count, relative));
        }
    }
    Ok(out)
}

fn create(store: &Path, root: &Path, args: &MemoryArgs) -> Result<String, String> {
    let file_text = require(&args.file_text, "file_text", Command::Create)?;
    let target = paths::resolve(root, &args.path)?;
    if target.is_root {
        return Err(format!(
            "Error: cannot create the memory root itself ({})",
            args.path
        ));
    }
    if let Ok(meta) = std::fs::symlink_metadata(&target.path)
        && meta.is_dir()
    {
        return Err(format!("Error: {} is a directory", args.path));
    }
    write_new_content(&target, &args.path, file_text)?;
    record_change(store, root, &target, file_text, Command::Create.as_str())?;
    Ok(format!("File created successfully at: {}", args.path))
}

fn str_replace(store: &Path, root: &Path, args: &MemoryArgs) -> Result<String, String> {
    let old_str = require(&args.old_str, "old_str", Command::StrReplace)?;
    let target = paths::resolve(root, &args.path)?;
    let content = read_target_file(root, &args.path, &target)?;

    let match_count = content.matches(old_str).count();
    if match_count == 0 {
        return Err(format!(
            "No replacement was performed, old_str `{old_str}` did not appear verbatim in {}.",
            args.path
        ));
    }
    if match_count > 1 {
        return Err(format!(
            "No replacement was performed. Multiple occurrences ({match_count}) of old_str `{old_str}` in {}. Please ensure it is unique by including more surrounding context.",
            args.path
        ));
    }
    let new_str = args.new_str.as_deref().unwrap_or("");
    let updated = content.replacen(old_str, new_str, 1);
    std::fs::write(&target.path, &updated)
        .map_err(|e| format!("Error: cannot write {}: {e}", args.path))?;
    record_change(store, root, &target, &updated, Command::StrReplace.as_str())?;
    Ok("The memory file has been edited.".to_string())
}

fn insert(store: &Path, root: &Path, args: &MemoryArgs) -> Result<String, String> {
    let insert_line = args
        .insert_line
        .ok_or_else(|| "Error: `insert_line` is required for the `insert` command".to_string())?;
    let insert_text = require(&args.insert_text, "insert_text", Command::Insert)?;
    let target = paths::resolve(root, &args.path)?;
    let content = read_target_file(root, &args.path, &target)?;

    let mut lines: Vec<String> = if content.is_empty() {
        Vec::new()
    } else {
        content.lines().map(|line| line.to_string()).collect()
    };
    let count = lines.len() as i64;
    if insert_line < 0 || insert_line > count {
        return Err(format!(
            "Error: Invalid `insert_line` parameter: {insert_line}. It should be within the range of lines of the file: [0, {count}]"
        ));
    }
    let position = insert_line as usize;
    lines.splice(
        position..position,
        insert_text
            .strip_suffix('\n')
            .unwrap_or(insert_text)
            .split('\n')
            .map(str::to_string),
    );
    let mut updated = lines.join("\n");
    if content.ends_with('\n') {
        updated.push('\n');
    }
    std::fs::write(&target.path, &updated)
        .map_err(|e| format!("Error: cannot write {}: {e}", args.path))?;
    record_change(store, root, &target, &updated, Command::Insert.as_str())?;
    Ok(format!("The file {} has been edited.", args.path))
}

fn delete(root: &Path, args: &MemoryArgs) -> Result<String, String> {
    let target = paths::resolve(root, &args.path)?;
    if target.is_root {
        return Err(format!(
            "Error: the memory root ({}) cannot be deleted",
            args.path
        ));
    }
    let meta = std::fs::symlink_metadata(&target.path).map_err(|_| missing(&args.path))?;
    if meta.is_dir() {
        std::fs::remove_dir_all(&target.path)
            .map_err(|e| format!("Error: cannot delete {}: {e}", args.path))?;
    } else {
        std::fs::remove_file(&target.path)
            .map_err(|e| format!("Error: cannot delete {}: {e}", args.path))?;
    }
    Ok(format!("Successfully deleted {}", args.path))
}

fn rename(root: &Path, args: &MemoryArgs) -> Result<String, String> {
    let new_path = require(&args.new_path, "new_path", Command::Rename)?;
    let source = paths::resolve(root, &args.path)?;
    if source.is_root {
        return Err(format!(
            "Error: the memory root ({}) cannot be renamed",
            args.path
        ));
    }
    let destination = paths::resolve(root, new_path)?;
    if destination.is_root {
        return Err(format!(
            "Error: the destination {} already exists",
            new_path
        ));
    }
    std::fs::symlink_metadata(&source.path).map_err(|_| missing(&args.path))?;
    if std::fs::symlink_metadata(&destination.path).is_ok() {
        return Err(format!(
            "Error: The destination {} already exists",
            new_path
        ));
    }
    if let Some(parent) = destination.path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Error: cannot create destination directory: {e}"))?;
    }
    std::fs::rename(&source.path, &destination.path)
        .map_err(|e| format!("Error: cannot rename {} to {}: {e}", args.path, new_path))?;
    Ok(format!(
        "Successfully renamed {} to {}",
        args.path, new_path
    ))
}

fn history(store: &Path, root: &Path, args: &MemoryArgs) -> Result<String, String> {
    let target = paths::resolve(root, &args.path)?;
    let rel = rel_key(root, &target)?;
    let records = versions::read(store, &rel)?;
    if records.is_empty() {
        return Err(format!(
            "Error: {} has no version history (it was never changed through this tool). Please provide a valid path.",
            args.path
        ));
    }

    let mut out = format!(
        "Version history for {} ({} versions, oldest first):\n",
        args.path,
        records.len()
    );
    for (index, record) in records.iter().enumerate() {
        let previous = if index == 0 {
            None
        } else {
            Some(records[index - 1].content.as_str())
        };
        let (summary, excerpt) = change_summary(previous, &record.content);
        out.push_str(&format!(
            "v{}\t{}\t{}\t{}\n",
            record.seq, record.recorded_at, record.cause, summary
        ));
        for line in excerpt {
            out.push_str(&format!("    {line}\n"));
        }
    }
    Ok(out)
}

fn restore(store: &Path, root: &Path, args: &MemoryArgs) -> Result<String, String> {
    let version = args
        .version
        .ok_or_else(|| "Error: `version` is required for the `restore` command".to_string())?;
    let target = paths::resolve(root, &args.path)?;
    if target.is_root {
        return Err(format!(
            "Error: the memory root ({}) has no version history",
            args.path
        ));
    }
    let meta = std::fs::symlink_metadata(&target.path).map_err(|_| missing(&args.path))?;
    if meta.is_dir() {
        return Err(format!("Error: The path {} is a directory", args.path));
    }

    let record = record_at(store, root, &args.path, &target, version)?;
    write_new_content(&target, &args.path, &record.content)?;
    record_change(store, root, &target, &record.content, "restore")?;
    Ok(format!("Restored {} to version {}", args.path, version))
}

/// The path of a resolved target relative to the memory root — the key the
/// version store records under. Shared with the CLI inspection layer.
pub(crate) fn rel_key(root: &Path, target: &paths::Target) -> Result<String, String> {
    let canon = root.canonicalize().map_err(|e| {
        format!(
            "Error: the memory root {} is not accessible: {e}",
            root.display()
        )
    })?;
    target
        .path
        .strip_prefix(&canon)
        .map(paths::key_string)
        .map_err(|_| {
            "Error: internal path bookkeeping failed (target outside the memory root)".to_string()
        })
}

/// Append a version record for a successful content change.
fn record_change(
    store: &Path,
    root: &Path,
    target: &paths::Target,
    content: &str,
    cause: &str,
) -> Result<(), String> {
    let rel = rel_key(root, target)?;
    versions::append(store, &rel, content, cause).map(|_| ())
}

/// Write new file content through the same path `create` uses: ensure the
/// parent directory exists, then write the file.
fn write_new_content(target: &paths::Target, display: &str, content: &str) -> Result<(), String> {
    if let Some(parent) = target.path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Error: cannot create parent directory: {e}"))?;
    }
    std::fs::write(&target.path, content).map_err(|e| format!("Error: cannot write {display}: {e}"))
}

/// Added/removed line counts between two file contents, trimming the common
/// prefix and suffix so only the changed middle counts. Shared between the
/// `history` summary and the CLI `changes` log.
pub(crate) fn diff_counts(old: &str, new: &str) -> (usize, usize) {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let (added, removed) = changed_middle(&old_lines, &new_lines);
    (added.len(), removed.len())
}

/// The changed middle of two line lists: `(added, removed)` after trimming the
/// common prefix and suffix.
fn changed_middle<'a, 'b>(
    old_lines: &'b [&'a str],
    new_lines: &'b [&'a str],
) -> (&'b [&'a str], &'b [&'a str]) {
    // Trim the common prefix and suffix; whatever remains changed.
    let mut start = 0;
    while start < old_lines.len() && start < new_lines.len() && old_lines[start] == new_lines[start]
    {
        start += 1;
    }
    let mut end = 0;
    while end < old_lines.len() - start
        && end < new_lines.len() - start
        && old_lines[old_lines.len() - 1 - end] == new_lines[new_lines.len() - 1 - end]
    {
        end += 1;
    }
    let removed = &old_lines[start..old_lines.len() - end];
    let added = &new_lines[start..new_lines.len() - end];
    (added, removed)
}

/// A compact change summary against the previous version (`+added -removed`
/// with a short excerpt of the changed lines), for the `history` timeline.
fn change_summary(previous: Option<&str>, content: &str) -> (String, Vec<String>) {
    let new_lines: Vec<&str> = content.lines().collect();

    let Some(previous) = previous else {
        return (
            format!("initial version ({} line(s))", new_lines.len()),
            Vec::new(),
        );
    };

    let old_lines: Vec<&str> = previous.lines().collect();
    let (added, removed) = changed_middle(&old_lines, &new_lines);

    let mut excerpt = Vec::new();
    for line in removed.iter().take(2) {
        excerpt.push(format!("- {}", truncate(line)));
    }
    for line in added.iter().take(2) {
        excerpt.push(format!("+ {}", truncate(line)));
    }
    (
        format!("+{} -{} lines", added.len(), removed.len()),
        excerpt,
    )
}

/// A line shortened for a history excerpt.
fn truncate(line: &str) -> String {
    const MAX: usize = 72;
    if line.chars().count() <= MAX {
        line.to_string()
    } else {
        format!("{}…", line.chars().take(MAX).collect::<String>())
    }
}

fn read_target_file(root: &Path, display: &str, target: &paths::Target) -> Result<String, String> {
    let _ = root;
    let meta = std::fs::symlink_metadata(&target.path).map_err(|_| missing(display))?;
    if meta.is_dir() {
        return Err(format!("Error: The path {display} is a directory"));
    }
    std::fs::read_to_string(&target.path)
        .map_err(|e| format!("Error: The path {display} is not a readable text file: {e}"))
}

fn missing(display: &str) -> String {
    format!("Error: The path {display} does not exist. Please provide a valid path.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{CommandArg, MemoryArgs};
    use crate::testutil::temp_dir;
    use std::fs;
    use std::path::PathBuf;

    fn args(command: &str, path: &str) -> MemoryArgs {
        MemoryArgs {
            command: serde_json::from_value::<CommandArg>(serde_json::Value::String(
                command.to_string(),
            ))
            .unwrap(),
            path: path.to_string(),
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
            insert_text: None,
            new_path: None,
            view_range: None,
            version: None,
        }
    }

    fn with_file(root: &Path, rel: &str, content: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    /// The memory root nested inside a unique temp directory, so the version
    /// store (a `versions` directory beside the root) is isolated per test.
    fn setup(label: &str) -> PathBuf {
        let root = temp_dir(label).join("memories");
        fs::create_dir_all(&root).unwrap();
        root
    }

    /// All version records recorded for `rel` under `root`'s store.
    fn records_for(root: &Path, rel: &str) -> Vec<crate::versions::Record> {
        crate::versions::read(&crate::versions::store_dir(root), rel).unwrap()
    }

    // --- 2.3: symlink containment and root protections ---

    // Symlinks are a Unix feature here: Windows symlinks need privileges and
    // the containment rule has no Windows equivalent yet (design D8).
    #[cfg(unix)]
    #[test]
    fn escaping_symlink_is_refused_and_outside_file_untouched() {
        let root = setup("symlink-escape");
        let outside = temp_dir("symlink-outside");
        fs::write(outside.join("secret.txt"), "top secret").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("leak")).unwrap();

        // view through the escaping symlink is refused
        let err = run(&root, &args("view", "/memories/leak")).unwrap_err();
        assert!(err.contains("symbolic link"), "{err}");

        // create through it would overwrite the outside file — refused
        let mut create_args = args("create", "/memories/leak/secret.txt");
        create_args.file_text = Some("clobber".to_string());
        assert!(run(&root, &create_args).is_err());
        assert_eq!(
            fs::read_to_string(outside.join("secret.txt")).unwrap(),
            "top secret"
        );

        // delete through it is refused as well
        assert!(run(&root, &args("delete", "/memories/leak")).is_err());
        assert!(outside.join("secret.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_within_root_is_followed() {
        let root = setup("symlink-inside");
        with_file(&root, "real.txt", "hello\n");
        std::os::unix::fs::symlink(root.join("real.txt"), root.join("alias.txt")).unwrap();

        let out = run(&root, &args("view", "/memories/alias.txt")).unwrap();
        assert!(out.contains("hello"), "{out}");
    }

    #[test]
    fn delete_root_is_refused() {
        let root = setup("delete-root");
        with_file(&root, "notes.txt", "keep\n");

        let err = run(&root, &args("delete", "/memories")).unwrap_err();
        assert!(err.contains("cannot be deleted"), "{err}");
        assert!(root.join("notes.txt").exists());
    }

    #[test]
    fn rename_root_is_refused() {
        let root = setup("rename-root");
        with_file(&root, "notes.txt", "keep\n");

        let mut rename_args = args("rename", "/memories");
        rename_args.new_path = Some("/memories/renamed".to_string());
        let err = run(&root, &rename_args).unwrap_err();
        assert!(err.contains("cannot be renamed"), "{err}");
        assert!(root.join("notes.txt").exists());
    }

    #[test]
    fn view_root_is_allowed() {
        let root = setup("view-root");
        with_file(&root, "notes.txt", "one\n");
        let out = run(&root, &args("view", "/memories")).unwrap();
        assert!(out.contains("notes.txt"), "{out}");
    }

    // --- 3.1: view ---

    #[test]
    fn view_file_numbers_all_lines() {
        let root = setup("view-file");
        with_file(&root, "notes.txt", "Hello World\nThis is line two\n");

        let out = run(&root, &args("view", "/memories/notes.txt")).unwrap();
        assert!(out.contains("     1\tHello World"), "{out}");
        assert!(out.contains("     2\tThis is line two"), "{out}");
    }

    #[test]
    fn view_file_with_range() {
        let root = setup("view-range");
        with_file(&root, "notes.txt", "one\ntwo\nthree\nfour\nfive\n");

        let mut range_args = args("view", "/memories/notes.txt");
        range_args.view_range = Some([5, -1]);
        let out = run(&root, &range_args).unwrap();
        assert!(
            out.contains("5\tfive") || out.contains("     5\tfive"),
            "{out}"
        );
        assert!(!out.contains("one"), "{out}");

        let mut range_args = args("view", "/memories/notes.txt");
        range_args.view_range = Some([2, 3]);
        let out = run(&root, &range_args).unwrap();
        assert!(out.contains("2\ttwo") && out.contains("3\tthree"), "{out}");
        assert!(!out.contains("one") && !out.contains("four"), "{out}");
    }

    #[test]
    fn view_range_beyond_eof_reports_line_count() {
        let root = setup("view-range-err");
        with_file(&root, "notes.txt", "one\ntwo\n");

        let mut range_args = args("view", "/memories/notes.txt");
        range_args.view_range = Some([10, -1]);
        let err = run(&root, &range_args).unwrap_err();
        assert!(err.contains("2 lines"), "{err}");
    }

    #[test]
    fn walk_and_the_store_key_agree_on_forward_slashes() {
        let root = setup("canonical-relative");
        with_file(&root, "projects/alpha/notes.txt", "x\n");

        let walked = walk(&root, &root).unwrap();
        assert!(
            walked
                .iter()
                .any(|(lines, path)| path == "projects/alpha/notes.txt" && lines == "1"),
            "{walked:?}"
        );
        let target = crate::paths::resolve(&root, "/memories/projects/alpha/notes.txt").unwrap();
        assert_eq!(rel_key(&root, &target).unwrap(), "projects/alpha/notes.txt");
    }

    #[test]
    fn view_directory_lists_tree_with_line_counts() {
        let root = setup("view-dir");
        with_file(&root, "a.txt", "1\n2\n");
        with_file(&root, "projects/alpha/notes.txt", "x\n");
        fs::create_dir_all(root.join("empty")).unwrap();

        let out = run(&root, &args("view", "/memories")).unwrap();
        assert!(
            out.contains("2\ta.txt") || out.contains("2\t a.txt"),
            "{out}"
        );
        assert!(out.contains("projects/"), "{out}");
        assert!(out.contains("projects/alpha/notes.txt"), "{out}");
        assert!(out.contains("1\tprojects/alpha/notes.txt"), "{out}");

        // view_range is ignored for directories
        let mut range_args = args("view", "/memories");
        range_args.view_range = Some([1, 5]);
        let out = run(&root, &range_args).unwrap();
        assert!(out.contains("projects/"), "{out}");
    }

    #[test]
    fn view_missing_path_errors() {
        let root = setup("view-missing");
        let err = run(&root, &args("view", "/memories/nope.txt")).unwrap_err();
        assert!(err.contains("does not exist"), "{err}");
    }

    // --- 3.2: create ---

    #[test]
    fn create_in_nested_directory() {
        let root = setup("create-nested");
        let mut create_args = args("create", "/memories/projects/alpha/notes.txt");
        create_args.file_text = Some("Meeting notes\n".to_string());

        let out = run(&root, &create_args).unwrap();
        assert!(out.contains("created"), "{out}");
        assert_eq!(
            fs::read_to_string(root.join("projects/alpha/notes.txt")).unwrap(),
            "Meeting notes\n"
        );
    }

    #[test]
    fn create_overwrites_existing_file() {
        let root = setup("create-overwrite");
        with_file(&root, "notes.txt", "old\n");

        let mut create_args = args("create", "/memories/notes.txt");
        create_args.file_text = Some("new\n".to_string());
        assert!(run(&root, &create_args).is_ok());
        assert_eq!(fs::read_to_string(root.join("notes.txt")).unwrap(), "new\n");
    }

    // --- 3.3: str_replace ---

    #[test]
    fn str_replace_unique_match() {
        let root = setup("replace-unique");
        with_file(&root, "notes.txt", "Favorite color: blue\n");

        let mut replace_args = args("str_replace", "/memories/notes.txt");
        replace_args.old_str = Some("blue".to_string());
        replace_args.new_str = Some("green".to_string());

        assert!(run(&root, &replace_args).is_ok());
        assert_eq!(
            fs::read_to_string(root.join("notes.txt")).unwrap(),
            "Favorite color: green\n"
        );
    }

    #[test]
    fn str_replace_ambiguous_fails_and_file_unchanged() {
        let root = setup("replace-ambiguous");
        with_file(&root, "notes.txt", "blue\nblue\n");

        let mut replace_args = args("str_replace", "/memories/notes.txt");
        replace_args.old_str = Some("blue".to_string());
        replace_args.new_str = Some("green".to_string());

        let err = run(&root, &replace_args).unwrap_err();
        assert!(err.contains("2"), "{err}");
        assert!(err.contains("unique") || err.contains("context"), "{err}");
        assert_eq!(
            fs::read_to_string(root.join("notes.txt")).unwrap(),
            "blue\nblue\n"
        );
    }

    #[test]
    fn str_replace_zero_matches_fails() {
        let root = setup("replace-missing");
        with_file(&root, "notes.txt", "hello\n");

        let mut replace_args = args("str_replace", "/memories/notes.txt");
        replace_args.old_str = Some("absent text".to_string());
        replace_args.new_str = Some("x".to_string());

        let err = run(&root, &replace_args).unwrap_err();
        assert!(err.contains("did not appear verbatim"), "{err}");
    }

    #[test]
    fn str_replace_without_new_str_deletes() {
        let root = setup("replace-delete");
        with_file(&root, "notes.txt", "keepme REMOVEthistail\n");

        let mut replace_args = args("str_replace", "/memories/notes.txt");
        replace_args.old_str = Some("REMOVEthis".to_string());

        assert!(run(&root, &replace_args).is_ok());
        assert_eq!(
            fs::read_to_string(root.join("notes.txt")).unwrap(),
            "keepme tail\n"
        );
    }

    #[test]
    fn str_replace_multiline_old_str() {
        let root = setup("replace-multiline");
        with_file(&root, "notes.txt", "alpha\nbeta\ngamma\n");

        let mut replace_args = args("str_replace", "/memories/notes.txt");
        replace_args.old_str = Some("alpha\nbeta".to_string());
        replace_args.new_str = Some("ALPHA".to_string());

        assert!(run(&root, &replace_args).is_ok());
        assert_eq!(
            fs::read_to_string(root.join("notes.txt")).unwrap(),
            "ALPHA\ngamma\n"
        );
    }

    // --- 3.4: insert ---

    #[test]
    fn insert_at_top() {
        let root = setup("insert-top");
        with_file(&root, "todo.txt", "first\nsecond\n");

        let mut insert_args = args("insert", "/memories/todo.txt");
        insert_args.insert_line = Some(0);
        insert_args.insert_text = Some("top\n".to_string());

        assert!(run(&root, &insert_args).is_ok());
        assert_eq!(
            fs::read_to_string(root.join("todo.txt")).unwrap(),
            "top\nfirst\nsecond\n"
        );
    }

    #[test]
    fn insert_after_last_line() {
        let root = setup("insert-last");
        with_file(&root, "todo.txt", "first\nsecond\n");

        let mut insert_args = args("insert", "/memories/todo.txt");
        insert_args.insert_line = Some(2);
        insert_args.insert_text = Some("last\n".to_string());

        assert!(run(&root, &insert_args).is_ok());
        assert_eq!(
            fs::read_to_string(root.join("todo.txt")).unwrap(),
            "first\nsecond\nlast\n"
        );
    }

    #[test]
    fn insert_multiline_text() {
        let root = setup("insert-multiline");
        with_file(&root, "todo.txt", "first\nsecond\n");

        let mut insert_args = args("insert", "/memories/todo.txt");
        insert_args.insert_line = Some(1);
        insert_args.insert_text = Some("a\nb\n".to_string());

        assert!(run(&root, &insert_args).is_ok());
        assert_eq!(
            fs::read_to_string(root.join("todo.txt")).unwrap(),
            "first\na\nb\nsecond\n"
        );
    }

    #[test]
    fn insert_beyond_end_fails_reporting_count() {
        let root = setup("insert-beyond");
        with_file(&root, "todo.txt", "first\nsecond\n");

        let mut insert_args = args("insert", "/memories/todo.txt");
        insert_args.insert_line = Some(5);
        insert_args.insert_text = Some("x\n".to_string());

        let err = run(&root, &insert_args).unwrap_err();
        assert!(err.contains("[0, 2]"), "{err}");
        assert_eq!(
            fs::read_to_string(root.join("todo.txt")).unwrap(),
            "first\nsecond\n"
        );
    }

    // --- 3.5: delete and rename ---

    #[test]
    fn delete_directory_tree_recursively() {
        let root = setup("delete-tree");
        with_file(&root, "projects/alpha/notes.txt", "x\n");
        with_file(&root, "projects/alpha/more.txt", "y\n");

        assert!(run(&root, &args("delete", "/memories/projects")).is_ok());
        assert!(!root.join("projects").exists());
    }

    #[test]
    fn delete_missing_path_fails_naming_path() {
        let root = setup("delete-missing");
        let err = run(&root, &args("delete", "/memories/gone.txt")).unwrap_err();
        assert!(
            err.contains("/memories/gone.txt") || err.contains("gone.txt"),
            "{err}"
        );
    }

    #[test]
    fn rename_file() {
        let root = setup("rename-file");
        with_file(&root, "draft.txt", "content\n");

        let mut rename_args = args("rename", "/memories/draft.txt");
        rename_args.new_path = Some("/memories/final.txt".to_string());

        assert!(run(&root, &rename_args).is_ok());
        assert_eq!(
            fs::read_to_string(root.join("final.txt")).unwrap(),
            "content\n"
        );
        assert!(!root.join("draft.txt").exists());
    }

    #[test]
    fn rename_creates_missing_parents() {
        let root = setup("rename-parents");
        with_file(&root, "draft.txt", "content\n");

        let mut rename_args = args("rename", "/memories/draft.txt");
        rename_args.new_path = Some("/memories/archive/2024/draft.txt".to_string());

        assert!(run(&root, &rename_args).is_ok());
        assert_eq!(
            fs::read_to_string(root.join("archive/2024/draft.txt")).unwrap(),
            "content\n"
        );
    }

    #[test]
    fn rename_onto_existing_destination_fails() {
        let root = setup("rename-existing");
        with_file(&root, "draft.txt", "draft\n");
        with_file(&root, "final.txt", "final\n");

        let mut rename_args = args("rename", "/memories/draft.txt");
        rename_args.new_path = Some("/memories/final.txt".to_string());

        let err = run(&root, &rename_args).unwrap_err();
        assert!(err.contains("already exists"), "{err}");
        assert_eq!(
            fs::read_to_string(root.join("draft.txt")).unwrap(),
            "draft\n"
        );
        assert_eq!(
            fs::read_to_string(root.join("final.txt")).unwrap(),
            "final\n"
        );
    }

    // --- 3.6: per-command validation and unknown commands ---

    #[test]
    fn create_without_file_text_names_field() {
        let root = setup("validate-create");
        let err = run(&root, &args("create", "/memories/notes.txt")).unwrap_err();
        assert!(err.contains("file_text") && err.contains("create"), "{err}");
    }

    #[test]
    fn str_replace_without_old_str_names_field() {
        let root = setup("validate-replace");
        with_file(&root, "notes.txt", "x\n");
        let err = run(&root, &args("str_replace", "/memories/notes.txt")).unwrap_err();
        assert!(
            err.contains("old_str") && err.contains("str_replace"),
            "{err}"
        );
    }

    #[test]
    fn rename_without_new_path_names_field() {
        let root = setup("validate-rename");
        with_file(&root, "notes.txt", "x\n");
        let err = run(&root, &args("rename", "/memories/notes.txt")).unwrap_err();
        assert!(err.contains("new_path") && err.contains("rename"), "{err}");
    }

    #[test]
    fn insert_without_line_or_text_names_field() {
        let root = setup("validate-insert");
        with_file(&root, "notes.txt", "x\n");
        let err = run(&root, &args("insert", "/memories/notes.txt")).unwrap_err();
        assert!(err.contains("insert_line"), "{err}");
    }

    #[test]
    fn unknown_command_lists_supported_commands() {
        let root = setup("validate-unknown");
        let err = run(&root, &args("copy", "/memories/notes.txt")).unwrap_err();
        assert!(
            err.contains("view")
                && err.contains("create")
                && err.contains("str_replace")
                && err.contains("insert")
                && err.contains("delete")
                && err.contains("rename"),
            "{err}"
        );
    }

    #[test]
    fn traversal_is_tool_error_not_crash() {
        let root = setup("validate-traversal");
        let err = run(&root, &args("view", "/memories/../..")).unwrap_err();
        assert!(err.contains("outside the memory root"), "{err}");
    }

    // --- 1.2: the version store is outside path resolution ---

    #[test]
    fn paths_attempting_the_version_store_are_refused() {
        let root = setup("store-unreachable");
        // The store lives beside the root; a `/memories/../versions` path
        // reaches for it and must be refused like any traversal.
        let err = run(&root, &args("view", "/memories/../versions")).unwrap_err();
        assert!(err.contains("outside the memory root"), "{err}");

        let mut create_args = args("create", "/memories/../versions/notes.jsonl");
        create_args.file_text = Some("clobber".to_string());
        let err = run(&root, &create_args).unwrap_err();
        assert!(err.contains("outside the memory root"), "{err}");
        assert!(
            !crate::versions::store_dir(&root)
                .join("notes.jsonl")
                .exists()
        );
    }

    // --- 2.1: mutations append version records ---

    #[test]
    fn create_appends_one_record_with_create_cause() {
        let root = setup("vc-create");
        let mut create_args = args("create", "/memories/notes.txt");
        create_args.file_text = Some("one\n".to_string());
        run(&root, &create_args).unwrap();

        let records = records_for(&root, "notes.txt");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].cause, "create");
        assert_eq!(records[0].content, "one\n");
        assert_eq!(records[0].seq, 1);
    }

    #[test]
    fn str_replace_appends_edit_record() {
        let root = setup("vc-replace");
        with_file(&root, "notes.txt", "blue\n");
        let mut replace_args = args("str_replace", "/memories/notes.txt");
        replace_args.old_str = Some("blue".to_string());
        replace_args.new_str = Some("green".to_string());
        run(&root, &replace_args).unwrap();

        let records = records_for(&root, "notes.txt");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].cause, "str_replace");
        assert_eq!(records[0].content, "green\n");
    }

    #[test]
    fn str_replace_without_new_str_appends_record() {
        let root = setup("vc-replace-delete");
        with_file(&root, "notes.txt", "keepme REMOVEthistail\n");
        let mut replace_args = args("str_replace", "/memories/notes.txt");
        replace_args.old_str = Some("REMOVEthis".to_string());
        run(&root, &replace_args).unwrap();

        let records = records_for(&root, "notes.txt");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].cause, "str_replace");
        assert_eq!(records[0].content, "keepme tail\n");
    }

    #[test]
    fn insert_appends_record() {
        let root = setup("vc-insert");
        with_file(&root, "notes.txt", "first\nsecond\n");
        let mut insert_args = args("insert", "/memories/notes.txt");
        insert_args.insert_line = Some(0);
        insert_args.insert_text = Some("top\n".to_string());
        run(&root, &insert_args).unwrap();

        let records = records_for(&root, "notes.txt");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].cause, "insert");
        assert_eq!(records[0].content, "top\nfirst\nsecond\n");
    }

    #[test]
    fn failed_mutations_record_nothing() {
        let root = setup("vc-failed");
        with_file(&root, "notes.txt", "blue\n");
        let mut replace_args = args("str_replace", "/memories/notes.txt");
        replace_args.old_str = Some("absent".to_string());
        assert!(run(&root, &replace_args).is_err());

        assert!(records_for(&root, "notes.txt").is_empty());
    }

    // --- 3.1: history ---

    #[test]
    fn history_lists_three_records_oldest_first() {
        let root = setup("history-three");
        let mut create_args = args("create", "/memories/notes.txt");
        create_args.file_text = Some("Favorite color: blue\n".to_string());
        run(&root, &create_args).unwrap();
        let mut replace_args = args("str_replace", "/memories/notes.txt");
        replace_args.old_str = Some("blue".to_string());
        replace_args.new_str = Some("green".to_string());
        run(&root, &replace_args).unwrap();
        let mut insert_args = args("insert", "/memories/notes.txt");
        insert_args.insert_line = Some(0);
        insert_args.insert_text = Some("top\n".to_string());
        run(&root, &insert_args).unwrap();

        let out = run(&root, &args("history", "/memories/notes.txt")).unwrap();
        let v1 = out.find("v1").expect("v1 entry");
        let v2 = out.find("v2").expect("v2 entry");
        let v3 = out.find("v3").expect("v3 entry");
        assert!(v1 < v2 && v2 < v3, "oldest first: {out}");
        assert!(out.contains("create"), "{out}");
        assert!(out.contains("str_replace"), "{out}");
        assert!(out.contains("insert"), "{out}");
        assert!(out.contains("+1 -1 lines"), "{out}");
        assert!(out.contains("- Favorite color: blue"), "{out}");
        assert!(out.contains("+ Favorite color: green"), "{out}");
    }

    #[test]
    fn history_after_rename_is_scoped_to_new_path() {
        let root = setup("history-rename");
        let mut create_args = args("create", "/memories/draft.txt");
        create_args.file_text = Some("content\n".to_string());
        run(&root, &create_args).unwrap();
        let mut rename_args = args("rename", "/memories/draft.txt");
        rename_args.new_path = Some("/memories/final.txt".to_string());
        run(&root, &rename_args).unwrap();

        let err = run(&root, &args("history", "/memories/final.txt")).unwrap_err();
        assert!(err.contains("final.txt"), "{err}");
        assert!(err.contains("no version history"), "{err}");
    }

    #[test]
    fn history_without_records_names_the_path() {
        let root = setup("history-empty");
        with_file(&root, "out-of-band.txt", "edited by hand\n");
        let err = run(&root, &args("history", "/memories/out-of-band.txt")).unwrap_err();
        assert!(err.contains("out-of-band.txt"), "{err}");
    }

    // --- 4.1: view at a past version ---

    #[test]
    fn view_old_version_returns_recorded_content_with_line_numbers() {
        let root = setup("view-version");
        let mut create_args = args("create", "/memories/notes.txt");
        create_args.file_text = Some("first\n".to_string());
        run(&root, &create_args).unwrap();
        let mut replace_args = args("str_replace", "/memories/notes.txt");
        replace_args.old_str = Some("first".to_string());
        replace_args.new_str = Some("second".to_string());
        run(&root, &replace_args).unwrap();

        let mut view_args = args("view", "/memories/notes.txt");
        view_args.version = Some(1);
        let out = run(&root, &view_args).unwrap();
        assert!(out.contains("1\tfirst"), "{out}");
        assert!(!out.contains("second"), "{out}");

        let mut view_args = args("view", "/memories/notes.txt");
        view_args.version = Some(2);
        let out = run(&root, &view_args).unwrap();
        assert!(out.contains("1\tsecond"), "{out}");
    }

    #[test]
    fn view_unknown_version_names_available_range() {
        let root = setup("view-version-range");
        let mut create_args = args("create", "/memories/notes.txt");
        create_args.file_text = Some("first\n".to_string());
        run(&root, &create_args).unwrap();

        let mut view_args = args("view", "/memories/notes.txt");
        view_args.version = Some(5);
        let err = run(&root, &view_args).unwrap_err();
        assert!(err.contains("no version 5"), "{err}");
        assert!(err.contains("1..=1"), "{err}");
    }

    #[test]
    fn view_directory_with_version_is_rejected() {
        let root = setup("view-version-dir");
        with_file(&root, "notes.txt", "x\n");
        let mut view_args = args("view", "/memories");
        view_args.version = Some(2);
        let err = run(&root, &view_args).unwrap_err();
        assert!(err.contains("applies to files only"), "{err}");
    }

    // --- 4.2 / 4.3: restore ---

    #[test]
    fn restore_sets_content_to_target_version_and_appends_record() {
        let root = setup("restore-basic");
        let mut create_args = args("create", "/memories/notes.txt");
        create_args.file_text = Some("first\n".to_string());
        run(&root, &create_args).unwrap();
        let mut replace_args = args("str_replace", "/memories/notes.txt");
        replace_args.old_str = Some("first".to_string());
        replace_args.new_str = Some("second".to_string());
        run(&root, &replace_args).unwrap();

        let mut restore_args = args("restore", "/memories/notes.txt");
        restore_args.version = Some(1);
        run(&root, &restore_args).unwrap();

        assert_eq!(
            fs::read_to_string(root.join("notes.txt")).unwrap(),
            "first\n"
        );
        let records = records_for(&root, "notes.txt");
        assert_eq!(records.len(), 3);
        assert_eq!(records[2].cause, "restore");
        assert_eq!(records[2].content, "first\n");
        assert_eq!(records[0].content, "first\n", "earlier records unchanged");
    }

    #[test]
    fn restore_then_restore_back() {
        let root = setup("restore-back");
        let mut create_args = args("create", "/memories/notes.txt");
        create_args.file_text = Some("v1\n".to_string());
        run(&root, &create_args).unwrap();
        let mut replace_args = args("str_replace", "/memories/notes.txt");
        replace_args.old_str = Some("v1".to_string());
        replace_args.new_str = Some("v2".to_string());
        run(&root, &replace_args).unwrap();

        let mut restore_args = args("restore", "/memories/notes.txt");
        restore_args.version = Some(1);
        run(&root, &restore_args).unwrap();
        let mut restore_args = args("restore", "/memories/notes.txt");
        restore_args.version = Some(2);
        run(&root, &restore_args).unwrap();

        assert_eq!(fs::read_to_string(root.join("notes.txt")).unwrap(), "v2\n");
        let records = records_for(&root, "notes.txt");
        assert_eq!(records.len(), 4);
        assert_eq!(records[2].cause, "restore");
        assert_eq!(records[3].cause, "restore");
        assert_eq!(records[3].content, "v2\n");
    }

    #[test]
    fn restore_unknown_version_names_range_and_leaves_file() {
        let root = setup("restore-unknown");
        let mut create_args = args("create", "/memories/notes.txt");
        create_args.file_text = Some("first\n".to_string());
        run(&root, &create_args).unwrap();

        let mut restore_args = args("restore", "/memories/notes.txt");
        restore_args.version = Some(9);
        let err = run(&root, &restore_args).unwrap_err();
        assert!(err.contains("no version 9"), "{err}");
        assert!(err.contains("1..=1"), "{err}");
        assert_eq!(
            fs::read_to_string(root.join("notes.txt")).unwrap(),
            "first\n"
        );
    }

    #[test]
    fn restore_of_deleted_file_is_refused() {
        let root = setup("restore-deleted");
        let mut create_args = args("create", "/memories/notes.txt");
        create_args.file_text = Some("first\n".to_string());
        run(&root, &create_args).unwrap();
        run(&root, &args("delete", "/memories/notes.txt")).unwrap();

        let mut restore_args = args("restore", "/memories/notes.txt");
        restore_args.version = Some(1);
        let err = run(&root, &restore_args).unwrap_err();
        assert!(err.contains("does not exist"), "{err}");
        assert!(!root.join("notes.txt").exists());
    }

    #[test]
    fn restore_without_version_names_field() {
        let root = setup("validate-restore");
        with_file(&root, "notes.txt", "x\n");
        let err = run(&root, &args("restore", "/memories/notes.txt")).unwrap_err();
        assert!(err.contains("version") && err.contains("restore"), "{err}");
    }

    // --- 5.2: end-to-end versioning through the run dispatch ---

    /// The whole versioning sequence against the real layout serve sets up
    /// (`HOME/.memorija/memories` with the store beside it): create, edit,
    /// insert, history, view at a version, restore, history again, and a root
    /// listing that shows no version-store entries.
    #[test]
    fn end_to_end_versioning_through_run() {
        let home = temp_dir("e2e-versioning");
        let root = crate::root::root_in(&home);
        crate::root::ensure(&root).unwrap();

        // create
        let mut create_args = args("create", "/memories/notes.txt");
        create_args.file_text = Some("first\n".to_string());
        run(&root, &create_args).unwrap();

        // edit
        let mut replace_args = args("str_replace", "/memories/notes.txt");
        replace_args.old_str = Some("first".to_string());
        replace_args.new_str = Some("second".to_string());
        run(&root, &replace_args).unwrap();

        // insert
        let mut insert_args = args("insert", "/memories/notes.txt");
        insert_args.insert_line = Some(0);
        insert_args.insert_text = Some("top\n".to_string());
        run(&root, &insert_args).unwrap();

        // history: three versions, oldest first, with their causes
        let out = run(&root, &args("history", "/memories/notes.txt")).unwrap();
        let v1 = out.find("v1").expect("v1 entry");
        let v2 = out.find("v2").expect("v2 entry");
        let v3 = out.find("v3").expect("v3 entry");
        assert!(v1 < v2 && v2 < v3, "oldest first: {out}");
        assert!(
            out.contains("create") && out.contains("str_replace") && out.contains("insert"),
            "{out}"
        );

        // view at version 1: the recorded content, not the live file
        let mut view_args = args("view", "/memories/notes.txt");
        view_args.version = Some(1);
        let out = run(&root, &view_args).unwrap();
        assert!(out.contains("1\tfirst"), "{out}");
        assert!(!out.contains("top"), "{out}");

        // restore to version 1
        let mut restore_args = args("restore", "/memories/notes.txt");
        restore_args.version = Some(1);
        run(&root, &restore_args).unwrap();
        assert_eq!(
            fs::read_to_string(root.join("notes.txt")).unwrap(),
            "first\n"
        );

        // history again: the restore is recorded, nothing rewritten
        let out = run(&root, &args("history", "/memories/notes.txt")).unwrap();
        let v4 = out.find("v4").expect("v4 entry");
        assert!(v3 < v4, "restore appended: {out}");
        assert!(out[v4..].contains("restore"), "{out}");

        // the root listing shows only user files, no version-store entries
        let out = run(&root, &args("view", "/memories")).unwrap();
        assert!(out.contains("MEMORY.md"), "{out}");
        assert!(out.contains("notes.txt"), "{out}");
        assert!(!out.contains("versions"), "{out}");
        assert!(home.join(".memorija/versions").is_dir());
    }
}
