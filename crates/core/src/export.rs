//! Writes a generated project to a chosen directory (spec: 导出契约).
//!
//! Paths come from the generator and are project-relative. Anything that
//! escapes the target directory (absolute paths, `..`) is rejected.

use std::path::{Component, Path, PathBuf};

use crate::generate::GeneratedFile;

/// Why an export could not proceed.
#[derive(Debug, PartialEq)]
pub enum ExportError {
    /// A file path is absolute or contains a `..` component.
    UnsafePath(String),
    /// Files already exist and `overwrite` was false.
    Conflicts(Vec<String>),
    /// Filesystem failure while writing.
    Io(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::UnsafePath(path) => {
                write!(f, "unsafe export path: {path}")
            }
            ExportError::Conflicts(paths) => {
                write!(f, "{} file(s) already exist", paths.len())
            }
            ExportError::Io(msg) => write!(f, "write failed: {msg}"),
        }
    }
}

/// Writes every file under `dir`, creating parent directories as needed.
///
/// When `overwrite` is false, the call writes nothing and returns
/// `Conflicts` if any target path already exists — the caller confirms, then
/// retries with `overwrite: true`.
pub fn export_project(
    dir: &Path,
    files: &[GeneratedFile],
    overwrite: bool,
) -> Result<(), ExportError> {
    if !dir.is_absolute() {
        return Err(ExportError::UnsafePath(dir.display().to_string()));
    }
    let mut seen = std::collections::HashSet::new();
    let mut targets = Vec::with_capacity(files.len());
    for file in files {
        let path = safe_join(dir, &file.path)?;
        if !seen.insert(path.clone()) {
            return Err(ExportError::UnsafePath(format!("duplicate export path: {}", file.path)));
        }
        targets.push((path, &file.content));
    }
    // The chosen root may itself be an OS alias; only descendants are rejected.
    // These checks prevent known symlink escapes, not concurrent path replacement.
    for (path, _) in &targets { reject_links(dir, path)?; }
    if !overwrite {
        let conflicts: Vec<_> = files.iter().zip(&targets)
            .filter(|(_, (path, _))| path.exists()).map(|(file, _)| file.path.clone()).collect();
        if !conflicts.is_empty() { return Err(ExportError::Conflicts(conflicts)); }
    }
    // Stage every file first. A failed write leaves all original files intact.
    // Persistence is atomic per file; the batch is not a filesystem transaction.
    let mut staged = Vec::new();
    for (path, content) in &targets {
        let parent = path.parent().ok_or_else(|| ExportError::UnsafePath(path.display().to_string()))?;
        std::fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
        reject_links(dir, path)?;
        // `NamedTempFile` defaults to 0600; exported sources should honor the
        // umask the way `std::fs::write` did (typically 0644).
        #[allow(unused_mut)]
        let mut builder = tempfile::Builder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            builder.permissions(std::fs::Permissions::from_mode(0o666));
        }
        let mut temp = builder.tempfile_in(parent).map_err(|e| io_error(path, e))?;
        use std::io::Write;
        temp.write_all(content.as_bytes()).map_err(|e| io_error(path, e))?;
        temp.as_file().sync_all().map_err(|e| io_error(path, e))?;
        staged.push((temp, path));
    }
    for (temp, path) in staged {
        reject_links(dir, path)?;
        if overwrite {
            temp.persist(path).map_err(|e| io_error(path, e.error))?;
        } else {
            temp.persist_noclobber(path).map_err(|e| io_error(path, e.error))?;
        }
    }
    Ok(())
}

fn io_error(path: &Path, error: std::io::Error) -> ExportError {
    ExportError::Io(format!("{}: {error}", path.display()))
}

fn reject_links(root: &Path, path: &Path) -> Result<(), ExportError> {
    let mut current = root.to_path_buf();
    let relative = path.strip_prefix(root).map_err(|_| ExportError::UnsafePath(path.display().to_string()))?;
    for part in relative.components() {
        current.push(part);
        match std::fs::symlink_metadata(&current) {
            Ok(meta) => {
                #[allow(unused_mut)]
                let mut link = meta.file_type().is_symlink();
                #[cfg(windows)]
                { use std::os::windows::fs::MetadataExt; link |= meta.file_attributes() & 0x400 != 0; }
                if link { return Err(ExportError::UnsafePath(format!("symlink or reparse point: {}", current.display()))); }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(e) => return Err(io_error(&current, e)),
        }
    }
    Ok(())
}

/// Joins a project-relative path onto `dir`, rejecting absolute paths and
/// any `..` traversal.
fn safe_join(dir: &Path, relative: &str) -> Result<PathBuf, ExportError> {
    let rel = Path::new(relative);
    if relative.is_empty() || relative.contains('\\') || relative.contains(':') {
        return Err(ExportError::UnsafePath(relative.to_string()));
    }
    for component in rel.components() {
        match component {
            Component::Normal(_) => {}
            _ => return Err(ExportError::UnsafePath(relative.to_string())),
        }
    }
    Ok(dir.join(rel))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> Vec<GeneratedFile> {
        vec![
            GeneratedFile {
                path: "internal/model/user.go".to_string(),
                content: "package model\n".to_string(),
                table: "users".to_string(),
            },
            GeneratedFile {
                path: "internal/repository/user_repo.go".to_string(),
                content: "package repository\n".to_string(),
                table: "users".to_string(),
            },
        ]
    }

    /// Fresh unique temp dir; caller removes it.
    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "gorm-generate-export-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn writes_files_with_directory_structure() {
        let dir = temp_dir("structure");
        export_project(&dir, &files(), false).expect("should export");

        let model = dir.join("internal/model/user.go");
        let repo = dir.join("internal/repository/user_repo.go");
        assert!(model.exists(), "model file written");
        assert!(repo.exists(), "repo file written");
        assert_eq!(std::fs::read_to_string(&model).unwrap(), "package model\n");
        assert_eq!(
            std::fs::read_to_string(&repo).unwrap(),
            "package repository\n"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn existing_files_conflict_without_overwrite() {
        let dir = temp_dir("conflict");
        export_project(&dir, &files(), false).expect("first export");

        let err = export_project(&dir, &files(), false).expect_err("should conflict");
        match err {
            ExportError::Conflicts(paths) => {
                assert!(paths.contains(&"internal/model/user.go".to_string()));
                assert_eq!(paths.len(), 2, "both files reported, got: {paths:?}");
            }
            other => panic!("expected Conflicts, got {other:?}"),
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn overwrite_replaces_existing_files() {
        let dir = temp_dir("overwrite");
        export_project(&dir, &files(), false).expect("first export");

        let changed = vec![GeneratedFile {
            path: "internal/model/user.go".to_string(),
            content: "package model // v2\n".to_string(),
            table: "users".to_string(),
        }];
        export_project(&dir, &changed, true).expect("overwrite allowed");

        assert_eq!(
            std::fs::read_to_string(dir.join("internal/model/user.go")).unwrap(),
            "package model // v2\n"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_absolute_paths() {
        let dir = temp_dir("absolute");
        let bad = vec![GeneratedFile {
            path: "/etc/passwd".to_string(),
            content: "x".to_string(),
            table: "users".to_string(),
        }];
        let err = export_project(&dir, &bad, false).expect_err("absolute rejected");
        assert!(matches!(err, ExportError::UnsafePath(_)), "got {err:?}");
        assert!(!dir.exists(), "nothing written on unsafe path");
    }

    #[test]
    fn rejects_parent_traversal() {
        let dir = temp_dir("traversal");
        let bad = vec![GeneratedFile {
            path: "../escape.go".to_string(),
            content: "x".to_string(),
            table: "users".to_string(),
        }];
        let err = export_project(&dir, &bad, false).expect_err("traversal rejected");
        assert!(matches!(err, ExportError::UnsafePath(_)), "got {err:?}");
        assert!(!dir.exists(), "nothing written on unsafe path");
    }

    #[test]
    fn rejects_relative_root() {
        // A relative root would resolve against the process CWD, so it must be
        // refused before any path is joined or any directory is created.
        let root = "gorm-generate-relative-root-should-not-exist/root";
        let err = export_project(Path::new(root), &files(), false)
            .expect_err("relative root rejected");
        match err {
            ExportError::UnsafePath(path) => {
                assert_eq!(path, root, "error must name the rejected root");
            }
            other => panic!("expected UnsafePath, got {other:?}"),
        }
        assert!(
            !Path::new("gorm-generate-relative-root-should-not-exist").exists(),
            "nothing written relative to CWD"
        );
    }
}
