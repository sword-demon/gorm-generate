use gorm_generate_core::{generate, export_project, GenerateRequest, GeneratedFile};
use gorm_generate_core::generate::Dialect;

fn request(ddl: &str, dialect: Dialect) -> GenerateRequest {
    GenerateRequest { ddl: ddl.into(), dialect, module_path: "example.com/app".into(), cache_enabled: true, cache_ttl_seconds: 300, tinyint_as_bool: true }
}

#[test]
fn review_primary_key_case_and_types() {
    for dialect in [Dialect::MySql, Dialect::PostgreSql] {
        let p = generate(&request("CREATE TABLE t (id BIGINT NOT NULL, PRIMARY KEY (ID));", dialect)).unwrap();
        assert!(p.files[1].content.contains("id int64"), "PK must not disappear");
        assert!(p.files[0].content.contains("primaryKey"));
    }
    let p = generate(&request("CREATE TABLE flags (id TINYINT(1) NOT NULL PRIMARY KEY);", Dialect::MySql)).unwrap();
    for f in p.files.iter().skip(1) { assert!(f.content.contains("id bool"), "{}", f.path); }
    let p = generate(&request("CREATE TABLE events (id DATETIME PRIMARY KEY);", Dialect::MySql)).unwrap();
    for f in p.files.iter().skip(1) { assert!(f.content.contains("\"time\""), "{}", f.path); }
}

#[test]
fn review_rejects_unsafe_or_colliding_names() {
    for ddl in [
        "CREATE TABLE logs (table_name TEXT);",
        "CREATE TABLE t (user_id BIGINT, userId BIGINT);",
        "CREATE TABLE t (id BIGINT, id BIGINT);",
        "CREATE TABLE user (id BIGINT); CREATE TABLE users (id BIGINT);",
        "CREATE TABLE t (id BIGINT); CREATE TABLE t (id BIGINT);",
        "CREATE TABLE `we\"ird` (id BIGINT);",
        "CREATE TABLE t (`we\"ird` TEXT);",
    ] {
        assert!(generate(&request(ddl, Dialect::MySql)).is_err(), "must reject: {ddl}");
    }
}

#[test]
fn review_pg_aliases_and_qualified_comments() {
    let p = generate(&request("COMMENT ON COLUMN public.users.name IS 'display name'; CREATE TABLE public.users (id int8 PRIMARY KEY, name TEXT, n int4, small int2, f float4, d float8);", Dialect::PostgreSql)).unwrap();
    let model = &p.files[0].content;
    for text in ["Id int64", "N *int32", "Small *int16", "F *float32", "D *float64", "// display name"] { assert!(model.contains(text), "missing {text}: {model}"); }
}

#[test]
fn review_generated_columns_and_quoted_queries() {
    let p = generate(&request("CREATE TABLE t (`order` BIGINT PRIMARY KEY, total INT GENERATED ALWAYS AS (`order` + 1) STORED);", Dialect::MySql)).unwrap();
    assert!(p.files[0].content.contains("column:total;->"));
    assert!(!p.files[0].content.contains("column:total;autoIncrement"));
    assert!(p.files[1].content.contains("`order` = ?"));
    assert!(p.files[1].content.contains("page < 1"));
}

#[test]
fn review_validates_options() {
    let mut req = request("CREATE TABLE t (id BIGINT PRIMARY KEY);", Dialect::MySql);
    for module in ["", "my module", "bad\"path", "bad\npath", "/absolute", "a/../b"] {
        req.module_path = module.into();
        assert!(generate(&req).is_err(), "invalid module: {module:?}");
    }
    req.module_path = "example.com/app".into();
    req.cache_ttl_seconds = 0;
    assert!(generate(&req).is_err());
}

#[test]
fn review_export_rejects_duplicate_paths() {
    let dir = std::env::temp_dir().join(format!("gorm-review-duplicate-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let files = (0..2).map(|_| GeneratedFile { path: "a.go".into(), content: "new".into(), table: "t".into() }).collect::<Vec<_>>();
    let result = export_project(&dir, &files, true);
    let written = dir.join("a.go").exists();
    std::fs::remove_dir_all(dir).unwrap();
    assert!(result.is_err(), "duplicates must fail before writing");
    assert!(!written);
}

#[cfg(unix)]
#[test]
fn review_export_rejects_symlinks() {
    use std::os::unix::fs::symlink;
    let dir = std::env::temp_dir().join(format!("gorm-review-symlink-{}", std::process::id()));
    let outside = dir.with_extension("outside");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    symlink(&outside, dir.join("internal")).unwrap();
    let files = vec![GeneratedFile { path: "internal/victim.go".into(), content: "new".into(), table: "t".into() }];
    let result = export_project(&dir, &files, true);
    let escaped = outside.join("victim.go").exists();
    std::fs::remove_dir_all(dir).unwrap();
    std::fs::remove_dir_all(outside).unwrap();
    assert!(result.is_err(), "symlink must be rejected");
    assert!(!escaped);
}

#[test]
fn review_quoted_pg_keys_respect_case() {
    let mut req = request("CREATE TABLE t (\"ID\" BIGINT, PRIMARY KEY (id));", Dialect::PostgreSql);
    assert!(generate(&req).is_err());
    req.ddl = "CREATE TABLE t (\"ID\" BIGINT, PRIMARY KEY (\"ID\"));".into();
    let project = generate(&req).unwrap();
    assert!(project.files[0].content.contains("primaryKey"));
    assert!(project.files[1].content.contains("id int64"));
    req.dialect = Dialect::MySql;
    req.ddl = "CREATE TABLE MixedCase (Id BIGINT, PRIMARY KEY (ID));".into();
    let project = generate(&req).unwrap();
    assert!(project.files[0].content.contains("return \"MixedCase\""));
    assert!(project.files[1].content.contains("id int64"));
}

#[test]
fn review_parse_error_reports_actual_statement() {
    let req = request("CREATE TABLE t (id BIGINT); -- semicolon ; in comment\n CREATE TABLE broken (", Dialect::MySql);
    let error = generate(&req).unwrap_err();
    assert_eq!(error.statement_index, 2);
    assert!(error.snippet.contains("broken"));
}

#[test]
fn review_generated_shorthand_is_read_only() {
    let project = generate(&request("CREATE TABLE t (id INT PRIMARY KEY, total INT AS (id + 1) STORED);", Dialect::MySql)).unwrap();
    assert!(project.files[0].content.contains("column:total;->"));
}

#[test]
fn review_common_plural_names() {
    let project = generate(&request("CREATE TABLE categories (id INT); CREATE TABLE addresses (id INT); CREATE TABLE statuses (id INT);", Dialect::MySql)).unwrap();
    for name in ["Category", "Address", "Status"] {
        assert!(project.files.iter().any(|f| f.content.contains(&format!("type {name} struct"))));
    }
}

#[test]
fn review_export_staging_failure_preserves_originals() {
    let dir = std::env::temp_dir().join(format!("gorm-review-staging-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("original.go"), "original").unwrap();
    std::fs::write(dir.join("blocked"), "not a directory").unwrap();
    let files = vec![GeneratedFile { path: "original.go".into(), content: "new".into(), table: "t".into() }, GeneratedFile { path: "blocked/later.go".into(), content: "new".into(), table: "t".into() }];
    let result = export_project(&dir, &files, true);
    let original = std::fs::read_to_string(dir.join("original.go")).unwrap();
    let remaining = std::fs::read_dir(&dir).unwrap().count();
    std::fs::remove_dir_all(dir).unwrap();
    assert!(result.unwrap_err().to_string().contains("blocked"));
    assert_eq!(original, "original");
    assert_eq!(remaining, 2, "staged temporary files must be cleaned up");
}
