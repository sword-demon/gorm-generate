//! Generation core for the SQL DDL -> Go layered code generator.
//!
//! Parsing and code generation live here, independent of Tauri, so the core
//! stays testable without a webview.

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
