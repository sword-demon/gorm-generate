//! Every renderer: the model plus the repository, service and cache layers
//! (spec: 模型 / 仓储契约 / 服务契约 / 缓存契约). `table` is this module's
//! interface: it decides what one table yields, including its warnings and its
//! third-party dependencies. Every fact comes from the resolved table; each
//! layer lives in its own file and only formats text.

use crate::generate::{GenerateRequest, GeneratedFile};
use crate::resolve::Table;

mod cache;
mod contract;
mod model;
mod repository;
mod service;

use contract::Contract;

const GORM: &str = "gorm.io/gorm";
const REDIS: &str = "github.com/redis/go-redis/v9";

/// What one table contributes to the project.
pub(crate) struct TableOutput {
    pub(crate) files: Vec<GeneratedFile>,
    /// Final warning text: prefixed with the table name, or self-describing.
    pub(crate) warnings: Vec<String>,
    /// Third-party modules the emitted files import, deduplicated, in
    /// declaration order.
    pub(crate) dependencies: Vec<&'static str>,
}

/// Decides what one table produces. Tables without a primary key only get a
/// model, plus a warning; their repository/service/cache are not generated.
pub(crate) fn table(table: &Table, request: &GenerateRequest) -> TableOutput {
    let mut output = TableOutput {
        files: vec![model::render(table)],
        warnings: table
            .warnings
            .iter()
            .map(|warning| format!("{}: {}", table.sql_name, warning))
            .collect(),
        dependencies: Vec::new(),
    };
    if table.imports.gorm {
        push_dependency(&mut output.dependencies, GORM);
    }
    if !table.has_layers() {
        output.warnings.push(format!(
            "table `{}` has no primary key; only the model was generated",
            table.sql_name
        ));
        return output;
    }
    let contract = Contract::new(table);
    output
        .files
        .push(repository::render(table, request, &contract));
    output
        .files
        .push(service::render(table, request, &contract));
    push_dependency(&mut output.dependencies, GORM);
    if request.cache_enabled {
        output.files.push(cache::render(table, request, &contract));
        push_dependency(&mut output.dependencies, REDIS);
    }
    output
}

fn push_dependency(dependencies: &mut Vec<&'static str>, dependency: &'static str) {
    if !dependencies.contains(&dependency) {
        dependencies.push(dependency);
    }
}

/// Unexported impl type name: `users` -> `userRepository` (repo pkg) and
/// `userService` (service pkg). The two packages are separate, so the same
/// lowercase prefix can exist in each without collision.
fn impl_type(table: &Table, suffix: &str) -> String {
    format!("{}{}", table.impl_type_base, suffix)
}

fn pk_import_fragments(table: &Table) -> (&'static str, &'static str) {
    let imports = table.pk_imports();
    let type_imports = if imports.time { "\t\"time\"\n" } else { "" };
    let json_import = if imports.json {
        "\t\"encoding/json\"\n"
    } else {
        ""
    };
    (type_imports, json_import)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{Dialect, FileKind};
    use crate::parser;
    use crate::resolve::resolve_table;

    fn request(ddl: &str) -> GenerateRequest {
        GenerateRequest {
            ddl: ddl.into(),
            dialect: Dialect::MySql,
            module_path: "example.com/app".into(),
            cache_enabled: true,
            cache_ttl_seconds: 300,
            tinyint_as_bool: false,
        }
    }

    #[test]
    fn table_owns_its_files_warnings_and_dependencies() {
        let ddl = "CREATE TABLE `users` (`id` BIGINT NOT NULL, PRIMARY KEY (`id`)); \
                   CREATE TABLE `logs` (`msg` TEXT NOT NULL);";
        let script = parser::parse_ddl(ddl, Dialect::MySql).expect("ddl should parse");
        let request = request(ddl);

        let users = table(&resolve_table(&script.tables[0], false), &request);
        let kinds: Vec<FileKind> = users.files.iter().map(|file| file.kind).collect();
        assert_eq!(
            kinds,
            [
                FileKind::Model,
                FileKind::Repository,
                FileKind::Service,
                FileKind::Cache
            ]
        );
        assert_eq!(users.dependencies, [GORM, REDIS]);
        assert!(users.warnings.is_empty());

        // No primary key: one model, no third-party imports, and the rule is
        // visible here rather than only through `generate`.
        let logs = table(&resolve_table(&script.tables[1], false), &request);
        assert_eq!(logs.files.len(), 1);
        assert_eq!(logs.files[0].kind, FileKind::Model);
        assert!(logs.dependencies.is_empty());
        assert_eq!(logs.warnings.len(), 1);
        assert!(
            logs.warnings[0].contains("no primary key"),
            "{:?}",
            logs.warnings
        );
    }
}
