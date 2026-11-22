//! Tests for the `generate` seam: one vertical slice per test, plus a golden
//! file locking the sample DDL's complete output.

use super::*;

fn request(ddl: &str) -> GenerateRequest {
    GenerateRequest {
        ddl: ddl.to_string(),
        dialect: Dialect::MySql,
        module_path: "github.com/example/app".to_string(),
        cache_enabled: false,
        cache_ttl_seconds: 0,
        tinyint_as_bool: false,
    }
}

#[test]
fn generates_model_file_for_single_table() {
    let ddl = "CREATE TABLE `users` (`id` BIGINT NOT NULL AUTO_INCREMENT, PRIMARY KEY (`id`));";
    let result = generate(&request(ddl)).expect("should generate");

    let file = result
        .files
        .iter()
        .find(|f| f.path == "internal/model/user.go")
        .expect("model file for users table");
    assert!(file.content.contains("type User struct {"));
    assert!(file.content.contains("func (u User) TableName() string {"));
}

#[test]
fn parse_error_reports_statement_and_snippet() {
    let ddl = "CREATE TABLE `users` (";
    let err = generate(&request(ddl)).expect_err("invalid DDL should error");
    assert!(err.statement_index >= 1);
    assert!(!err.snippet.is_empty());
}

#[test]
fn maps_column_types_to_go_types() {
    let ddl = "CREATE TABLE `t` (\
        `i` INT NOT NULL, \
        `d` DECIMAL(10,2) NOT NULL, \
        `s` VARCHAR(100) NOT NULL, \
        `c` DATETIME NOT NULL, \
        `j` JSON NOT NULL, \
        `b` BLOB NOT NULL, \
        `n` VARCHAR(10) NULL, \
        `u` BIGINT UNSIGNED NOT NULL, \
        `e` ENUM('a','b') NOT NULL\
    );";
    let result = generate(&request(ddl)).expect("should generate");
    let content = &result.files[0].content;

    assert!(content.contains("I int32"), "int -> int32, got: {content}");
    assert!(content.contains("D string"), "decimal -> string");
    assert!(content.contains("S string"), "varchar -> string");
    assert!(content.contains("C time.Time"), "datetime -> time.Time");
    assert!(content.contains("J json.RawMessage"), "json -> json.RawMessage");
    assert!(content.contains("B []byte"), "blob -> []byte");
    assert!(content.contains("N *string"), "nullable -> pointer");
    assert!(content.contains("U uint64"), "unsigned bigint -> uint64");
    assert!(content.contains("E string"), "enum -> string");
}

#[test]
fn unknown_type_maps_to_any_with_warning() {
    let ddl = "CREATE TABLE `t` (`x` WEIRDTYPE NOT NULL);";
    let result = generate(&request(ddl)).expect("should generate");
    assert!(result.files[0].content.contains("X any"));
    assert_eq!(result.warnings.len(), 1, "one warning for unknown type");
}

#[test]
fn tinyint_maps_to_int8_by_default_and_bool_with_flag() {
    let ddl = "CREATE TABLE `t` (`f` TINYINT(1) NOT NULL);";

    let default = generate(&request(ddl)).expect("should generate");
    assert!(default.files[0].content.contains("F int8"), "tinyint(1) -> int8 by default");

    let mut req = request(ddl);
    req.tinyint_as_bool = true;
    let as_bool = generate(&req).expect("should generate");
    assert!(as_bool.files[0].content.contains("F bool"), "tinyint(1) -> bool with flag");
}

#[test]
fn generates_gorm_and_json_tags() {
    let ddl = "CREATE TABLE `users` (\
        `id` BIGINT NOT NULL AUTO_INCREMENT, \
        `name` VARCHAR(50) NOT NULL, \
        `deleted_at` DATETIME NULL, \
        PRIMARY KEY (`id`)\
    );";
    let result = generate(&request(ddl)).expect("should generate");
    let content = &result.files[0].content;

    assert!(content.contains("gorm:\"column:id;primaryKey;autoIncrement;not null\""));
    assert!(content.contains("gorm:\"column:name;not null\""));
    assert!(content.contains("json:\"name\""), "json tag uses column name verbatim");
    assert!(content.contains("DeletedAt gorm.DeletedAt"), "deleted_at -> gorm.DeletedAt for soft delete");
}

#[test]
fn multiple_tables_generate_separate_files() {
    let ddl = "CREATE TABLE `users` (`id` BIGINT NOT NULL, PRIMARY KEY (`id`));\
        CREATE TABLE `orders` (`id` BIGINT NOT NULL, PRIMARY KEY (`id`));";
    let result = generate(&request(ddl)).expect("should generate");
    assert_eq!(result.tables, vec!["users", "orders"]);
    let paths: Vec<_> = result.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, vec!["internal/model/user.go", "internal/model/order.go"]);
}

#[test]
fn golden_sample_ddl_is_locked() {
    let ddl = include_str!("../fixtures/sample_mysql_ddl.sql");
    let result = generate(&request(ddl)).expect("golden DDL should parse");

    let actual: Vec<(&str, &str)> = result
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.content.as_str()))
        .collect();
    let golden: Vec<(&str, &str)> = GOLDEN
        .iter()
        .map(|(p, c)| (*p, *c))
        .collect();
    assert_eq!(actual, golden, "golden output changed; update fixtures/golden_model.go intentionally");
}

#[test]
fn table_and_column_comments_become_go_comments() {
    let ddl = "CREATE TABLE `users` (\
        `id` BIGINT NOT NULL COMMENT 'primary key', \
        `n` VARCHAR(10) NULL COMMENT 'the name', \
        PRIMARY KEY (`id`)\
    ) COMMENT='user table';";
    let result = generate(&request(ddl)).expect("should generate");
    let content = &result.files[0].content;

    assert!(content.contains("// user table"), "table comment above struct, got: {content}");
    assert!(content.contains("\t// primary key\n\tId int64"), "column comment above field");
    assert!(content.contains("\t// the name\n\tN *string"), "nullable column keeps comment");
}

#[test]
fn ignored_statements_are_counted_into_warnings() {
    let ddl = "CREATE TABLE `users` (`id` BIGINT NOT NULL, PRIMARY KEY (`id`));\
        INSERT INTO `users` VALUES (1);\
        CREATE INDEX idx_name ON `users` (`n`);";
    let result = generate(&request(ddl)).expect("should generate");
    assert_eq!(result.tables, vec!["users"]);
    let ignored = result.warnings.iter().find(|w| w.contains("ignored"));
    assert!(ignored.is_some(), "warning should count ignored statements, got: {:?}", result.warnings);
    assert!(ignored.unwrap().contains("2"), "two ignored statements counted");
}

#[test]
fn multiline_comment_is_flattened_into_one_line() {
    let ddl = "CREATE TABLE `t` (`x` INT NOT NULL COMMENT 'line1\nline2');";
    let result = generate(&request(ddl)).expect("should generate");
    let content = &result.files[0].content;
    assert!(
        content.contains("\t// line1 line2\n\tX "),
        "comment flattened to one line, got: {content}"
    );
}

#[test]
fn generation_is_idempotent() {
    let ddl = include_str!("../fixtures/sample_mysql_ddl.sql");
    let first = generate(&request(ddl)).expect("first run");
    let second = generate(&request(ddl)).expect("second run");
    assert_eq!(
        first.files.iter().map(|f| &f.content).collect::<Vec<_>>(),
        second.files.iter().map(|f| &f.content).collect::<Vec<_>>(),
        "same input must produce byte-identical output"
    );
}

/// Expected golden output, paired by path. Kept inline so a mismatch prints
/// the exact file content diff.
const GOLDEN: &[(&str, &str)] = &[("internal/model/user.go", include_str!("../fixtures/golden_user_model.go"))];
