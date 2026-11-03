use std::fs;
use std::path::{Path, PathBuf};

/// Starter content written to a freshly created memory root.
const SEED_FILENAME: &str = "MEMORY.md";

const SEED_TEXT: &str = r#"# Memory

This directory (`~/.memorija/memories`) is your persistent memory: everything
here survives between sessions. Before starting any task, view this directory
to recover earlier progress, and record new progress here as you work.

Suggested organization:

- One file per topic or project (e.g. `projects/alpha/notes.txt`).
- Keep a `progress.md` tracking what is done and what comes next.
- Rename or delete files that are no longer relevant; avoid clutter.

Use the `memory` tool's commands (`view`, `create`, `str_replace`, `insert`,
`delete`, `rename`) to read and maintain these files.
"#;

/// The memory root for a given home directory: `~/.memorija/memories`.
pub fn root_in(home: &Path) -> PathBuf {
    home.join(".memorija").join("memories")
}

/// The memory root in the current user's home directory.
pub fn resolve() -> std::io::Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| std::io::Error::other("HOME is not set"))?;
    Ok(root_in(Path::new(&home)))
}

/// Create the memory root (and seed it) if it does not exist yet.
///
/// An existing root is left completely untouched — in particular, an existing
/// `MEMORY.md` is never overwritten.
pub fn ensure(root: &Path) -> std::io::Result<()> {
    if root.exists() {
        return Ok(());
    }
    fs::create_dir_all(root)?;
    fs::write(root.join(SEED_FILENAME), SEED_TEXT)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "memorija-test-{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn root_lives_under_home() {
        assert_eq!(
            root_in(Path::new("/home/tester")),
            PathBuf::from("/home/tester/.memorija/memories")
        );
    }

    #[test]
    fn fresh_start_creates_root_and_seed() {
        let home = temp_root("fresh");
        let root = root_in(&home);
        assert!(!root.exists());

        ensure(&root).unwrap();
        assert!(root.is_dir());
        let seed = fs::read_to_string(root.join(SEED_FILENAME)).unwrap();
        assert!(seed.contains("persistent memory"), "seed should give usage guidance");
    }

    #[test]
    fn existing_root_is_untouched() {
        let home = temp_root("existing");
        let root = root_in(&home);
        ensure(&root).unwrap();

        // User content written after the first launch must survive.
        fs::write(root.join(SEED_FILENAME), "my own memory").unwrap();
        fs::create_dir_all(root.join("projects")).unwrap();
        fs::write(root.join("projects/notes.txt"), "kept").unwrap();

        ensure(&root).unwrap();

        assert_eq!(fs::read_to_string(root.join(SEED_FILENAME)).unwrap(), "my own memory");
        assert_eq!(fs::read_to_string(root.join("projects/notes.txt")).unwrap(), "kept");
    }
}
