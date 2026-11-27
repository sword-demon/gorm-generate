// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

use gorm_generate_core::{
    ExportError, GenerateError, GenerateRequest, GeneratedFile, GeneratedProject,
};
use serde::Serialize;
use tauri_plugin_dialog::DialogExt;

/// Thin wrapper over the core `generate` seam. The request and the tagged
/// error are core types; the core owns their wire format.
#[tauri::command(async)]
fn generate_project(request: GenerateRequest) -> Result<GeneratedProject, GenerateError> {
    gorm_generate_core::generate(&request)
}

/// Why an export command failed: a core export error, whose wire format the
/// core owns (ADR-0009), or the lack of a picker-confirmed root — a Tauri
/// precondition the core never sees.
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ExportFailure {
    Core(ExportError),
    NoRoot { kind: &'static str, message: String },
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
    files: Vec<GeneratedFile>,
    overwrite: bool,
) -> Result<(), ExportFailure> {
    let root = confirmed_root(&state.0).map_err(|message| ExportFailure::NoRoot {
        kind: "noRoot",
        message,
    })?;
    gorm_generate_core::export_project(&root, &files, overwrite).map_err(ExportFailure::Core)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // The in-app report entry opens the prefilled issue page in the
        // system browser; the webview itself never navigates away.
        .plugin(tauri_plugin_opener::init())
        .manage(ExportRoot::default())
        .invoke_handler(tauri::generate_handler![
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

    #[test]
    fn export_failures_reach_the_ui_as_one_tagged_shape() {
        // The untagged wrapper must not nest the core error: the UI branches on
        // `kind` for both sources alike.
        let core = ExportFailure::Core(ExportError::Conflicts {
            paths: vec!["a.go".into()],
        });
        assert_eq!(
            serde_json::to_value(core).unwrap(),
            serde_json::json!({"kind": "conflicts", "paths": ["a.go"]})
        );

        let no_root = ExportFailure::NoRoot {
            kind: "noRoot",
            message: "No export directory selected".into(),
        };
        assert_eq!(
            serde_json::to_value(no_root).unwrap(),
            serde_json::json!({"kind": "noRoot", "message": "No export directory selected"})
        );
    }
}
