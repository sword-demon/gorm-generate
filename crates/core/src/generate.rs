//! `generate(request) -> GeneratedProject`: the pure generation seam.

use std::collections::HashSet;

use crate::layers;
use crate::parser;
use crate::resolve::{self, Table};

#[cfg(test)]
mod tests;

/// Source database dialect of the DDL script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    MySql,
    PostgreSql,
}

/// Everything the generator needs; UI-facing options map 1:1.
#[derive(Debug)]
pub struct GenerateRequest {
    pub ddl: String,
    pub dialect: Dialect,
    pub module_path: String,
    pub cache_enabled: bool,
    pub cache_ttl_seconds: u32,
    pub tinyint_as_bool: bool,
}

/// One generated file, path is project-relative. `table` names the source
/// table so the UI can filter files by selected tables.
#[derive(Debug, PartialEq)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
    pub table: String,
}

/// Full output of one generation run. Pure: same request, byte-identical result.
#[derive(Debug)]
pub struct GeneratedProject {
    pub files: Vec<GeneratedFile>,
    pub tables: Vec<String>,
    pub warnings: Vec<String>,
}

/// Structured parse error: which statement failed and what it looked like.
#[derive(Debug)]
pub struct ParseError {
    pub statement_index: usize,
    pub snippet: String,
}

/// Generates the layered Go project from a DDL script. No IO.
pub fn generate(request: &GenerateRequest) -> Result<GeneratedProject, ParseError> {
    validate_request(request)?;
    let script = parser::parse_ddl(&request.ddl, request.dialect)?;

    // Resolve every table once. Validation and all renderers read only the IR,
    // so type mapping and naming cannot drift between layers (ADR-0003).
    let tables: Vec<Table> = script
        .tables
        .iter()
        .map(|table| resolve::resolve_table(table, request.tinyint_as_bool))
        .collect();
    validate_tables(&tables)?;

    let mut files = Vec::new();
    let mut table_names = Vec::new();
    let mut warnings = Vec::new();

    for table in &tables {
        for warning in &table.warnings {
            warnings.push(format!("{}: {}", table.sql_name, warning));
        }
        table_names.push(table.sql_name.clone());
        files.push(render_model(table));

        // Spec: tables without a primary key only get a model, plus a
        // warning; repo/service/cache are not generated for them.
        if !table.has_layers() {
            warnings.push(format!(
                "table `{}` has no primary key; only the model was generated",
                table.sql_name
            ));
            continue;
        }
        files.push(layers::render_repository(table, request));
        files.push(layers::render_service(table, request));
        if request.cache_enabled {
            files.push(layers::render_cache(table, request));
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
    })
}

/// Renders `internal/model/<table>.go`. Formatting only: every fact comes from
/// the resolved table.
fn render_model(table: &Table) -> GeneratedFile {
    let mut body = String::new();

    for (index, col) in table.columns.iter().enumerate() {
        let is_pk = table.pk_indices.contains(&index);
        let mut gorm_parts = vec![format!("column:{}", col.sql_name)];
        if is_pk {
            gorm_parts.push("primaryKey".to_string());
        }
        if col.auto_increment {
            gorm_parts.push("autoIncrement".to_string());
        }
        if col.generated {
            gorm_parts.push("->".to_string());
        }
        if !col.nullable {
            gorm_parts.push("not null".to_string());
        }
        let gorm_tag = gorm_parts.join(";");

        // Spec: DDL comments are preserved as Go comments, verbatim. Line
        // comment (`//`) cannot span lines, so newlines collapse to spaces.
        if let Some(comment) = &col.comment {
            body.push_str(&format!("\t// {}\n", comment.replace(['\n', '\r'], " ")));
        }
        body.push_str(&format!(
            "\t{field} {go_type} `gorm:\"{gorm_tag}\" json:\"{json}\"`\n",
            field = col.go_name,
            go_type = col.go_type,
            json = col.sql_name,
        ));
    }

    // Import block order is part of the golden output: json, gorm, time.
    let mut import_names: Vec<&str> = Vec::new();
    if table.imports.json {
        import_names.push("\"encoding/json\"");
    }
    if table.imports.gorm {
        import_names.push("\"gorm.io/gorm\"");
    }
    if table.imports.time {
        import_names.push("\"time\"");
    }
    let imports = match import_names.len() {
        0 => String::new(),
        1 => format!("import {}\n\n", import_names[0]),
        _ => format!(
            "import (\n{}\n)\n\n",
            import_names
                .iter()
                .map(|i| format!("\t{i}"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    };

    let table_comment = table
        .comment
        .as_ref()
        .map(|c| format!("// {}\n", c.replace(['\n', '\r'], " ")))
        .unwrap_or_default();

    let content = format!(
        "// Code generated by gorm-generate. DO NOT EDIT.\n\npackage model\n\n{imports}{table_comment}type {struct_name} struct {{\n{body}}}\n\nfunc ({recv} {struct_name}) TableName() string {{\n\treturn \"{table}\"\n}}\n",
        imports = imports,
        table_comment = table_comment,
        recv = table.struct_name.chars().next().map(|c| c.to_ascii_lowercase()).unwrap_or('x'),
        struct_name = table.struct_name,
        table = table.sql_name,
    );

    GeneratedFile {
        // File follows the struct name's singular form: users -> user.go.
        path: format!("internal/model/{}.go", table.file_base),
        content,
        table: table.sql_name.clone(),
    }
}

fn invalid(snippet: String) -> ParseError { ParseError { statement_index: 0, snippet } }

fn validate_request(request: &GenerateRequest) -> Result<(), ParseError> {
    let path = &request.module_path;
    if path.is_empty() || path.split('/').any(|part| part.is_empty() || part == "." || part == ".." || part.ends_with('.') || part.starts_with('.'))
        || !path.bytes().all(|c| c.is_ascii_alphanumeric() || b"-._~/".contains(&c)) {
        return Err(invalid("module path must be a non-empty Go import path without spaces, quotes or traversal".into()));
    }
    if request.cache_enabled && request.cache_ttl_seconds == 0 {
        return Err(invalid("cache TTL must be at least 1 second".into()));
    }
    Ok(())
}

/// Validates identifier safety and generated-name collisions over the resolved
/// tables. Different from `resolve_table`: resolution is total, this may fail.
fn validate_tables(tables: &[Table]) -> Result<(), ParseError> {
    fn safe_name(name: &str) -> bool {
        !name.is_empty() && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') && !name.as_bytes()[0].is_ascii_digit()
    }
    // Validate the generated identifier too: `_1` is a legal SQL identifier
    // that `pascal_case` sanitizes to `1`, which is not a Go identifier.
    fn valid_ident_start(ident: &str) -> bool {
        ident.bytes().next().is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
    }

    // File paths and unexported impl type names derive from the struct name
    // differently: `snake_case` keeps word boundaries, the impl type lowercases
    // the whole name. Both must be unique or two tables collide in one package.
    let mut file_keys = HashSet::new();
    let mut type_keys = HashSet::new();

    for table in tables {
        if !safe_name(&table.sql_name) || !valid_ident_start(&table.struct_name) {
            return Err(invalid(format!("unsupported table identifier `{}`; use letters, digits and underscores", table.sql_name)));
        }

        let mut fields = HashSet::new();
        for column in &table.columns {
            if !safe_name(&column.sql_name)
                || !valid_ident_start(&column.go_name)
                || column.go_name == "TableName"
                || !fields.insert(column.go_name.to_lowercase())
            {
                return Err(invalid(format!(
                    "column `{}` in table `{}` has an unsupported or conflicting Go field name",
                    column.sql_name, table.sql_name
                )));
            }
        }

        // Only tables with a primary key get repo/service/cache impl types.
        if !file_keys.insert(table.file_base.clone())
            || (table.has_layers() && !type_keys.insert(table.impl_type_base.clone()))
        {
            return Err(invalid(format!("table `{}` conflicts with another generated type or path", table.sql_name)));
        }
    }
    Ok(())
}
