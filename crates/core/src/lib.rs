//! Generation core for the SQL DDL -> Go layered code generator.
//!
//! Parsing and code generation live here, independent of Tauri, so the core
//! stays testable without a webview.

pub mod export;
pub mod generate;
pub mod layers;
pub mod naming;
pub mod parser;
mod resolve;
pub mod types;

pub use export::{export_project, ExportError};
pub use generate::{generate, FileKind, GenerateError, GenerateRequest, GeneratedFile, GeneratedProject};

/// Liveness probe used to verify the IPC path in the scaffold.
pub fn ping() -> String {
    "pong from core".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_reports_core_response() {
        assert_eq!(ping(), "pong from core");
    }
}
