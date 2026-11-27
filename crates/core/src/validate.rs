//! Request and table validation: the checks that may reject a generation run.
//! Parsing and resolution are total; this is where a request or a set of
//! resolved tables fails (spec: 校验).

use std::collections::HashSet;

use crate::generate::{GenerateError, GenerateRequest};
use crate::resolve::Table;

fn invalid(message: String) -> GenerateError {
    GenerateError::Validation { message }
}

pub(crate) fn request(request: &GenerateRequest) -> Result<(), GenerateError> {
    let path = &request.module_path;
    if path.is_empty()
        || path.split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || part.ends_with('.')
                || part.starts_with('.')
        })
        || !path
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-._~/".contains(&c))
    {
        return Err(invalid(
            "module path must be a non-empty Go import path without spaces, quotes or traversal"
                .into(),
        ));
    }
    if request.cache_enabled && request.cache_ttl_seconds == 0 {
        return Err(invalid("cache TTL must be at least 1 second".into()));
    }
    Ok(())
}

/// Validates identifier safety and generated-name collisions over the resolved
/// tables. Different from `resolve_table`: resolution is total, this may fail.
pub(crate) fn tables(tables: &[Table]) -> Result<(), GenerateError> {
    fn safe_name(name: &str) -> bool {
        !name.is_empty()
            && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
            && !name.as_bytes()[0].is_ascii_digit()
    }
    // Validate the generated identifier too: `_1` is a legal SQL identifier
    // that `pascal_case` sanitizes to `1`, which is not a Go identifier.
    fn valid_ident_start(ident: &str) -> bool {
        ident
            .bytes()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
    }

    // File paths and unexported impl type names derive from the struct name
    // differently: `snake_case` keeps word boundaries, the impl type lowercases
    // the whole name. Both must be unique or two tables collide in one package.
    let mut file_keys = HashSet::new();
    let mut type_keys = HashSet::new();

    for table in tables {
        if !safe_name(&table.sql_name) || !valid_ident_start(&table.struct_name) {
            return Err(invalid(format!(
                "unsupported table identifier `{}`; use letters, digits and underscores",
                table.sql_name
            )));
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
            return Err(invalid(format!(
                "table `{}` conflicts with another generated type or path",
                table.sql_name
            )));
        }
    }
    Ok(())
}
