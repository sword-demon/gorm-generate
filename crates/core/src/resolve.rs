//! Resolved table facts, the single owner of Go names, types, imports and
//! cache keys (ADR-0003).
//!
//! `resolve_table` runs after parsing and before any rendering. It is total:
//! invalid identifiers still resolve, and the following validation pass rejects
//! them. Renderers read this value and never derive types or names themselves.

use crate::naming::{field_name, snake_case, struct_name};
use crate::parser::{
    self, is_auto_increment, is_generated_expression, is_identity, is_serial, ParsedTable,
};
use crate::types::{classify_mysql_type, map_type};

/// Conditional imports a generated file may need.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Imports {
    pub time: bool,
    pub json: bool,
    pub gorm: bool,
}

/// One resolved column: its Go shape and the flags the model tag needs.
pub(crate) struct Column {
    pub sql_name: String,
    pub go_name: String,
    pub go_type: String,
    /// Effective nullability: false for NOT NULL and for every primary key.
    pub nullable: bool,
    pub auto_increment: bool,
    pub generated: bool,
    pub comment: Option<String>,
    pub needs_time: bool,
    pub needs_json: bool,
}

/// A table resolved to the facts every renderer needs.
pub(crate) struct Table {
    pub sql_name: String,
    pub struct_name: String,
    pub file_base: String,
    pub impl_type_base: String,
    pub columns: Vec<Column>,
    /// Indices into `columns`, in primary-key declaration order.
    pub pk_indices: Vec<usize>,
    /// Import needs over all columns (the model's field types).
    pub imports: Imports,
    /// `users:%v` / `order_items:%v:%v`; None without a primary key.
    pub cache_key_format: Option<String>,
    /// Per-table warnings, without the table-name prefix.
    pub warnings: Vec<String>,
    pub comment: Option<String>,
}

impl Table {
    pub(crate) fn has_layers(&self) -> bool {
        !self.pk_indices.is_empty()
    }

    /// Primary-key columns in declaration order.
    pub(crate) fn pk(&self) -> impl Iterator<Item = &Column> + '_ {
        self.pk_indices.iter().map(|&index| &self.columns[index])
    }

    /// Import needs of the PK parameter list that repository and service
    /// render into their method signatures. Narrower than `imports`, which
    /// covers every model field.
    pub(crate) fn pk_imports(&self) -> Imports {
        let mut imports = Imports::default();
        for column in self.pk() {
            imports.time |= column.needs_time;
            imports.json |= column.needs_json;
        }
        imports
    }
}

/// Resolves one parsed table. Total by design: never errors.
pub(crate) fn resolve_table(table: &ParsedTable, tinyint_as_bool: bool) -> Table {
    let struct_name = struct_name(&table.name);
    let file_base = snake_case(&struct_name);
    let impl_type_base = struct_name.to_lowercase();

    // Primary-key order follows the DDL, not column order: the where chain,
    // the List order and the cache key all depend on it.
    let pk_indices: Vec<usize> = table
        .primary_key
        .iter()
        .filter_map(|pk| table.columns.iter().position(|c| c.name.value == *pk))
        .collect();
    let pk_index_set: std::collections::HashSet<usize> = pk_indices.iter().copied().collect();

    let mut imports = Imports::default();
    let mut warnings = Vec::new();
    let mut columns = Vec::with_capacity(table.columns.len());

    for (index, col) in table.columns.iter().enumerate() {
        let is_pk = pk_index_set.contains(&index);
        // A column is nullable unless declared NOT NULL. Primary keys are
        // never nullable regardless of declaration.
        let declared_not_null = col
            .options
            .iter()
            .any(|opt| matches!(opt.option, sqlparser::ast::ColumnOption::NotNull));
        let nullable = !declared_not_null && !is_pk;

        // Spec: deleted_at -> gorm.DeletedAt (soft delete). Overrides the
        // plain time.Time mapping; a primary key keeps its real type.
        let (go_type, needs_time, needs_json) =
            if col.name.value.eq_ignore_ascii_case("deleted_at") && !is_pk {
                imports.gorm = true;
                ("gorm.DeletedAt".to_string(), false, false)
            } else {
                let (base, unsigned, width) = classify_mysql_type(&col.data_type);
                let mapped = map_type(&base, width, unsigned, nullable, tinyint_as_bool);
                if let Some(warning) = mapped.warning {
                    warnings.push(warning);
                }
                (mapped.go_type, mapped.needs_time, mapped.needs_json)
            };
        imports.time |= needs_time;
        imports.json |= needs_json;

        columns.push(Column {
            sql_name: col.name.value.clone(),
            go_name: field_name(&col.name.value),
            go_type,
            nullable,
            // MySQL AUTO_INCREMENT, the PG SERIAL family, or GENERATED AS
            // IDENTITY all mean auto-increment.
            auto_increment: is_auto_increment(col) || is_serial(&col.data_type) || is_identity(col),
            generated: is_generated_expression(col),
            comment: parser::column_comment(col),
            needs_time,
            needs_json,
        });
    }

    let cache_key_format = if pk_indices.is_empty() {
        None
    } else {
        Some(format!(
            "{}:{}",
            table.name,
            pk_indices
                .iter()
                .map(|_| "%v")
                .collect::<Vec<_>>()
                .join(":")
        ))
    };

    Table {
        sql_name: table.name.clone(),
        struct_name,
        file_base,
        impl_type_base,
        columns,
        pk_indices,
        imports,
        cache_key_format,
        warnings,
        comment: table.comment.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::Dialect;

    fn resolve(ddl: &str) -> Table {
        let script = parser::parse_ddl(ddl, Dialect::MySql).expect("ddl should parse");
        resolve_table(&script.tables[0], false)
    }

    fn column<'a>(table: &'a Table, name: &str) -> &'a Column {
        table
            .columns
            .iter()
            .find(|c| c.sql_name == name)
            .unwrap_or_else(|| panic!("column {name} not found"))
    }

    #[test]
    fn maps_columns_and_aggregates_imports_once() {
        let table = resolve(
            "CREATE TABLE `t` (`id` BIGINT NOT NULL, `at` DATETIME NOT NULL, \
             `doc` JSON NOT NULL, `blob` BLOB NOT NULL, `note` VARCHAR(10) NULL, \
             PRIMARY KEY (`id`));",
        );

        assert_eq!(column(&table, "id").go_type, "int64");
        assert_eq!(column(&table, "at").go_type, "time.Time");
        assert_eq!(column(&table, "doc").go_type, "json.RawMessage");
        assert_eq!(column(&table, "blob").go_type, "[]byte");
        assert_eq!(column(&table, "note").go_type, "*string");
        assert!(table.imports.time);
        assert!(table.imports.json);
        assert!(!table.imports.gorm);
        // Repository/service params only carry PK types, so their import need
        // is narrower than the model's.
        assert!(!table.pk_imports().time);
        assert!(!table.pk_imports().json);
    }

    #[test]
    fn primary_key_columns_are_never_nullable_even_when_declared_null() {
        let table = resolve("CREATE TABLE `t` (`id` INT NULL, PRIMARY KEY (`id`));");
        let id = column(&table, "id");
        assert_eq!(id.go_type, "int32");
        assert!(!id.nullable);
    }

    #[test]
    fn deleted_at_becomes_gorm_soft_delete_except_for_primary_keys() {
        let table = resolve(
            "CREATE TABLE `t` (`id` BIGINT NOT NULL, `deleted_at` DATETIME NULL, \
             PRIMARY KEY (`id`));",
        );
        let deleted = column(&table, "deleted_at");
        assert_eq!(deleted.go_type, "gorm.DeletedAt");
        assert!(deleted.nullable);
        assert!(table.imports.gorm);
        assert!(!table.pk_imports().gorm);

        let pk =
            resolve("CREATE TABLE `t` (`deleted_at` BIGINT NOT NULL, PRIMARY KEY (`deleted_at`));");
        assert_eq!(column(&pk, "deleted_at").go_type, "int64");
        assert!(!pk.imports.gorm);
    }

    #[test]
    fn primary_key_indices_follow_declaration_order() {
        let table = resolve(
            "CREATE TABLE `order_items` (`order_id` BIGINT NOT NULL, `item_id` BIGINT NOT NULL, \
             PRIMARY KEY (`item_id`, `order_id`));",
        );
        let names: Vec<&str> = table.pk().map(|c| c.sql_name.as_str()).collect();
        assert_eq!(names, vec!["item_id", "order_id"]);
        assert_eq!(table.cache_key_format.as_deref(), Some("order_items:%v:%v"));
    }

    #[test]
    fn cache_key_format_is_absent_without_a_primary_key() {
        let table = resolve("CREATE TABLE `logs` (`msg` TEXT NOT NULL);");
        assert_eq!(table.cache_key_format, None);
        assert!(!table.has_layers());
    }

    #[test]
    fn unknown_type_warns_without_a_table_prefix() {
        let table = resolve(
            "CREATE TABLE `t` (`id` BIGINT NOT NULL, `x` WEIRDTYPE NOT NULL, \
             PRIMARY KEY (`id`));",
        );
        let warning = table
            .warnings
            .iter()
            .find(|w| w.contains("weirdtype"))
            .expect("unknown type should warn");
        assert!(
            warning.starts_with("unsupported column type"),
            "prefix belongs to the project layer: {warning}"
        );
    }

    #[test]
    fn names_are_derived_once_from_the_table_name() {
        let table =
            resolve("CREATE TABLE `user_orders` (`id` BIGINT NOT NULL, PRIMARY KEY (`id`));");
        assert_eq!(table.struct_name, "UserOrder");
        assert_eq!(table.file_base, "user_order");
        assert_eq!(table.impl_type_base, "userorder");
    }

    #[test]
    fn invalid_identifiers_still_resolve_for_the_validation_pass() {
        let table = resolve("CREATE TABLE `t` (`_1` INT NOT NULL);");
        assert_eq!(column(&table, "_1").go_name, "1");
    }

    #[test]
    fn tinyint_as_bool_reaches_the_column() {
        let script = parser::parse_ddl(
            "CREATE TABLE `t` (`flag` TINYINT(1) NOT NULL);",
            Dialect::MySql,
        )
        .expect("ddl should parse");
        let table = resolve_table(&script.tables[0], true);
        assert_eq!(column(&table, "flag").go_type, "bool");
    }

    #[test]
    fn postgres_serial_is_auto_increment() {
        let script = parser::parse_ddl(
            "CREATE TABLE users (id BIGSERIAL PRIMARY KEY, name TEXT NOT NULL);",
            Dialect::PostgreSql,
        )
        .expect("ddl should parse");
        let table = resolve_table(&script.tables[0], false);
        let id = column(&table, "id");
        assert_eq!(id.go_type, "int64");
        assert!(id.auto_increment);
        assert!(!table.pk_imports().time);
    }
}
