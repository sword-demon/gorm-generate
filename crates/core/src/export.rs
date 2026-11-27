//! Writes a generated project to a chosen directory (spec: 导出契约).
//!
//! Paths come from the generator and are project-relative. Anything that
//! escapes the target directory (absolute paths, `..`) is rejected.
//!
//! Export is split in two: `plan_export` decides *where* every file goes
//! (pure — no filesystem access), and `write_plan` puts it there. All the
//! path algebra is tested through the plan alone (ADR-0006).

use std::path::{Component, Path, PathBuf};

use crate::generate::GeneratedFile;

/// Why an export could not proceed. The variants are the core's own wire
/// format, like `GenerateError` (ADR-0009): each carries the raw fact, and the
/// caller words the message.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "camelCase"))]
pub enum ExportError {
    /// A file path is absolute or contains a `..` component, the root is not
    /// absolute, or a path component is a symlink.
    UnsafePath { path: String },
    /// Two generated files resolve to the same target.
    DuplicatePath { path: String },
    /// Files already exist and `overwrite` was false.
    Conflicts { paths: Vec<String> },
    /// Filesystem failure while writing.
    Io { message: String },
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
    write_plan(&plan_export(dir, files)?, overwrite)
}

/// One resolved write: the declared path (for error messages and conflict
/// reporting), its absolute target under the plan's root, and the bytes.
#[derive(Debug)]
struct PlannedWrite<'a> {
    path: &'a str,
    target: PathBuf,
    content: &'a str,
}

/// Every write of an export, resolved and validated. Building one touches no
/// filesystem.
#[derive(Debug)]
struct WritePlan<'a> {
    root: PathBuf,
    writes: Vec<PlannedWrite<'a>>,
}

/// Decides where each file goes. Pure: the root must be absolute, each path is
/// joined only if it stays inside the root, and two files may not share a
/// target.
fn plan_export<'a>(dir: &Path, files: &'a [GeneratedFile]) -> Result<WritePlan<'a>, ExportError> {
    if !dir.is_absolute() {
        return Err(ExportError::UnsafePath {
            path: dir.display().to_string(),
        });
    }
    let mut seen = std::collections::HashSet::new();
    let mut writes = Vec::with_capacity(files.len());
    for file in files {
        let target = safe_join(dir, &file.path)?;
        if !seen.insert(target.clone()) {
            return Err(ExportError::DuplicatePath {
                path: file.path.clone(),
            });
        }
        writes.push(PlannedWrite {
            path: &file.path,
            target,
            content: &file.content,
        });
    }
    Ok(WritePlan {
        root: dir.to_path_buf(),
        writes,
    })
}

/// Carries out a plan. The chosen root may itself be an OS alias; only
/// descendants are rejected. These checks prevent known symlink escapes, not
/// concurrent path replacement.
fn write_plan(plan: &WritePlan, overwrite: bool) -> Result<(), ExportError> {
    let root = &plan.root;
    for write in &plan.writes {
        reject_links(root, &write.target)?;
    }
    if !overwrite {
        let conflicts: Vec<String> = plan
            .writes
            .iter()
            .filter(|write| write.target.exists())
            .map(|write| write.path.to_string())
            .collect();
        if !conflicts.is_empty() {
            return Err(ExportError::Conflicts { paths: conflicts });
        }
    }
    // Stage every file first. A failed write leaves all original files intact.
    // Persistence is atomic per file; the batch is not a filesystem transaction.
    let mut staged = Vec::new();
    for write in &plan.writes {
        let path = &write.target;
        let parent = path.parent().ok_or_else(|| ExportError::UnsafePath {
            path: path.display().to_string(),
        })?;
        std::fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
        reject_links(root, path)?;
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
        temp.write_all(write.content.as_bytes())
            .map_err(|e| io_error(path, e))?;
        temp.as_file().sync_all().map_err(|e| io_error(path, e))?;
        staged.push((temp, path));
    }
    for (temp, path) in staged {
        reject_links(root, path)?;
        if overwrite {
            temp.persist(path).map_err(|e| io_error(path, e.error))?;
        } else {
            temp.persist_noclobber(path)
                .map_err(|e| io_error(path, e.error))?;
        }
    }
    Ok(())
}

fn io_error(path: &Path, error: std::io::Error) -> ExportError {
    ExportError::Io {
        message: format!("{}: {error}", path.display()),
    }
}

fn reject_links(root: &Path, path: &Path) -> Result<(), ExportError> {
    let mut current = root.to_path_buf();
    let relative = path
        .strip_prefix(root)
        .map_err(|_| ExportError::UnsafePath {
            path: path.display().to_string(),
        })?;
    for part in relative.components() {
        current.push(part);
        match std::fs::symlink_metadata(&current) {
            Ok(meta) => {
                #[allow(unused_mut)]
                let mut link = meta.file_type().is_symlink();
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    link |= meta.file_attributes() & 0x400 != 0;
                }
                if link {
                    return Err(ExportError::UnsafePath {
                        path: current.display().to_string(),
                    });
                }
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
        return Err(ExportError::UnsafePath {
            path: relative.to_string(),
        });
    }
    for component in rel.components() {
        match component {
            Component::Normal(_) => {}
            _ => {
                return Err(ExportError::UnsafePath {
                    path: relative.to_string(),
                })
            }
        }
    }
    Ok(dir.join(rel))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::FileKind;

    fn files() -> Vec<GeneratedFile> {
        vec![
            GeneratedFile {
                path: "internal/model/user.go".to_string(),
                content: "package model\n".to_string(),
                table: "users".to_string(),
                kind: FileKind::Model,
            },
            GeneratedFile {
                path: "internal/repository/user_repo.go".to_string(),
                content: "package repository\n".to_string(),
                table: "users".to_string(),
                kind: FileKind::Repository,
            },
        ]
    }

    fn file(path: &str) -> GeneratedFile {
        GeneratedFile {
            path: path.to_string(),
            content: "x".to_string(),
            table: "users".to_string(),
            kind: FileKind::Model,
        }
    }

    /// Fresh unique temp dir; caller removes it.
    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("gorm-generate-export-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn plan_resolves_every_target_under_the_root() {
        let root = temp_dir("plan-targets");
        let files = files();
        let plan = plan_export(&root, &files).expect("plan should resolve");
        let targets: Vec<&Path> = plan.writes.iter().map(|w| w.target.as_path()).collect();
        assert_eq!(
            targets,
            [
                root.join("internal/model/user.go").as_path(),
                root.join("internal/repository/user_repo.go").as_path(),
            ]
        );
        assert!(!root.exists(), "planning must not touch the filesystem");
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
            ExportError::Conflicts { paths } => {
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
            kind: FileKind::Model,
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
        let err = plan_export(&dir, &[file("/etc/passwd")]).expect_err("absolute rejected");
        assert!(matches!(err, ExportError::UnsafePath { .. }), "got {err:?}");
        assert!(!dir.exists(), "nothing written on unsafe path");
    }

    #[test]
    fn rejects_parent_traversal() {
        let dir = temp_dir("traversal");
        let err = plan_export(&dir, &[file("../escape.go")]).expect_err("traversal rejected");
        assert!(matches!(err, ExportError::UnsafePath { .. }), "got {err:?}");
        assert!(!dir.exists(), "nothing written on unsafe path");
    }

    #[test]
    fn rejects_duplicate_paths() {
        let dir = temp_dir("duplicate");
        let err = plan_export(
            &dir,
            &[
                file("internal/model/user.go"),
                file("internal/model/user.go"),
            ],
        )
        .expect_err("duplicate rejected");
        match err {
            ExportError::DuplicatePath { path } => {
                assert_eq!(path, "internal/model/user.go");
            }
            other => panic!("expected DuplicatePath, got {other:?}"),
        }
    }

    #[test]
    fn rejects_relative_root() {
        // A relative root would resolve against the process CWD, so it must be
        // refused before any path is joined or any directory is created.
        let root = "gorm-generate-relative-root-should-not-exist/root";
        let err = plan_export(Path::new(root), &files()).expect_err("relative root rejected");
        match err {
            ExportError::UnsafePath { path } => {
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
