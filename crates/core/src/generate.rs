//! `generate(request) -> GeneratedProject`: the pure generation seam.

use crate::layers;
use crate::naming::{field_name, snake_case, struct_name};
use crate::parser::{self, classify_mysql_type, is_auto_increment, is_identity, is_serial};
use crate::types::map_type;

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
    let script = parser::parse_ddl(&request.ddl, request.dialect)?;

    let mut files = Vec::new();
    let mut table_names = Vec::new();
    let mut warnings = Vec::new();

    for table in &script.tables {
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
        let (base, unsigned, width) = classify_mysql_type(&col.data_type);

        // A column is nullable unless declared NOT NULL. Primary keys are
        // never nullable regardless of declaration.
        let not_null = col
            .options
            .iter()
            .any(|opt| matches!(opt.option, sqlparser::ast::ColumnOption::NotNull));
        let is_pk = table.primary_key.iter().any(|pk| *pk == col.name.value);
        let nullable = !not_null && !is_pk;

        let mapped = map_type(&base, width, unsigned, nullable, request.tinyint_as_bool);
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
        if !nullable {
            gorm_parts.push("not null".to_string());
        }
        let gorm_tag = gorm_parts.join(";");

        // Spec: deleted_at -> gorm.DeletedAt (soft delete). Overrides the
        // plain time.Time mapping; gorm tag stays structural only.
        let go_type = if col.name.value.eq_ignore_ascii_case("deleted_at") {
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
