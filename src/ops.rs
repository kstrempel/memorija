//! The six memory commands (spec: `memory-tool`).
//!
//! `run` executes one tool call against the memory root. Recoverable failures
//! come back as `Err(message)` — the handler turns that into an `isError`
//! tool result so the calling model can self-correct.

use crate::args::{Command, CommandArg, MemoryArgs};
use crate::paths;
use std::path::Path;

pub fn run(root: &Path, args: &MemoryArgs) -> Result<String, String> {
    let command = match &args.command {
        CommandArg::Known(command) => command.clone(),
        CommandArg::Unknown(value) => {
            let supported = Command::ALL
                .map(|c| c.as_str())
                .join(", ");
            return Err(format!(
                "Error: unknown command `{value}`. Supported commands are: {supported}"
            ));
        }
    };

    match command {
        Command::View => view(root, args),
        Command::Create => create(root, args),
        Command::StrReplace => str_replace(root, args),
        Command::Insert => insert(root, args),
        Command::Delete => delete(root, args),
        Command::Rename => rename(root, args),
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

fn view(root: &Path, args: &MemoryArgs) -> Result<String, String> {
    let target = paths::resolve(root, &args.path)?;
    let meta = std::fs::symlink_metadata(&target.path)
        .map_err(|_| missing(&args.path))?;

    if meta.is_dir() {
        // `view_range` is ignored for directory listings.
        return list_dir(root, &target.path, &args.path);
    }

    let content = std::fs::read_to_string(&target.path)
        .map_err(|_| format!("Error: The path {} is not a readable text file", args.path))?;
    view_file(&args.path, &content, args.view_range)
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

fn list_dir(root: &Path, path: &Path, display: &str) -> Result<String, String> {
    let mut out = format!("Here're the files and directories in {display}:\n");
    let entries = walk(root, path, path)?;
    if entries.is_empty() {
        out.push_str("(empty)\n");
    }
    for (line_count, relative) in entries {
        out.push_str(&format!("{line_count}\t{relative}\n"));
    }
    Ok(out)
}

/// Recursively collect `(line_count, display_path)` for every file under
/// `dir`, with display paths relative to the listing root.
fn walk(root: &Path, dir: &Path, display_root: &Path) -> Result<Vec<(String, String)>, String> {
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
            .unwrap_or(&entry_path)
            .display()
            .to_string();
        let meta = std::fs::symlink_metadata(&entry_path)
            .map_err(|e| format!("Error: cannot inspect {}: {e}", entry_path.display()))?;
        if meta.is_dir() {
            out.push(("-".to_string(), format!("{relative}/")));
            out.extend(walk(root, &entry_path, display_root)?);
        } else {
            let line_count = std::fs::read_to_string(&entry_path)
                .map(|content| content.lines().count().to_string())
                .unwrap_or_else(|_| "-".to_string());
            out.push((line_count, relative));
        }
    }
    Ok(out)
}

fn create(root: &Path, args: &MemoryArgs) -> Result<String, String> {
    let file_text = require(&args.file_text, "file_text", Command::Create)?;
    let target = paths::resolve(root, &args.path)?;
    if target.is_root {
        return Err(format!(
            "Error: cannot create the memory root itself ({})"
        , args.path));
    }
    if let Ok(meta) = std::fs::symlink_metadata(&target.path) {
        if meta.is_dir() {
            return Err(format!("Error: {} is a directory", args.path));
        }
    }
    if let Some(parent) = target.path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Error: cannot create parent directory: {e}"))?;
    }
    std::fs::write(&target.path, file_text)
        .map_err(|e| format!("Error: cannot write {}: {e}", args.path))?;
    Ok(format!("File created successfully at: {}", args.path))
}

fn str_replace(root: &Path, args: &MemoryArgs) -> Result<String, String> {
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
    std::fs::write(&target.path, updated)
        .map_err(|e| format!("Error: cannot write {}: {e}", args.path))?;
    Ok("The memory file has been edited.".to_string())
}

fn insert(root: &Path, args: &MemoryArgs) -> Result<String, String> {
    let insert_line = args.insert_line.ok_or_else(|| {
        "Error: `insert_line` is required for the `insert` command".to_string()
    })?;
    let insert_text = require(&args.insert_text, "insert_text", Command::Insert)?;
    let target = paths::resolve(root, &args.path)?;
    let content = read_target_file(root, &args.path, &target)?;

    let mut lines: Vec<String> = if content.is_empty() {
        Vec::new()
    } else {
        content
            .lines()
            .map(|line| line.to_string())
            .collect()
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
        insert_text.strip_suffix('\n').unwrap_or(insert_text).split('\n').map(str::to_string),
    );
    let mut updated = lines.join("\n");
    if content.ends_with('\n') {
        updated.push('\n');
    }
    std::fs::write(&target.path, updated)
        .map_err(|e| format!("Error: cannot write {}: {e}", args.path))?;
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
    let meta = std::fs::symlink_metadata(&target.path)
        .map_err(|_| missing(&args.path))?;
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
    std::fs::symlink_metadata(&source.path)
        .map_err(|_| missing(&args.path))?;
    if std::fs::symlink_metadata(&destination.path).is_ok() {
        return Err(format!("Error: The destination {} already exists", new_path));
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

fn read_target_file(root: &Path, display: &str, target: &paths::Target) -> Result<String, String> {
    let _ = root;
    let meta = std::fs::symlink_metadata(&target.path)
        .map_err(|_| missing(display))?;
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
        }
    }

    fn with_file(root: &Path, rel: &str, content: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn setup(label: &str) -> PathBuf {
        temp_dir(label)
    }

    // --- 2.3: symlink containment and root protections ---

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
        assert!(out.contains("5\tfive") || out.contains("     5\tfive"), "{out}");
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
    fn view_directory_lists_tree_with_line_counts() {
        let root = setup("view-dir");
        with_file(&root, "a.txt", "1\n2\n");
        with_file(&root, "projects/alpha/notes.txt", "x\n");
        fs::create_dir_all(root.join("empty")).unwrap();

        let out = run(&root, &args("view", "/memories")).unwrap();
        assert!(out.contains("2\ta.txt") || out.contains("2\t a.txt"), "{out}");
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
        assert_eq!(fs::read_to_string(root.join("notes.txt")).unwrap(), "blue\nblue\n");
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
        assert_eq!(fs::read_to_string(root.join("todo.txt")).unwrap(), "first\nsecond\n");
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
        assert!(err.contains("/memories/gone.txt") || err.contains("gone.txt"), "{err}");
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
        assert_eq!(fs::read_to_string(root.join("draft.txt")).unwrap(), "draft\n");
        assert_eq!(fs::read_to_string(root.join("final.txt")).unwrap(), "final\n");
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
        assert!(err.contains("old_str") && err.contains("str_replace"), "{err}");
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
}
