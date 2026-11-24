//! `generate(request) -> GeneratedProject`: the pure generation seam.

use crate::layers;
use crate::naming::{field_name, snake_case, struct_name};
use crate::parser::{self, is_auto_increment, is_identity, is_serial, is_generated_expression};
use crate::types::column_type;

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

    let mut files = Vec::new();
    let mut table_names = Vec::new();
    let mut warnings = Vec::new();

    // File paths and unexported impl type names derive from the struct name
    // differently: `snake_case` keeps word boundaries, the impl type lowercases
    // the whole name. Both must be unique or two tables collide in one package.
    let mut file_keys = std::collections::HashSet::new();
    let mut type_keys = std::collections::HashSet::new();
    for table in &script.tables {
        validate_table(table)?;
        let file_key = snake_case(&struct_name(&table.name));
        // Only tables with a primary key get repo/service/cache impl types.
        let type_key = struct_name(&table.name).to_lowercase();
        let has_layers = !table.primary_key.is_empty();
        if !file_keys.insert(file_key) || (has_layers && !type_keys.insert(type_key)) {
            return Err(invalid(format!("table `{}` conflicts with another generated type or path", table.name)));
        }
        let (file, table_warnings) = render_model(table, request);
        warnings.extend(table_warnings);
        table_names.push(table.name.clone());
        files.push(file);

        // Spec: tables without a primary key only get a model, plus a
        // warning; repo/service/cache are not generated for them.
        if table.primary_key.is_empty() {
            warnings.push(format!(
                "table `{}` has no primary key; only the model was generated",
                table.name
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

/// Renders `internal/model/<table>.go` for one parsed table.
fn render_model(
    table: &parser::ParsedTable,
    request: &GenerateRequest,
) -> (GeneratedFile, Vec<String>) {
    let struct_name = struct_name(&table.name);
    let mut body = String::new();
    let mut warnings = Vec::new();
    let mut needs_time_import = false;
    let mut needs_json_import = false;
    let mut needs_gorm_import = false;

    for col in &table.columns {

        // A column is nullable unless declared NOT NULL. Primary keys are
        // never nullable regardless of declaration.
        let not_null = col
            .options
            .iter()
            .any(|opt| matches!(opt.option, sqlparser::ast::ColumnOption::NotNull));
        let is_pk = table.primary_key.iter().any(|pk| *pk == col.name.value);
        let nullable = !not_null && !is_pk;

        let mapped = column_type(col, nullable, request.tinyint_as_bool, is_pk);
        if let Some(w) = mapped.warning {
            warnings.push(format!("{}: {}", table.name, w));
        }

        let mut gorm_parts = vec![format!("column:{}", col.name.value)];
        if is_pk {
            gorm_parts.push("primaryKey".to_string());
        }
        // MySQL AUTO_INCREMENT token, PG SERIAL family, or GENERATED AS
        // IDENTITY all mean auto-increment.
        if is_auto_increment(col) || is_serial(&col.data_type) || is_identity(col) {
            gorm_parts.push("autoIncrement".to_string());
        }
        if is_generated_expression(col) {
            gorm_parts.push("->".to_string());
        }
        if !nullable {
            gorm_parts.push("not null".to_string());
        }
        let gorm_tag = gorm_parts.join(";");

        // Spec: deleted_at -> gorm.DeletedAt (soft delete). Overrides the
        // plain time.Time mapping; gorm tag stays structural only.
        let go_type = if col.name.value.eq_ignore_ascii_case("deleted_at") && !is_pk {
            needs_gorm_import = true;
            "gorm.DeletedAt".to_string()
        } else {
            if mapped.go_type.contains("time.Time") {
                needs_time_import = true;
            }
            if mapped.go_type.contains("json.RawMessage") {
                needs_json_import = true;
            }
            mapped.go_type
        };

        // Spec: DDL comments are preserved as Go comments, verbatim. Line
        // comment (`//`) cannot span lines, so newlines collapse to spaces.
        if let Some(comment) = parser::column_comment(col) {
            body.push_str(&format!("\t// {}\n", comment.replace(['\n', '\r'], " ")));
        }
        body.push_str(&format!(
            "\t{field} {go_type} `gorm:\"{gorm_tag}\" json:\"{json}\"`\n",
            field = field_name(&col.name.value),
            go_type = go_type,
            json = col.name.value,
        ));
    }

    // Build the import block from the flags; gorm is only needed for
    // gorm.DeletedAt (soft delete).
    let mut import_names: Vec<&str> = Vec::new();
    if needs_json_import {
        import_names.push("\"encoding/json\"");
    }
    if needs_gorm_import {
        import_names.push("\"gorm.io/gorm\"");
    }
    if needs_time_import {
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
        recv = struct_name.chars().next().map(|c| c.to_ascii_lowercase()).unwrap_or('x'),
        struct_name = struct_name,
        table = table.name,
    );

    (
        GeneratedFile {
            // File follows the struct name's singular form: users -> user.go.
            path: format!(
                "internal/model/{}.go",
                snake_case(&struct_name)
            ),
            content,
            table: table.name.clone(),
        },
        warnings,
    )
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

fn validate_table(table: &parser::ParsedTable) -> Result<(), ParseError> {
    fn safe_name(name: &str) -> bool {
        !name.is_empty() && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') && !name.as_bytes()[0].is_ascii_digit()
    }
    // Validate the generated identifier too: `_1` is a legal SQL identifier
    // that `pascal_case` sanitizes to `1`, which is not a Go identifier.
    fn valid_ident_start(ident: &str) -> bool {
        ident.bytes().next().is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
    }
    if !safe_name(&table.name) || !valid_ident_start(&struct_name(&table.name)) {
        return Err(invalid(format!("unsupported table identifier `{}`; use letters, digits and underscores", table.name)));
    }
    let mut fields = std::collections::HashSet::new();
    for col in &table.columns {
        let field = field_name(&col.name.value);
        if !safe_name(&col.name.value) || !valid_ident_start(&field) || field == "TableName" || !fields.insert(field.to_lowercase()) {
            return Err(invalid(format!("column `{}` in table `{}` has an unsupported or conflicting Go field name", col.name.value, table.name)));
        }
    }
    Ok(())
}
