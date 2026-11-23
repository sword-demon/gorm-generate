//! DDL script parsing: extracts table definitions via sqlparser.

use sqlparser::ast::{
    ColumnDef, ColumnOption, CreateTable, DataType, Statement, TableConstraint,
};
use sqlparser::dialect::{MySqlDialect, PostgreSqlDialect};
use sqlparser::parser::Parser;
use sqlparser::tokenizer::Token;

use crate::generate::{Dialect, ParseError};

pub struct ParsedTable {
    pub name: String,
    pub comment: Option<String>,
    pub columns: Vec<ColumnDef>,
    pub primary_key: Vec<String>,
}

/// Result of parsing one DDL script.
pub struct ParsedScript {
    pub tables: Vec<ParsedTable>,
    /// Statements that are not CREATE TABLE; spec says ignore and count.
    pub ignored_count: usize,
}

/// Parses the DDL script, keeping only CREATE TABLE statements and PG
/// COMMENT ON (merged into the referenced table). Other statements are
/// ignored and counted (spec: 非建表语句被忽略并计数).
pub fn parse_ddl(ddl: &str, dialect: Dialect) -> Result<ParsedScript, ParseError> {
    let parser_dialect: &dyn sqlparser::dialect::Dialect = match dialect {
        Dialect::MySql => &MySqlDialect {},
        Dialect::PostgreSql => &PostgreSqlDialect {},
    };

    let statements = Parser::parse_sql(parser_dialect, ddl).map_err(|e| ParseError {
        statement_index: 1,
        snippet: e.to_string(),
    })?;

    let mut tables = Vec::new();
    let mut ignored_count = 0usize;
    for statement in &statements {
        match statement {
            Statement::CreateTable(CreateTable {
                name,
                table_options,
                columns,
                constraints,
                ..
            }) => {
                let table_name = ident_of(name);
                let primary_key = extract_primary_key(columns, constraints);
                // MySQL table COMMENT='...' lands in table_options (Plain or
                // With), not the Hive-only `comment` field.
                let table_comment = match table_options {
                    sqlparser::ast::CreateTableOptions::Plain(options)
                    | sqlparser::ast::CreateTableOptions::With(options) => options.iter().find_map(
                        |opt| match opt {
                            sqlparser::ast::SqlOption::Comment(def) => Some(comment_text(def)),
                            _ => None,
                        },
                    ),
                    _ => None,
                };
                tables.push(ParsedTable {
                    name: table_name,
                    comment: table_comment,
                    columns: columns.clone(),
                    primary_key,
                });
            }
            Statement::Comment {
                object_type,
                object_name,
                comment,
                ..
            } => {
                apply_comment(
                    &mut tables,
                    *object_type,
                    object_name,
                    comment.as_deref(),
                );
            }
            _ => {
                ignored_count += 1;
            }
        }
    }
    Ok(ParsedScript {
        tables,
        ignored_count,
    })
}

/// Merges PG `COMMENT ON TABLE t IS '...'` / `COMMENT ON COLUMN t.c IS '...'`
/// into the already-parsed table. Column names are matched
/// case-insensitively (PG folds unquoted identifiers to lowercase).
fn apply_comment(
    tables: &mut [ParsedTable],
    object_type: sqlparser::ast::CommentObject,
    object_name: &sqlparser::ast::ObjectName,
    comment: Option<&str>,
) {
    let Some(comment) = comment else { return };
    let Some(table_part) = object_name.0.first() else { return };
    let sqlparser::ast::ObjectNamePart::Identifier(table_ident) = table_part else { return };
    let table_name = table_ident.value.to_lowercase();

    let column_name = match object_name.0.get(1) {
        Some(sqlparser::ast::ObjectNamePart::Identifier(ident)) => {
            Some(ident.value.to_lowercase())
        }
        _ => None,
    };

    let table = match tables
        .iter_mut()
        .find(|t| t.name.to_lowercase() == table_name)
    {
        Some(t) => t,
        None => return,
    };

    match (object_type, column_name) {
        (sqlparser::ast::CommentObject::Table, _) => table.comment = Some(comment.to_string()),
        (sqlparser::ast::CommentObject::Column, Some(col)) => {
            if let Some(found) = table
                .columns
                .iter_mut()
                .find(|c| c.name.value.to_lowercase() == col)
            {
                // PG semantics are last-write-wins: replace an existing
                // comment instead of appending a second one.
                match found
                    .options
                    .iter_mut()
                    .find(|o| matches!(o.option, sqlparser::ast::ColumnOption::Comment(_)))
                {
                    Some(existing) => {
                        existing.option = sqlparser::ast::ColumnOption::Comment(comment.to_string())
                    }
                    None => found.options.push(sqlparser::ast::ColumnOptionDef {
                        name: None,
                        option: sqlparser::ast::ColumnOption::Comment(comment.to_string()),
                    }),
                }
            }
        }
        _ => {}
    }
}

fn comment_text(def: &sqlparser::ast::CommentDef) -> String {
    match def {
        sqlparser::ast::CommentDef::WithEq(s) | sqlparser::ast::CommentDef::WithoutEq(s) => {
            s.clone()
        }
    }
}

/// Extracts a column's `COMMENT '...'` option, if present.
pub fn column_comment(col: &ColumnDef) -> Option<String> {
    col.options.iter().find_map(|opt| match &opt.option {
        ColumnOption::Comment(s) => Some(s.clone()),
        _ => None,
    })
}

fn ident_of(name: &sqlparser::ast::ObjectName) -> String {
    match name.0.last() {
        Some(sqlparser::ast::ObjectNamePart::Identifier(ident)) => ident.value.clone(),
        _ => String::new(),
    }
}

fn extract_primary_key(columns: &[ColumnDef], constraints: &[TableConstraint]) -> Vec<String> {
    // Inline `PRIMARY KEY` on a column definition.
    for col in columns {
        let is_pk = col
            .options
            .iter()
            .any(|opt| matches!(opt.option, sqlparser::ast::ColumnOption::PrimaryKey(_)));
        if is_pk {
            return vec![col.name.value.clone()];
        }
    }
    // Table-level `PRIMARY KEY (a, b)`.
    for constraint in constraints {
        if let TableConstraint::PrimaryKey(pk) = constraint {
            return pk
                .columns
                .iter()
                .map(|c| match &c.column.expr {
                    sqlparser::ast::Expr::Identifier(ident) => ident.value.clone(),
                    _ => String::new(),
                })
                .collect();
        }
    }
    Vec::new()
}

/// MySQL `AUTO_INCREMENT` parses as a DialectSpecific token.
pub fn is_auto_increment(col: &ColumnDef) -> bool {
    col.options.iter().any(|opt| {
        matches!(
            &opt.option,
            sqlparser::ast::ColumnOption::DialectSpecific(tokens)
                if tokens.iter().any(|t| matches!(t, Token::Word(w) if w.value.eq_ignore_ascii_case("AUTO_INCREMENT")))
        )
    })
}

/// Classifies a MySQL column type: (base name, unsigned, display width).
pub fn classify_mysql_type(data_type: &DataType) -> (String, bool, Option<u64>) {
    match data_type {
        DataType::TinyInt(w) => ("tinyint".into(), false, *w),
        DataType::TinyIntUnsigned(w) => ("tinyint".into(), true, *w),
        DataType::SmallInt(w) => ("smallint".into(), false, *w),
        DataType::SmallIntUnsigned(w) => ("smallint".into(), true, *w),
        DataType::MediumInt(w) => ("mediumint".into(), false, *w),
        DataType::MediumIntUnsigned(w) => ("mediumint".into(), true, *w),
        DataType::Int(w) => ("int".into(), false, *w),
        DataType::IntUnsigned(w) => ("int".into(), true, *w),
        DataType::Integer(w) => ("int".into(), false, *w),
        DataType::IntegerUnsigned(w) => ("int".into(), true, *w),
        DataType::BigInt(w) => ("bigint".into(), false, *w),
        DataType::BigIntUnsigned(w) => ("bigint".into(), true, *w),
        DataType::Decimal(_) | DataType::Dec(_) => ("decimal".into(), false, None),
        DataType::DecimalUnsigned(_) | DataType::DecUnsigned(_) => ("decimal".into(), true, None),
        DataType::Numeric(_) | DataType::BigNumeric(_) => ("numeric".into(), false, None),
        DataType::Float(_) | DataType::FloatUnsigned(_) => ("float".into(), false, None),
        DataType::Double(_) | DataType::DoubleUnsigned(_) | DataType::DoublePrecision => {
            ("double".into(), false, None)
        }
        DataType::Boolean | DataType::Bool => ("bool".into(), false, None),
        DataType::Varchar(_) | DataType::CharacterVarying(_) | DataType::CharVarying(_) | DataType::Nvarchar(_) => {
            ("varchar".into(), false, None)
        }
        DataType::Char(_) | DataType::Character(_) => ("char".into(), false, None),
        DataType::Text | DataType::TinyText | DataType::MediumText | DataType::LongText => {
            ("text".into(), false, None)
        }
        DataType::Varbinary(_) | DataType::Binary(_) | DataType::Bytea => ("binary".into(), false, None),
        DataType::Blob(_) | DataType::TinyBlob | DataType::MediumBlob | DataType::LongBlob => {
            ("blob".into(), false, None)
        }
        DataType::JSON | DataType::JSONB => ("json".into(), false, None),
        DataType::Enum(_, _) => ("enum".into(), false, None),
        DataType::Set(_) => ("set".into(), false, None),
        DataType::Datetime(_) => ("datetime".into(), false, None),
        DataType::Timestamp(_, _) => ("timestamp".into(), false, None),
        DataType::Time(_, _) => ("time".into(), false, None),
        DataType::Date => ("date".into(), false, None),
        DataType::Custom(name, _) => {
            // PG SERIAL family parses as Custom types; normalize to the
            // underlying int with an autoIncrement signal carried separately.
            let base = name
                .0
                .last()
                .map(ident_value)
                .unwrap_or_default()
                .to_lowercase();
            match base.as_str() {
                "serial" => ("int".into(), false, None),
                "smallserial" => ("smallint".into(), false, None),
                "bigserial" => ("bigint".into(), false, None),
                "uuid" => ("uuid".into(), false, None),
                "bytea" => ("bytea".into(), false, None),
                other => (other.to_string(), false, None),
            }
        }
        other => (format!("{other:?}").to_lowercase(), false, None),
    }
}

/// PG `GENERATED ... AS IDENTITY` marks a column auto-increment.
pub fn is_identity(col: &ColumnDef) -> bool {
    col.options.iter().any(|opt| {
        matches!(
            &opt.option,
            sqlparser::ast::ColumnOption::Generated { generated_keyword: true, .. }
        )
    })
}

/// True when the column is a SERIAL-family Custom type (PG).
pub fn is_serial(data_type: &DataType) -> bool {
    if let DataType::Custom(name, _) = data_type {
        if let Some(part) = name.0.last() {
            let v = ident_value(part).to_lowercase();
            return matches!(v.as_str(), "serial" | "bigserial" | "smallserial");
        }
    }
    false
}

fn ident_value(part: &sqlparser::ast::ObjectNamePart) -> String {
    match part {
        sqlparser::ast::ObjectNamePart::Identifier(ident) => ident.value.clone(),
        _ => String::new(),
    }
}
