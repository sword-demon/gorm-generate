//! The five-method contract shared by the repository, service and cache
//! layers. Signatures and PK argument expressions are computed once here;
//! each layer only renders its own bodies.

use crate::naming::field_name;
use crate::resolve::Table;

/// One method of the layered contract, in declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Method {
    Create,
    GetByID,
    List,
    Update,
    Delete,
}

impl Method {
    pub(crate) const ALL: [Method; 5] = [
        Method::Create,
        Method::GetByID,
        Method::List,
        Method::Update,
        Method::Delete,
    ];
}

/// Signatures and PK argument expressions for one table. All three layers
/// expose the same method set, so the text is produced here exactly once.
pub(crate) struct Contract<'a> {
    table: &'a Table,
    /// `ctx context.Context, id int64` — primary-key parameters as declared.
    get_params: String,
    /// `id` — the same parameters as values, for calls and where clauses.
    get_args: String,
}

impl<'a> Contract<'a> {
    pub(crate) fn new(table: &'a Table) -> Self {
        let get_args = table
            .pk()
            .map(|pk| param_name(&pk.sql_name))
            .collect::<Vec<_>>()
            .join(", ");
        let get_params = table
            .pk()
            .map(|pk| format!("{} {}", param_name(&pk.sql_name), pk.go_type))
            .collect::<Vec<_>>()
            .join(", ");
        Self {
            table,
            get_params: format!("ctx context.Context, {get_params}"),
            get_args,
        }
    }

    pub(crate) fn model_type(&self) -> String {
        format!("model.{}", self.table.struct_name)
    }

    /// The method signature as it appears in an interface and, after the
    /// receiver, as the implementation's function header.
    pub(crate) fn signature(&self, method: Method) -> String {
        let s = self.model_type();
        match method {
            Method::Create => format!("Create(ctx context.Context, m *{s}) error"),
            Method::GetByID => format!("GetByID({}) (*{s}, error)", self.get_params),
            Method::List => format!(
                "List(ctx context.Context, page int, pageSize int) ([]*{s}, int64, error)"
            ),
            Method::Update => format!("Update(ctx context.Context, m *{s}) error"),
            Method::Delete => format!("Delete({}) error", self.get_params),
        }
    }

    /// Interface body: one indented signature per line.
    pub(crate) fn interface(&self) -> String {
        Method::ALL
            .iter()
            .map(|method| format!("\t{}", self.signature(*method)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Arguments that forward to a nested call, `ctx` included.
    pub(crate) fn forward_args(&self) -> String {
        format!("ctx, {}", self.get_args)
    }

    /// Primary-key arguments as model fields, for cache invalidation.
    pub(crate) fn model_args(&self) -> String {
        self.table
            .pk()
            .map(|pk| format!("m.{}", pk.go_name))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The same values without `ctx`, for where clauses and cache keys.
    pub(crate) fn get_args(&self) -> &str {
        &self.get_args
    }
}

/// Go-idiomatic parameter names for PK columns: `user_id` -> `userId`.
pub(crate) fn param_name(column: &str) -> String {
    let field = field_name(column);
    if field.is_empty() {
        return column.to_string();
    }
    let mut out = String::new();
    for (i, c) in field.chars().enumerate() {
        if i == 0 {
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    // An all-caps DDL identifier (`"ID"`, `"URL"`) is an initialism: Go style
    // wants `id`, not `iD`.
    if !field.chars().any(|c| c.is_lowercase()) {
        out = field.to_lowercase();
    }
    match out.as_str() {
        "break" | "default" | "func" | "interface" | "select" | "case" | "defer" | "go" | "map" | "struct" | "chan" | "else" | "goto" | "package" | "switch" | "const" | "fallthrough" | "if" | "range" | "type" | "continue" | "for" | "import" | "return" | "var" | "ctx" | "r" | "s" | "c" | "m" | "err" | "key" | "data" | "dbErr" | "encoded" | "fmt" | "time" | "json" | "model" | "repository" | "redis" | "context" => format!("{out}PK"),
        _ => out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::Dialect;
    use crate::parser;
    use crate::resolve::resolve_table;

    fn table(ddl: &str, dialect: Dialect) -> Table {
        let script = parser::parse_ddl(ddl, dialect).expect("ddl should parse");
        resolve_table(&script.tables[0], false)
    }

    #[test]
    fn signatures_carry_single_and_composite_primary_keys() {
        let users = table(
            "CREATE TABLE `users` (`id` BIGINT NOT NULL, PRIMARY KEY (`id`));",
            Dialect::MySql,
        );
        let contract = Contract::new(&users);
        assert_eq!(
            contract.signature(Method::GetByID),
            "GetByID(ctx context.Context, id int64) (*model.User, error)"
        );
        assert_eq!(contract.get_args(), "id");
        assert_eq!(contract.forward_args(), "ctx, id");
        assert_eq!(contract.model_args(), "m.Id");

        // Composite keys keep the DDL's declaration order.
        let items = table(
            "CREATE TABLE `order_items` (`order_id` BIGINT NOT NULL, \
             `item_id` BIGINT NOT NULL, PRIMARY KEY (`item_id`, `order_id`));",
            Dialect::MySql,
        );
        let contract = Contract::new(&items);
        assert_eq!(
            contract.signature(Method::Delete),
            "Delete(ctx context.Context, itemId int64, orderId int64) error"
        );
        assert_eq!(contract.get_args(), "itemId, orderId");
        assert_eq!(contract.model_args(), "m.ItemId, m.OrderId");
    }

    #[test]
    fn interface_lists_all_five_methods_in_order() {
        let users = table(
            "CREATE TABLE `users` (`id` BIGINT NOT NULL, PRIMARY KEY (`id`));",
            Dialect::MySql,
        );
        let contract = Contract::new(&users);
        let interface = contract.interface();
        let lines: Vec<&str> = interface.lines().collect();
        assert_eq!(lines.len(), 5, "{interface}");
        assert!(lines[0].starts_with("\tCreate(ctx context.Context, m *model.User) error"));
        assert!(lines[1].starts_with("\tGetByID(ctx context.Context, id int64)"));
        assert!(lines[2].starts_with("\tList(ctx context.Context, page int, pageSize int)"));
        assert!(lines[3].starts_with("\tUpdate(ctx context.Context, m *model.User) error"));
        assert!(lines[4].starts_with("\tDelete(ctx context.Context, id int64) error"));
    }

    #[test]
    fn parameter_names_avoid_reserved_words_and_keep_initialisms() {
        // `time` shadows the generated import, so the parameter becomes timePK.
        let logs = table(
            "CREATE TABLE `logs` (`time` DATETIME NOT NULL, PRIMARY KEY (`time`));",
            Dialect::MySql,
        );
        let contract = Contract::new(&logs);
        assert_eq!(contract.get_args(), "timePK");
        assert_eq!(
            contract.signature(Method::GetByID),
            "GetByID(ctx context.Context, timePK time.Time) (*model.Log, error)"
        );

        // A quoted all-caps PG identifier is an initialism: `id`, not `iD`.
        let users = table(
            "CREATE TABLE users (\"ID\" BIGINT, PRIMARY KEY (\"ID\"));",
            Dialect::PostgreSql,
        );
        assert_eq!(Contract::new(&users).get_args(), "id");
    }
}
