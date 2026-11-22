// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};

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

#[derive(Serialize)]
struct GeneratedFilePayload {
    path: String,
    content: String,
}

/// IPC layer only forwards: parsing and generation live in `gorm-generate-core`.
#[tauri::command]
fn ping() -> String {
    gorm_generate_core::ping()
}

/// Thin wrapper over the core `generate` seam. Returns a structured error
/// (statement index + snippet) instead of stringifying, so the UI can show
/// actionable parse errors.
#[tauri::command]
fn generate_project(request: GenerateRequestPayload) -> Result<GeneratedProjectPayload, ParseErrorPayload> {
    let dialect = match request.dialect.as_str() {
        "mysql" => gorm_generate_core::generate::Dialect::MySql,
        other => {
            return Err(ParseErrorPayload {
                statement_index: 0,
                snippet: format!("unsupported dialect `{other}`"),
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
                })
                .collect(),
            tables: project.tables,
            warnings: project.warnings,
        })
        .map_err(|e| ParseErrorPayload {
            statement_index: e.statement_index,
            snippet: e.snippet,
        })
}

/// Structured parse error for the frontend.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ParseErrorPayload {
    statement_index: usize,
    snippet: String,
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping, generate_project])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
