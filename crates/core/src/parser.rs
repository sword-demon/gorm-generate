//! DDL script parsing: extracts table definitions via sqlparser.

use sqlparser::ast::{ColumnDef, ColumnOption, CreateTable, DataType, Statement, TableConstraint};
use sqlparser::dialect::{MySqlDialect, PostgreSqlDialect};
use sqlparser::parser::Parser;
use sqlparser::tokenizer::{Token, Tokenizer};

use crate::generate::{Dialect, GenerateError};

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
    /// Facts about the script as a whole, not about any one table.
    pub warnings: Vec<String>,
}

/// Parses the DDL script, keeping only CREATE TABLE statements and PG
/// COMMENT ON (merged into the referenced table). Other statements are
/// ignored and counted (spec: 非建表语句被忽略并计数).
///
/// A script that only fails under the requested dialect is reported as a
/// dialect mismatch rather than as a broken script.
pub fn parse_ddl(ddl: &str, dialect: Dialect) -> Result<ParsedScript, GenerateError> {
    match parse_with_dialect(ddl, dialect) {
        Ok(script) => Ok(script),
        Err(error) => Err(diagnose_mismatch(ddl, dialect, error)),
    }
}

/// A script that fails under one dialect but parses under the other one is
/// written in the other dialect: say so, instead of leaving the user with a
/// raw parser error about syntax the dialect never accepted.
fn diagnose_mismatch(ddl: &str, dialect: Dialect, error: GenerateError) -> GenerateError {
    let GenerateError::Parse {
        statement_index,
        snippet,
    } = error
    else {
        return error;
    };
    let other = dialect.other();
    if parse_with_dialect(ddl, other).is_err() {
        return GenerateError::Parse {
            statement_index,
            snippet,
        };
    }
    GenerateError::DialectMismatch {
        statement_index,
        message: format!(
            "this DDL parses as {} DDL, but the dialect is set to {}; change the dialect or edit the DDL",
            other.label(),
            dialect.label()
        ),
        snippet,
    }
}

fn parse_with_dialect(ddl: &str, dialect: Dialect) -> Result<ParsedScript, GenerateError> {
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
    // `AUTO_INCREMENT` is a MySQL column option: sqlparser keeps it for the
    // MySQL dialect and swallows it for the others. The PG dialect would
    // drop the auto-increment flag without a word, so look for it in the
    // tokens, before the parser consumes them.
    let mysql_auto_increment = dialect == Dialect::PostgreSql
        && tokens.iter().any(|t| matches!(&t.token, Token::Word(w) if w.value.eq_ignore_ascii_case("AUTO_INCREMENT")));
    let mut parser = Parser::new(parser_dialect).with_tokens_with_locations(tokens);
    let mut statements = Vec::new();
    loop {
        while parser.consume_token(&Token::SemiColon) {}
        if parser.peek_token().token == Token::EOF {
            break;
        }
        let start = parser.peek_token().span.start;
        let statement = parser.parse_statement().map_err(|e| GenerateError::Parse {
            statement_index: statements.len() + 1,
            snippet: format!("{e}\n{}", statement_snippet(ddl, start.line, start.column)),
        })?;
        statements.push(statement);
        if parser.peek_token().token != Token::EOF && !parser.consume_token(&Token::SemiColon) {
            return Err(GenerateError::Parse {
                statement_index: statements.len(),
                snippet: format!(
                    "expected semicolon\n{}",
                    statement_snippet(ddl, start.line, start.column)
                ),
            });
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
                let table_name = name
                    .0
                    .last()
                    .and_then(|p| match p {
                        sqlparser::ast::ObjectNamePart::Identifier(i) => {
                            Some(if dialect == Dialect::MySql {
                                i.value.clone()
                            } else {
                                identifier_key(i, dialect)
                            })
                        }
                        _ => None,
                    })
                    .unwrap_or_default();
                let mut normalized_columns = columns.clone();
                for col in &mut normalized_columns {
                    if dialect == Dialect::PostgreSql {
                        col.name.value = identifier_key(&col.name, dialect);
                    }
                }
                let primary_key = extract_primary_key(&normalized_columns, constraints, dialect)?;
                // MySQL table COMMENT='...' lands in table_options (Plain or
                // With), not the Hive-only `comment` field.
                let table_comment = match table_options {
                    sqlparser::ast::CreateTableOptions::Plain(options)
                    | sqlparser::ast::CreateTableOptions::With(options) => {
                        options.iter().find_map(|opt| match opt {
                            sqlparser::ast::SqlOption::Comment(def) => Some(comment_text(def)),
                            _ => None,
                        })
                    }
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
        if let Statement::Comment {
            object_type,
            object_name,
            comment,
            ..
        } = statement
        {
            apply_comment(
                &mut tables,
                *object_type,
                object_name,
                comment.as_deref(),
                dialect,
            );
        }
    }
    let mut warnings = Vec::new();
    if mysql_auto_increment {
        warnings.push(
            "MySQL `AUTO_INCREMENT` is ignored by the PostgreSQL dialect; the column is not marked auto-increment — check the dialect"
                .to_string(),
        );
    }
    Ok(ParsedScript {
        tables,
        ignored_count,
        warnings,
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
    let Some(sqlparser::ast::ObjectNamePart::Identifier(table_ident)) =
        table_index.and_then(|i| parts.get(i))
    else {
        return;
    };
    let table_name = identifier_key(table_ident, dialect);
    let column_name = if column_object {
        match parts.last() {
            Some(sqlparser::ast::ObjectNamePart::Identifier(ident)) => {
                Some(identifier_key(ident, dialect))
            }
            _ => None,
        }
    } else {
        None
    };
    let Some(table) = tables.iter_mut().find(|t| t.name == table_name) else {
        return;
    };

    match (object_type, column_name) {
        (sqlparser::ast::CommentObject::Table, _) => table.comment = Some(comment.to_string()),
        (sqlparser::ast::CommentObject::Column, Some(col)) => {
            if let Some(found) = table.columns.iter_mut().find(|c| c.name.value == col) {
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

fn statement_snippet(ddl: &str, line: u64, column: u64) -> String {
    let mut snippet = String::new();
    for (index, source_line) in ddl
        .lines()
        .skip(line.saturating_sub(1) as usize)
        .enumerate()
    {
        if index > 0 {
            snippet.push('\n');
        }
        let start = if index == 0 {
            column.saturating_sub(1) as usize
        } else {
            0
        };
        let remaining = 500usize.saturating_sub(snippet.chars().count());
        snippet.extend(source_line.chars().skip(start).take(remaining));
        if snippet.chars().count() >= 500 {
            break;
        }
    }
    snippet
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

fn extract_primary_key(
    columns: &[ColumnDef],
    constraints: &[TableConstraint],
    dialect: Dialect,
) -> Result<Vec<String>, GenerateError> {
    let inline: Vec<_> = columns
        .iter()
        .filter(|col| {
            col.options
                .iter()
                .any(|opt| matches!(opt.option, ColumnOption::PrimaryKey(_)))
        })
        .map(|col| col.name.value.clone())
        .collect();
    if !inline.is_empty() {
        return Ok(inline);
    }
    for constraint in constraints {
        if let TableConstraint::PrimaryKey(pk) = constraint {
            return pk
                .columns
                .iter()
                .map(|c| {
                    let key = match &c.column.expr {
                        sqlparser::ast::Expr::Identifier(ident) => identifier_key(ident, dialect),
                        _ => {
                            return Err(GenerateError::Validation {
                                message: "unsupported primary key expression or prefix index"
                                    .into(),
                            })
                        }
                    };
                    columns
                        .iter()
                        .find(|c| identifier_key(&c.name, dialect) == key)
                        .map(|c| c.name.value.clone())
                        .ok_or_else(|| GenerateError::Validation {
                            message: format!("primary key column `{key}` does not exist"),
                        })
                })
                .collect();
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

/// PG `GENERATED ... AS IDENTITY` marks a column auto-increment.
pub fn is_identity(col: &ColumnDef) -> bool {
    col.options.iter().any(|opt| {
        matches!(
            &opt.option,
            sqlparser::ast::ColumnOption::Generated {
                generation_expr: None,
                ..
            }
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

pub(crate) fn ident_value(part: &sqlparser::ast::ObjectNamePart) -> String {
    match part {
        sqlparser::ast::ObjectNamePart::Identifier(ident) => ident.value.clone(),
        _ => String::new(),
    }
}

pub fn is_generated_expression(col: &ColumnDef) -> bool {
    col.options.iter().any(|opt| {
        matches!(
            &opt.option,
            ColumnOption::Generated {
                generation_expr: Some(_),
                ..
            }
        )
    })
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
) -> GenerateError {
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
    GenerateError::Parse {
        statement_index,
        snippet,
    }
}

fn source_line(ddl: &str, line: u64) -> Option<String> {
    if line == 0 {
        return None;
    }
    ddl.lines()
        .nth(line as usize - 1)
        .map(|text| text.chars().take(500).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_error(ddl: &str, dialect: Dialect) -> GenerateError {
        match parse_ddl(ddl, dialect) {
            Ok(_) => panic!("expected a parse error"),
            Err(e) => e,
        }
    }

    /// Narrowing helpers: these tests assert the parse variant's fields.
    fn statement_index(error: &GenerateError) -> usize {
        match error {
            GenerateError::Parse {
                statement_index, ..
            } => *statement_index,
            other => panic!("expected a parse error, got {other:?}"),
        }
    }

    fn snippet(error: &GenerateError) -> &str {
        match error {
            GenerateError::Parse { snippet, .. } => snippet,
            other => panic!("expected a parse error, got {other:?}"),
        }
    }

    const FIRST: &str = "CREATE TABLE a (id BIGINT PRIMARY KEY);\n";

    #[test]
    fn unterminated_string_reports_the_statement_it_occurs_in() {
        let e = parse_error(
            &format!("{FIRST}CREATE TABLE b (name VARCHAR(10) DEFAULT 'oops);\n"),
            Dialect::MySql,
        );
        assert_eq!(statement_index(&e), 2, "snippet: {}", snippet(&e));
    }

    #[test]
    fn unterminated_string_in_the_first_statement_reports_one() {
        let e = parse_error(
            "CREATE TABLE a (name VARCHAR(10) DEFAULT 'oops);\n",
            Dialect::MySql,
        );
        assert_eq!(statement_index(&e), 1, "snippet: {}", snippet(&e));
    }

    #[test]
    fn unterminated_comment_inside_a_statement_reports_that_statement() {
        let e = parse_error(
            &format!("{FIRST}CREATE TABLE b (id BIGINT /* oops\n"),
            Dialect::MySql,
        );
        assert_eq!(statement_index(&e), 2, "snippet: {}", snippet(&e));
    }

    #[test]
    fn tokenizer_snippet_shows_the_error_location_and_line() {
        let e = parse_error(
            &format!("{FIRST}CREATE TABLE b (name VARCHAR(10) DEFAULT 'oops);\n"),
            Dialect::MySql,
        );
        assert!(snippet(&e).contains("Line: 2"), "snippet: {}", snippet(&e));
        assert!(
            snippet(&e).contains("CREATE TABLE b"),
            "snippet: {}",
            snippet(&e)
        );
    }
}
