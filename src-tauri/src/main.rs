// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri_plugin_dialog::DialogExt;

/// Request payload from the frontend; mirrors gorm_generate_core options.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerateRequestPayload {
    ddl: String,
    dialect: String,
    module_path: String,
    cache_enabled: bool,
    cache_ttl_seconds: u32,
    tinyint_as_bool: bool,
}

/// Response payload; files/tables/warnings pass straight through the core.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedProjectPayload {
    files: Vec<GeneratedFilePayload>,
    tables: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct GeneratedFilePayload {
    path: String,
    content: String,
    /// Source table, used by the UI to filter files by selected tables.
    #[serde(default)]
    table: String,
}

/// IPC layer only forwards: parsing and generation live in `gorm-generate-core`.
#[tauri::command]
fn ping() -> String {
    gorm_generate_core::ping()
}

/// Thin wrapper over the core `generate` seam. Returns a tagged error so the
/// UI can tell a broken DDL from invalid options without interpreting a
/// sentinel value.
#[tauri::command(async)]
fn generate_project(
    request: GenerateRequestPayload,
) -> Result<GeneratedProjectPayload, GenerateErrorPayload> {
    let dialect = match request.dialect.as_str() {
        "mysql" => gorm_generate_core::generate::Dialect::MySql,
        "postgres" => gorm_generate_core::generate::Dialect::PostgreSql,
        other => {
            return Err(GenerateErrorPayload::Validation {
                message: format!("unsupported dialect `{other}`"),
            })
        }
    };
    let core_request = gorm_generate_core::GenerateRequest {
        ddl: request.ddl,
        dialect,
        module_path: request.module_path,
        cache_enabled: request.cache_enabled,
        cache_ttl_seconds: request.cache_ttl_seconds,
        tinyint_as_bool: request.tinyint_as_bool,
    };
    gorm_generate_core::generate(&core_request)
        .map(|project| GeneratedProjectPayload {
            files: project
                .files
                .into_iter()
                .map(|f| GeneratedFilePayload {
                    path: f.path,
                    content: f.content,
                    table: f.table,
                })
                .collect(),
            tables: project.tables,
            warnings: project.warnings,
        })
        .map_err(GenerateErrorPayload::from)
}

/// Structured generation failure for the frontend, internally tagged by `kind`.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum GenerateErrorPayload {
    Parse {
        #[serde(rename = "statementIndex")]
        statement_index: usize,
        snippet: String,
    },
    Validation {
        message: String,
    },
}

impl From<gorm_generate_core::GenerateError> for GenerateErrorPayload {
    fn from(error: gorm_generate_core::GenerateError) -> Self {
        match error {
            gorm_generate_core::GenerateError::Parse {
                statement_index,
                snippet,
            } => GenerateErrorPayload::Parse {
                statement_index,
                snippet,
            },
            gorm_generate_core::GenerateError::Validation(message) => {
                GenerateErrorPayload::Validation { message }
            }
        }
    }
}

/// Result of an export attempt. `conflicts` is non-empty when existing files
/// blocked the write and the caller must confirm before retrying; otherwise
/// `error` carries a readable message.
#[derive(Serialize)]
struct ExportResultPayload {
    conflicts: Vec<String>,
    error: Option<String>,
}

/// Directory the user confirmed through the native folder picker.
///
/// The renderer can ask for the picker, but it never supplies a path itself:
/// `export_project` only writes into this slot, so an injected or buggy
/// frontend cannot aim writes at an arbitrary absolute directory.
#[derive(Default)]
struct ExportRoot(Mutex<Option<PathBuf>>);

/// Only a root confirmed by the picker may be written to.
fn confirmed_root(root: &Mutex<Option<PathBuf>>) -> Result<PathBuf, String> {
    root.lock()
        .map_err(|_| "export root lock poisoned".to_string())?
        .clone()
        .ok_or_else(|| "No export directory selected".to_string())
}

/// Opens the native folder picker and remembers the choice.
///
/// Returns `None` when the user cancels, leaving any previous root in place.
/// Blocking on purpose: the command runs off the main thread, and the dialog
/// itself is dispatched to the main thread by the plugin.
#[tauri::command]
async fn pick_export_dir(
    app: tauri::AppHandle,
    state: tauri::State<'_, ExportRoot>,
) -> Result<Option<String>, String> {
    let Some(picked) = app
        .dialog()
        .file()
        .set_title("Export to directory")
        .blocking_pick_folder()
    else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    let display = path.display().to_string();
    *state
        .0
        .lock()
        .map_err(|_| "export root lock poisoned".to_string())? = Some(path);
    Ok(Some(display))
}

/// Thin wrapper over the core `export_project` seam. Writes under the root the
/// picker last returned; rejects absolute/`..` file paths; refuses to clobber
/// existing files unless `overwrite` is true.
#[tauri::command(async)]
fn export_project(
    state: tauri::State<'_, ExportRoot>,
    files: Vec<GeneratedFilePayload>,
    overwrite: bool,
) -> ExportResultPayload {
    let root = match confirmed_root(&state.0) {
        Ok(root) => root,
        Err(error) => {
            return ExportResultPayload {
                conflicts: Vec::new(),
                error: Some(error),
            }
        }
    };

    let generated: Vec<gorm_generate_core::GeneratedFile> = files
        .into_iter()
        .map(|f| gorm_generate_core::GeneratedFile {
            path: f.path,
            content: f.content,
            table: f.table,
        })
        .collect();

    match gorm_generate_core::export_project(&root, &generated, overwrite) {
        Ok(()) => ExportResultPayload {
            conflicts: Vec::new(),
            error: None,
        },
        Err(gorm_generate_core::ExportError::Conflicts(paths)) => ExportResultPayload {
            conflicts: paths,
            error: None,
        },
        Err(other) => ExportResultPayload {
            conflicts: Vec::new(),
            error: Some(other.to_string()),
        },
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(ExportRoot::default())
        .invoke_handler(tauri::generate_handler![
            ping,
            generate_project,
            export_project,
            pick_export_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_requires_a_picker_confirmed_root() {
        let root = Mutex::new(None);
        assert_eq!(
            confirmed_root(&root).unwrap_err(),
            "No export directory selected"
        );

        *root.lock().unwrap() = Some(PathBuf::from("/tmp/export"));
        assert_eq!(confirmed_root(&root).unwrap(), PathBuf::from("/tmp/export"));
    }
}
