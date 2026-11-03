//! Path resolution against the memory root (spec: `memory-root`).

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

/// A resolved operation target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Canonical path inside the memory root (existing components are
    /// canonicalized; not-yet-existing components are appended lexically).
    pub path: PathBuf,
    /// Whether the target is the memory root itself (`/memories`, `/`, or "").
    pub is_root: bool,
}

/// Resolve `input` against the memory root.
///
/// The optional `/memories` prefix is stripped, the remainder is joined onto
/// the root, `.`/`..` segments are normalized lexically, and the existing
/// prefix of the path is canonicalized. Any result outside the root — via
/// `..` traversal or a symlink that resolves outside — is refused with an
/// error message naming the rule.
pub fn resolve(root: &Path, input: &str) -> Result<Target, String> {
    let root_canon = root
        .canonicalize()
        .map_err(|e| format!("Error: the memory root {} is not accessible: {e}", root.display()))?;

    let rel = strip_memories_prefix(input);
    if rel.is_empty() {
        return Ok(Target {
            path: root_canon.clone(),
            is_root: true,
        });
    }

    // Lexical normalization: keep only normal components, popping on `..`.
    let mut stack: Vec<OsString> = Vec::new();
    for component in Path::new(rel).components() {
        match component {
            Component::Normal(name) => stack.push(name.to_os_string()),
            Component::CurDir => {}
            Component::ParentDir => {
                if stack.pop().is_none() {
                    return Err(format!(
                        "Error: the path `{input}` resolves outside the memory root; paths must stay within `/memories`"
                    ));
                }
            }
            // Inputs are relative after prefix stripping, so these cannot occur.
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "Error: the path `{input}` resolves outside the memory root; paths must stay within `/memories`"
                ));
            }
        }
    }

    // Walk from the canonical root, resolving symlinks in existing components
    // and refusing any that lead outside; once a component is missing, the
    // remaining components are appended lexically (they cannot be symlinks).
    let mut current = root_canon.clone();
    let mut missing = false;
    for name in stack {
        let next = current.join(&name);
        if missing {
            current = next;
            continue;
        }
        match std::fs::symlink_metadata(&next) {
            Ok(meta) if meta.file_type().is_symlink() => {
                let target = std::fs::canonicalize(&next).map_err(|e| {
                    format!("Error: cannot resolve the path `{input}`: {e}")
                })?;
                if !target.starts_with(&root_canon) {
                    return Err(format!(
                        "Error: the path `{input}` follows a symbolic link outside the memory root; paths must stay within `/memories`"
                    ));
                }
                current = target;
            }
            Ok(meta) if meta.is_dir() => {
                current = next;
            }
            Ok(_) => {
                // A file: everything after it cannot exist.
                current = next;
                missing = true;
            }
            Err(_) => {
                current = next;
                missing = true;
            }
        }
    }

    Ok(Target {
        path: current,
        is_root: false,
    })
}

/// Strip the optional `/memories` prefix and any leading slash, leaving a
/// relative path (`""` for the root itself).
fn strip_memories_prefix(input: &str) -> &str {
    let rest = if input == "/memories" || input.starts_with("/memories/") {
        &input["/memories".len()..]
    } else {
        input
    };
    rest.strip_prefix('/').unwrap_or(rest)
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
    fn prefixed_path_resolves_into_root() {
        let root = temp_root("prefixed");
        let target = resolve(&root, "/memories/notes.txt").unwrap();
        assert_eq!(target.path, root.canonicalize().unwrap().join("notes.txt"));
        assert!(!target.is_root);
    }

    #[test]
    fn bare_relative_path_resolves_into_root() {
        let root = temp_root("bare-relative");
        let target = resolve(&root, "notes.txt").unwrap();
        assert_eq!(target.path, root.canonicalize().unwrap().join("notes.txt"));
    }

    #[test]
    fn absolute_path_without_prefix_resolves_into_root() {
        let root = temp_root("absolute");
        let target = resolve(&root, "/notes.txt").unwrap();
        assert_eq!(target.path, root.canonicalize().unwrap().join("notes.txt"));
    }

    #[test]
    fn bare_root_resolves_to_root_itself() {
        let root = temp_root("bare-root");
        for input in ["/memories", "/", ""] {
            let target = resolve(&root, input).unwrap();
            assert!(target.is_root, "input {input:?} should be the root");
            assert_eq!(target.path, root.canonicalize().unwrap());
        }
    }

    #[test]
    fn prefix_only_matches_whole_segment() {
        let root = temp_root("prefix-segment");
        // `/memories2` is an ordinary name, not the prefix.
        let target = resolve(&root, "/memories2/notes.txt").unwrap();
        assert_eq!(target.path, root.canonicalize().unwrap().join("memories2/notes.txt"));
    }

    #[test]
    fn traversal_outside_root_is_refused() {
        let root = temp_root("traversal");
        for input in [
            "/memories/../..",
            "/memories/../../../etc/passwd",
            "../escape.txt",
            "sub/../../escape.txt",
        ] {
            let err = resolve(&root, input).unwrap_err();
            assert!(err.contains("outside the memory root"), "input {input:?}: {err}");
        }
    }

    #[test]
    fn traversal_back_inside_root_resolves() {
        let root = temp_root("traversal-inside");
        let target = resolve(&root, "/memories/projects/../notes.txt").unwrap();
        assert_eq!(target.path, root.canonicalize().unwrap().join("notes.txt"));
    }

    #[test]
    fn not_yet_existing_paths_resolve_lexically() {
        let root = temp_root("missing");
        let target = resolve(&root, "/memories/newdir/newfile.txt").unwrap();
        assert_eq!(target.path, root.canonicalize().unwrap().join("newdir/newfile.txt"));
        assert!(!target.is_root);
    }
}
