//! DDL script parsing: extracts table definitions via sqlparser.

use sqlparser::ast::{ColumnDef, CreateTable, DataType, Statement, TableConstraint};
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;
use sqlparser::tokenizer::Token;

use crate::generate::{Dialect, ParseError};

pub struct ParsedTable {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub primary_key: Vec<String>,
}

/// Parses the DDL script, keeping only CREATE TABLE statements. Other
/// statements are ignored per spec.
pub fn parse_ddl(ddl: &str, dialect: Dialect) -> Result<Vec<ParsedTable>, ParseError> {
    let parser_dialect: &dyn sqlparser::dialect::Dialect = match dialect {
        Dialect::MySql => &MySqlDialect {},
    };

    let statements = Parser::parse_sql(parser_dialect, ddl).map_err(|e| ParseError {
        statement_index: 1,
        snippet: e.to_string(),
    })?;

    let mut tables = Vec::new();
    for statement in &statements {
        if let Statement::CreateTable(CreateTable {
            name, columns, constraints, ..
        }) = statement
        {
            let table_name = ident_of(name);
            let primary_key = extract_primary_key(columns, constraints);
            tables.push(ParsedTable {
                name: table_name,
                columns: columns.clone(),
                primary_key,
            });
        }
    }
    Ok(tables)
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
        DataType::Varbinary(_) | DataType::Binary(_) => ("binary".into(), false, None),
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
        DataType::Custom(name, _) => (
            name.0
                .last()
                .map(ident_value)
                .unwrap_or_default()
                .to_lowercase(),
            false,
            None,
        ),
        other => (format!("{other:?}").to_lowercase(), false, None),
    }
}

fn ident_value(part: &sqlparser::ast::ObjectNamePart) -> String {
    match part {
        sqlparser::ast::ObjectNamePart::Identifier(ident) => ident.value.clone(),
        _ => String::new(),
    }
}
