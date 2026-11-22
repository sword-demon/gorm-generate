// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// IPC layer only forwards: parsing and generation live in `gorm-generate-core`.
#[tauri::command]
fn ping() -> String {
    gorm_generate_core::ping()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
