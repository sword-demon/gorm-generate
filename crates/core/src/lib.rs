//! Generation core for the SQL DDL -> Go layered code generator.
//!
//! Parsing and code generation live here, independent of Tauri, so the core
//! stays testable without a webview.
//!
//! The re-export list below is the crate's entire public interface. Every
//! implementation module is private, so third-party types — `sqlparser`'s
//! AST in particular — never become part of it (ADR-0007).

mod export;
mod generate;
mod naming;
mod parser;
mod render;
mod resolve;
mod types;
mod validate;

pub use export::{export_project, ExportError};
pub use generate::{
    generate, Dialect, FileKind, GenerateError, GenerateRequest, GeneratedFile, GeneratedProject,
};
