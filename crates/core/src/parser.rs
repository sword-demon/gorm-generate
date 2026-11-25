//! DDL script parsing: extracts table definitions via sqlparser.

use sqlparser::ast::{
    ColumnDef, ColumnOption, CreateTable, DataType, Statement, TableConstraint,
};
use sqlparser::dialect::{MySqlDialect, PostgreSqlDialect};
use sqlparser::parser::Parser;
use sqlparser::tokenizer::{Token, Tokenizer};

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

    // Tokenize up front instead of `Parser::try_with_sql`: the latter collapses
    // tokenizer failures into a string and loses both the location and the
    // tokens recognized before the failure.
    let mut tokens = Vec::new();
    Tokenizer::new(parser_dialect, ddl)
        .tokenize_with_location_into_buf(&mut tokens)
        .map_err(|e| tokenizer_error(ddl, &tokens, &e))?;
    let mut parser = Parser::new(parser_dialect).with_tokens_with_locations(tokens);
    let mut statements = Vec::new();
    loop {
        while parser.consume_token(&Token::SemiColon) {}
        if parser.peek_token().token == Token::EOF { break; }
        let start = parser.peek_token().span.start;
        let remaining: String = ddl.lines().skip(start.line.saturating_sub(1) as usize)
            .enumerate().map(|(i, line)| if i == 0 { line.chars().skip(start.column.saturating_sub(1) as usize).collect::<String>() } else { format!("\n{line}") }).collect();
        let statement = parser.parse_statement().map_err(|e| ParseError {
            statement_index: statements.len() + 1,
            snippet: format!("{e}\n{}", remaining.chars().take(500).collect::<String>()),
        })?;
        statements.push(statement);
        if parser.peek_token().token != Token::EOF && !parser.consume_token(&Token::SemiColon) {
            return Err(ParseError { statement_index: statements.len(), snippet: format!("expected semicolon\n{}", remaining.chars().take(500).collect::<String>()) });
        }
    }

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
                let table_name = name.0.last().and_then(|p| match p {
                    sqlparser::ast::ObjectNamePart::Identifier(i) => Some(if dialect == Dialect::MySql { i.value.clone() } else { identifier_key(i, dialect) }),
                    _ => None,
                }).unwrap_or_default();
                let mut normalized_columns = columns.clone();
                for col in &mut normalized_columns {
                    if dialect == Dialect::PostgreSql { col.name.value = identifier_key(&col.name, dialect); }
                }
                let primary_key = extract_primary_key(&normalized_columns, constraints, dialect)?;
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
                    columns: normalized_columns,
                    primary_key,
                });
            }
            Statement::Comment { .. } => {}
            _ => {
                ignored_count += 1;
            }
        }
    }
    // Comments may precede CREATE TABLE in an input script.
    for statement in &statements {
        if let Statement::Comment { object_type, object_name, comment, .. } = statement {
            apply_comment(&mut tables, *object_type, object_name, comment.as_deref(), dialect);
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
    dialect: Dialect,
) {
    let Some(comment) = comment else { return };
    let parts = &object_name.0;
    let column_object = object_type == sqlparser::ast::CommentObject::Column;
    let table_index = parts.len().checked_sub(if column_object { 2 } else { 1 });
    let Some(sqlparser::ast::ObjectNamePart::Identifier(table_ident)) = table_index.and_then(|i| parts.get(i)) else { return };
    let table_name = identifier_key(table_ident, dialect);
    let column_name = if column_object {
        match parts.last() {
            Some(sqlparser::ast::ObjectNamePart::Identifier(ident)) => Some(identifier_key(ident, dialect)),
            _ => None,
        }
    } else { None };
    let Some(table) = tables.iter_mut().find(|t| t.name == table_name) else { return };

    match (object_type, column_name) {
        (sqlparser::ast::CommentObject::Table, _) => table.comment = Some(comment.to_string()),
        (sqlparser::ast::CommentObject::Column, Some(col)) => {
            if let Some(found) = table
                .columns
                .iter_mut()
                .find(|c| c.name.value == col)
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

pub fn identifier_key(ident: &sqlparser::ast::Ident, dialect: Dialect) -> String {
    if dialect == Dialect::MySql || ident.quote_style.is_none() {
        ident.value.to_lowercase()
    } else {
        ident.value.clone()
    }
}

fn extract_primary_key(columns: &[ColumnDef], constraints: &[TableConstraint], dialect: Dialect) -> Result<Vec<String>, ParseError> {
    let inline: Vec<_> = columns.iter().filter(|col| col.options.iter().any(|opt| matches!(opt.option, ColumnOption::PrimaryKey(_)))).map(|col| col.name.value.clone()).collect();
    if !inline.is_empty() { return Ok(inline); }
    for constraint in constraints {
        if let TableConstraint::PrimaryKey(pk) = constraint {
            return pk.columns.iter().map(|c| {
                let key = match &c.column.expr {
                    sqlparser::ast::Expr::Identifier(ident) => identifier_key(ident, dialect),
                    _ => return Err(ParseError { statement_index: 0, snippet: "unsupported primary key expression or prefix index".into() }),
                };
                columns.iter().find(|c| identifier_key(&c.name, dialect) == key).map(|c| c.name.value.clone()).ok_or_else(|| ParseError {
                    statement_index: 0, snippet: format!("primary key column `{key}` does not exist"),
                })
            }).collect();
        }
    }
    Ok(Vec::new())
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
        DataType::Zerofill(inner) => { let (base, _, width) = classify_mysql_type(inner); (base, true, width) }
        DataType::Bit(_) | DataType::VarBit(_) => ("binary".into(), false, None),
        DataType::TinyInt(w) => ("tinyint".into(), false, *w),
        DataType::TinyIntUnsigned(w) => ("tinyint".into(), true, *w),
        DataType::SmallInt(w) | DataType::Int2(w) => ("smallint".into(), false, *w),
        DataType::SmallIntUnsigned(w) => ("smallint".into(), true, *w),
        DataType::MediumInt(w) => ("mediumint".into(), false, *w),
        DataType::MediumIntUnsigned(w) => ("mediumint".into(), true, *w),
        DataType::Int(w) | DataType::Int4(w) => ("int".into(), false, *w),
        DataType::IntUnsigned(w) => ("int".into(), true, *w),
        DataType::Integer(w) => ("int".into(), false, *w),
        DataType::IntegerUnsigned(w) => ("int".into(), true, *w),
        DataType::BigInt(w) | DataType::Int8(w) => ("bigint".into(), false, *w),
        DataType::BigIntUnsigned(w) => ("bigint".into(), true, *w),
        DataType::Decimal(_) | DataType::Dec(_) => ("decimal".into(), false, None),
        DataType::DecimalUnsigned(_) | DataType::DecUnsigned(_) => ("decimal".into(), true, None),
        DataType::Numeric(_) | DataType::BigNumeric(_) => ("numeric".into(), false, None),
        DataType::Float(_) | DataType::FloatUnsigned(_) | DataType::Float4 => ("float".into(), false, None),
        // MySQL `REAL` is a DOUBLE synonym unless REAL_AS_FLOAT is set; PG REAL
        // is 4-byte but widening to float64 is lossless, so map to double.
        DataType::Real | DataType::Float8 | DataType::Double(_) | DataType::DoubleUnsigned(_) | DataType::DoublePrecision => {
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
            sqlparser::ast::ColumnOption::Generated { generation_expr: None, .. }
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

pub fn is_generated_expression(col: &ColumnDef) -> bool {
    col.options.iter().any(|opt| matches!(&opt.option, ColumnOption::Generated { generation_expr: Some(_), .. }))
}

/// Maps a tokenizer failure to the statement it occurred in.
///
/// The tokenizer keeps the tokens it recognized before failing, so the
/// statement being tokenized is one past the last semicolon in that buffer.
/// `TokenizerError` also carries the line/column, which is appended to the
/// snippet because it points at the exact character.
fn tokenizer_error(
    ddl: &str,
    tokens: &[sqlparser::tokenizer::TokenWithSpan],
    err: &sqlparser::tokenizer::TokenizerError,
) -> ParseError {
    let statement_index = tokens
        .iter()
        .filter(|t| matches!(t.token, Token::SemiColon))
        .count()
        + 1;
    let mut snippet = err.to_string();
    if let Some(line) = source_line(ddl, err.location.line).filter(|line| !line.trim().is_empty()) {
        snippet.push('\n');
        snippet.push_str(&line);
    }
    ParseError { statement_index, snippet }
}

fn source_line(ddl: &str, line: u64) -> Option<String> {
    if line == 0 {
        return None;
    }
    ddl.lines().nth(line as usize - 1).map(|text| text.chars().take(500).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_error(ddl: &str, dialect: Dialect) -> ParseError {
        match parse_ddl(ddl, dialect) {
            Ok(_) => panic!("expected a parse error"),
            Err(e) => e,
        }
    }

    const FIRST: &str = "CREATE TABLE a (id BIGINT PRIMARY KEY);\n";

    #[test]
    fn unterminated_string_reports_the_statement_it_occurs_in() {
        let e = parse_error(
            &format!("{FIRST}CREATE TABLE b (name VARCHAR(10) DEFAULT 'oops);\n"),
            Dialect::MySql,
        );
        assert_eq!(e.statement_index, 2, "snippet: {}", e.snippet);
    }

    #[test]
    fn unterminated_string_in_the_first_statement_reports_one() {
        let e = parse_error(
            "CREATE TABLE a (name VARCHAR(10) DEFAULT 'oops);\n",
            Dialect::MySql,
        );
        assert_eq!(e.statement_index, 1, "snippet: {}", e.snippet);
    }

    #[test]
    fn unterminated_comment_inside_a_statement_reports_that_statement() {
        let e = parse_error(
            &format!("{FIRST}CREATE TABLE b (id BIGINT /* oops\n"),
            Dialect::MySql,
        );
        assert_eq!(e.statement_index, 2, "snippet: {}", e.snippet);
    }

    #[test]
    fn tokenizer_snippet_shows_the_error_location_and_line() {
        let e = parse_error(
            &format!("{FIRST}CREATE TABLE b (name VARCHAR(10) DEFAULT 'oops);\n"),
            Dialect::MySql,
        );
        assert!(e.snippet.contains("Line: 2"), "snippet: {}", e.snippet);
        assert!(e.snippet.contains("CREATE TABLE b"), "snippet: {}", e.snippet);
    }
}
