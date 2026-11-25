//! `generate(request) -> GeneratedProject`: the pure generation seam.

use crate::parser;
use crate::render;
use crate::resolve::{self, Table};
use crate::validate;

#[cfg(test)]
mod tests;

/// Source database dialect of the DDL script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum Dialect {
    #[cfg_attr(feature = "serde", serde(rename = "mysql"))]
    MySql,
    #[cfg_attr(feature = "serde", serde(rename = "postgres"))]
    PostgreSql,
}

/// Everything the generator needs; UI-facing options map 1:1.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct GenerateRequest {
    pub ddl: String,
    pub dialect: Dialect,
    pub module_path: String,
    pub cache_enabled: bool,
    pub cache_ttl_seconds: u32,
    pub tinyint_as_bool: bool,
}

/// Which layer a generated file belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum FileKind {
    Model,
    Repository,
    Service,
    Cache,
}

/// One generated file, path is project-relative. `table` names the source
/// table so the UI can filter files by selected tables; `kind` names the
/// layer, so callers can address a file without knowing the order or path.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
    pub table: String,
    pub kind: FileKind,
}

/// Full output of one generation run. Pure: same request, byte-identical result.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct GeneratedProject {
    pub files: Vec<GeneratedFile>,
    pub tables: Vec<String>,
    pub warnings: Vec<String>,
    /// Third-party modules the generated files import, in `go get` order.
    /// Derived from what was emitted, not from the request switches.
    pub dependencies: Vec<String>,
}

impl GeneratedProject {
    /// The file this table produced for one layer, if any. Tables without a
    /// primary key only have a model.
    pub fn file(&self, table: &str, kind: FileKind) -> Option<&GeneratedFile> {
        self.files
            .iter()
            .find(|file| file.table == table && file.kind == kind)
    }
}

/// Why generation failed: the DDL did not parse, or the request and the
/// generated names did not validate. The distinction is a type, not a
/// sentinel value, because callers render the two differently.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "camelCase"))]
pub enum GenerateError {
    /// The DDL failed to parse. `statement_index` is 1-based.
    Parse {
        #[cfg_attr(feature = "serde", serde(rename = "statementIndex"))]
        statement_index: usize,
        snippet: String,
    },
    /// The request or the generated identifiers violate a rule.
    Validation {
        message: String,
    },
}

/// Generates the layered Go project from a DDL script. No IO.
pub fn generate(request: &GenerateRequest) -> Result<GeneratedProject, GenerateError> {
    validate::request(request)?;
    let script = parser::parse_ddl(&request.ddl, request.dialect)?;

    // Resolve every table once. Validation and all renderers read only the IR,
    // so type mapping and naming cannot drift between layers (ADR-0003).
    let tables: Vec<Table> = script
        .tables
        .iter()
        .map(|table| resolve::resolve_table(table, request.tinyint_as_bool))
        .collect();
    validate::tables(&tables)?;

    let mut files = Vec::new();
    let mut table_names = Vec::new();
    let mut warnings = Vec::new();
    let mut dependencies: Vec<String> = Vec::new();

    // One entry point per table, so what a table produces — files, warnings and
    // dependencies — lives with the renderers (ADR-0008).
    for table in &tables {
        table_names.push(table.sql_name.clone());
        let output = render::table(table, request);
        files.extend(output.files);
        warnings.extend(output.warnings);
        for dependency in output.dependencies {
            if !dependencies.iter().any(|known| known == dependency) {
                dependencies.push(dependency.to_string());
            }
        }
    }

    if script.ignored_count > 0 {
        warnings.push(format!(
            "ignored {count} non-CREATE-TABLE statement(s)",
            count = script.ignored_count
        ));
    }

    Ok(GeneratedProject {
        files,
        tables: table_names,
        warnings,
        dependencies,
    })
}

