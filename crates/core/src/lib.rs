//! Generation core for the SQL DDL -> Go layered code generator.
//!
//! Parsing and code generation live here, independent of Tauri, so the core
//! stays testable without a webview.

pub mod generate;
pub mod naming;
pub mod parser;
pub mod types;

pub use generate::{generate, GeneratedFile, GeneratedProject, GenerateRequest, ParseError};

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
