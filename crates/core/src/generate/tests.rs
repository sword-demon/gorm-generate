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

fn file<'a>(project: &'a GeneratedProject, table: &str, kind: FileKind) -> &'a GeneratedFile {
    project
        .file(table, kind)
        .unwrap_or_else(|| panic!("missing {kind:?} file for table `{table}`"))
}

#[test]
fn generates_model_file_for_single_table() {
    let ddl = "CREATE TABLE `users` (`id` BIGINT NOT NULL AUTO_INCREMENT, PRIMARY KEY (`id`));";
    let result = generate(&request(ddl)).expect("should generate");

    let file = file(&result, "users", FileKind::Model);
    assert!(file.content.contains("type User struct {"));
    assert!(file.content.contains("func (u User) TableName() string {"));
}

#[test]
fn parse_error_reports_statement_and_snippet() {
    let ddl = "CREATE TABLE `users` (";
    let err = generate(&request(ddl)).expect_err("invalid DDL should error");
    match err {
        GenerateError::Parse { statement_index, snippet } => {
            assert!(statement_index >= 1);
            assert!(!snippet.is_empty());
        }
        other => panic!("expected a parse error, got {other:?}"),
    }
}

#[test]
fn dependencies_follow_what_was_emitted() {
    // A table without a primary key only produces a model. Without
    // `deleted_at` that model imports nothing, so there is nothing to get.
    let model_only = generate(&request("CREATE TABLE logs (msg TEXT NOT NULL);")).unwrap();
    assert!(model_only.dependencies.is_empty(), "{:?}", model_only.dependencies);

    // `deleted_at` pulls gorm into the model even without a primary key.
    let soft_delete = generate(&request(
        "CREATE TABLE logs (msg TEXT NOT NULL, deleted_at DATETIME NULL);",
    ))
    .unwrap();
    assert_eq!(soft_delete.dependencies, vec!["gorm.io/gorm"]);

    // A primary key yields repository files, which always import gorm.
    let mut req = request("CREATE TABLE users (id BIGINT PRIMARY KEY);");
    req.cache_ttl_seconds = 300;
    let plain = generate(&req).unwrap();
    assert_eq!(plain.dependencies, vec!["gorm.io/gorm"]);

    req.cache_enabled = true;
    let cached = generate(&req).unwrap();
    assert_eq!(
        cached.dependencies,
        vec!["gorm.io/gorm", "github.com/redis/go-redis/v9"]
    );

    // Cache enabled but no table can use it: no cache file, no redis hint.
    req.ddl = "CREATE TABLE logs (msg TEXT NOT NULL);".into();
    let cached_model_only = generate(&req).unwrap();
    assert!(cached_model_only.dependencies.is_empty(), "{:?}", cached_model_only.dependencies);
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
    let content = &file(&result, "t", FileKind::Model).content;

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
    let ddl = "CREATE TABLE `t` (`id` BIGINT NOT NULL, `x` WEIRDTYPE NOT NULL, PRIMARY KEY (`id`));";
    let result = generate(&request(ddl)).expect("should generate");
    assert!(file(&result, "t", FileKind::Model).content.contains("X any"));
    assert!(
        result.warnings.iter().any(|w| w.contains("weirdtype")),
        "warning for unknown type, got: {:?}",
        result.warnings
    );
}

#[test]
fn mysql_real_maps_to_float64_and_float_stays_float32() {
    let ddl = "CREATE TABLE `t` (`id` BIGINT NOT NULL, `r` REAL NOT NULL, `f` FLOAT NOT NULL, PRIMARY KEY (`id`));";
    let result = generate(&request(ddl)).expect("should generate");
    let content = &file(&result, "t", FileKind::Model).content;

    assert!(content.contains("R float64"), "MySQL REAL is a DOUBLE synonym, got: {content}");
    assert!(content.contains("F float32"), "FLOAT stays float32");
}

#[test]
fn tables_colliding_on_generated_impl_type_are_rejected() {
    // Distinct file keys (`api_user` / `apiuser`) but one impl type name.
    let ddl = "CREATE TABLE `api_users` (`id` BIGINT NOT NULL, PRIMARY KEY (`id`)); \
               CREATE TABLE `apiuser` (`id` BIGINT NOT NULL, PRIMARY KEY (`id`));";
    let err = generate(&request(ddl)).expect_err("colliding impl types should error");

    let message = validation_message(err);
    assert!(message.contains("conflicts"), "got: {message}");
}

#[test]
fn digit_leading_generated_identifier_is_rejected() {
    let ddl = "CREATE TABLE `t` (`id` BIGINT NOT NULL, `_1` INT NOT NULL, PRIMARY KEY (`id`));";
    let err = generate(&request(ddl)).expect_err("a field sanitizing to `1` should error");

    let message = validation_message(err);
    assert!(message.contains("unsupported"), "got: {message}");
}

fn validation_message(error: GenerateError) -> String {
    match error {
        GenerateError::Validation { message } => message,
        other => panic!("expected a validation error, got {other:?}"),
    }
}

#[test]
fn tinyint_maps_to_int8_by_default_and_bool_with_flag() {
    let ddl = "CREATE TABLE `t` (`f` TINYINT(1) NOT NULL);";

    let default = generate(&request(ddl)).expect("should generate");
    assert!(file(&default, "t", FileKind::Model).content.contains("F int8"), "tinyint(1) -> int8 by default");

    let mut req = request(ddl);
    req.tinyint_as_bool = true;
    let as_bool = generate(&req).expect("should generate");
    assert!(file(&as_bool, "t", FileKind::Model).content.contains("F bool"), "tinyint(1) -> bool with flag");
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
    let content = &file(&result, "users", FileKind::Model).content;

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
    // Model + repo + service per table.
    assert_eq!(result.files.len(), 6);
    let paths: Vec<_> = result.files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"internal/model/user.go"));
    assert!(paths.contains(&"internal/model/order.go"));
    assert!(paths.contains(&"internal/repository/order_repo.go"));
    assert!(paths.contains(&"internal/service/order_service.go"));
}

#[test]
fn every_file_carries_its_source_table() {
    let ddl = "CREATE TABLE `users` (`id` BIGINT NOT NULL, PRIMARY KEY (`id`));\
        CREATE TABLE `orders` (`id` BIGINT NOT NULL, PRIMARY KEY (`id`));";
    let result = generate(&request(ddl)).expect("should generate");
    for file in &result.files {
        assert!(
            file.table == "users" || file.table == "orders",
            "file {} has table {:?}",
            file.path,
            file.table
        );
        // Path prefix and table agree: user.go -> users, order_repo.go -> orders.
        let expected = if file.path.contains("/user") { "users" } else { "orders" };
        assert_eq!(file.table, expected, "path {} table mismatch", file.path);
    }
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
    let content = &file(&result, "users", FileKind::Model).content;

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
fn single_pk_table_generates_repository_and_service() {
    let ddl = "CREATE TABLE `users` (`id` BIGINT NOT NULL AUTO_INCREMENT, \
        `n` VARCHAR(10) NOT NULL, PRIMARY KEY (`id`));";
    let result = generate(&request(ddl)).expect("should generate");

    let paths: Vec<_> = result.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![
            "internal/model/user.go",
            "internal/repository/user_repo.go",
            "internal/service/user_service.go",
        ]
    );

    let repo = &result.files.iter().find(|f| f.path.ends_with("user_repo.go")).unwrap().content;
    assert!(repo.contains("type UserRepository interface {"), "repo interface");
    assert!(repo.contains("Create(ctx context.Context, m *model.User) error"));
    assert!(repo.contains("GetByID(ctx context.Context, id int64) (*model.User, error)"));
    assert!(repo.contains("List(ctx context.Context, page int, pageSize int) ([]*model.User, int64, error)"));
    assert!(repo.contains("Update(ctx context.Context, m *model.User) error"));
    assert!(repo.contains("Delete(ctx context.Context, id int64) error"));
    assert!(repo.contains("type userRepository struct"), "repo impl");
    assert!(repo.contains("func NewUserRepository(db *gorm.DB) UserRepository"), "repo constructor");
    assert!(repo.contains("Where(\"`id` = ?\", id).First(&m)"), "Where binds PK args");
    assert!(repo.contains("Where(\"`id` = ?\", id).Delete(&model.User{})"), "Delete binds PK args");

    let svc = &result.files.iter().find(|f| f.path.ends_with("user_service.go")).unwrap().content;
    assert!(svc.contains("type UserService interface {"), "service interface");
    assert!(svc.contains("func NewUserService(repo repository.UserRepository) UserService"), "service constructor injects repo interface");
    assert!(svc.contains("return s.repo.GetByID(ctx, id)"), "thin passthrough, got: {svc}");
    assert!(svc.contains("import (\n\t\"context\"\n\n\t\"github.com/example/app/internal/model\"\n\t\"github.com/example/app/internal/repository\"\n)"), "service imports model too");
}

#[test]
fn composite_pk_generates_multi_param_methods() {
    let ddl = "CREATE TABLE `order_items` (`order_id` BIGINT NOT NULL, \
        `item_id` BIGINT NOT NULL, PRIMARY KEY (`order_id`, `item_id`));";
    let result = generate(&request(ddl)).expect("should generate");
    let repo = &result.files.iter().find(|f| f.path.ends_with("order_item_repo.go")).unwrap().content;
    assert!(
        repo.contains("GetByID(ctx context.Context, orderId int64, itemId int64) (*model.OrderItem, error)"),
        "multi-param GetByID, got: {repo}"
    );
    assert!(
        repo.contains("Delete(ctx context.Context, orderId int64, itemId int64) error"),
        "multi-param Delete"
    );
}

#[test]
fn no_pk_table_generates_model_only_with_warning() {
    let ddl = "CREATE TABLE `logs` (`msg` VARCHAR(200) NOT NULL);";
    let result = generate(&request(ddl)).expect("should generate");
    let paths: Vec<_> = result.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, vec!["internal/model/log.go"], "model only");
    assert!(
        result.warnings.iter().any(|w| w.contains("logs") && w.contains("primary key")),
        "warning about missing primary key, got: {:?}",
        result.warnings
    );
}

#[test]
fn module_path_is_used_in_imports() {
    let ddl = "CREATE TABLE `users` (`id` BIGINT NOT NULL, PRIMARY KEY (`id`));";
    let result = generate(&request(ddl)).expect("should generate");
    let repo = &file(&result, "users", FileKind::Repository).content;
    assert!(repo.contains("\"github.com/example/app/internal/model\""), "repo imports model via module path");
    let svc = &file(&result, "users", FileKind::Service).content;
    assert!(svc.contains("\"github.com/example/app/internal/repository\""), "service imports repo via module path");
}

#[test]
fn multiline_comment_is_flattened_into_one_line() {
    let ddl = "CREATE TABLE `t` (`x` INT NOT NULL COMMENT 'line1\nline2');";
    let result = generate(&request(ddl)).expect("should generate");
    let content = &file(&result, "t", FileKind::Model).content;
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

#[test]
fn golden_composite_pk_ddl_is_locked() {
    let ddl = include_str!("../fixtures/composite_pk_ddl.sql");
    let result = generate(&request(ddl)).expect("composite DDL should parse");
    let actual: Vec<(&str, &str)> = result
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.content.as_str()))
        .collect();
    let golden: Vec<(&str, &str)> = GOLDEN_COMPOSITE.to_vec();
    assert_eq!(actual, golden, "composite-PK golden output changed");
}

#[test]
fn golden_no_pk_ddl_is_locked() {
    let ddl = include_str!("../fixtures/no_pk_ddl.sql");
    let result = generate(&request(ddl)).expect("no-PK DDL should parse");
    let actual: Vec<(&str, &str)> = result
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.content.as_str()))
        .collect();
    let golden: Vec<(&str, &str)> = GOLDEN_NO_PK.to_vec();
    assert_eq!(actual, golden, "no-PK golden output changed");
    assert!(
        result.warnings.iter().any(|w| w.contains("logs") && w.contains("no primary key")),
        "no-PK warning present"
    );
}

#[test]
fn postgres_ddl_generates_full_three_layer_output() {
    let ddl = "CREATE TABLE users (\
        id BIGSERIAL PRIMARY KEY, \
        name VARCHAR(64) NOT NULL, \
        balance NUMERIC(12,2) NOT NULL\
    );";
    let mut req = request(ddl);
    req.dialect = Dialect::PostgreSql;
    let result = generate(&req).expect("should generate");
    let paths: Vec<_> = result.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![
            "internal/model/user.go",
            "internal/repository/user_repo.go",
            "internal/service/user_service.go",
        ]
    );
    let model = &file(&result, "users", FileKind::Model).content;
    assert!(model.contains("Id int64 `gorm:\"column:id;primaryKey;autoIncrement;not null\""), "BIGSERIAL -> int64 autoIncrement, got: {model}");
    assert!(model.contains("Balance string"), "numeric -> string");
}

#[test]
fn postgres_comment_on_becomes_go_comments() {
    let ddl = "CREATE TABLE users (id BIGSERIAL PRIMARY KEY, name VARCHAR(10) NOT NULL);\
        COMMENT ON TABLE users IS 'user table';\
        COMMENT ON COLUMN users.name IS 'the name';";
    let mut req = request(ddl);
    req.dialect = Dialect::PostgreSql;
    let result = generate(&req).expect("should generate");
    let model = &file(&result, "users", FileKind::Model).content;
    assert!(model.contains("// user table"), "table comment, got: {model}");
    assert!(model.contains("\t// the name\n\tName string"), "column comment");
}

#[test]
fn postgres_serial_and_identity_generate_autoincrement() {
    let ddl = "CREATE TABLE users (\
        id SERIAL PRIMARY KEY, \
        big_id BIGSERIAL NOT NULL, \
        tiny_id SMALLSERIAL NOT NULL, \
        seq INT NOT NULL GENERATED BY DEFAULT AS IDENTITY\
    );";
    let mut req = request(ddl);
    req.dialect = Dialect::PostgreSql;
    let result = generate(&req).expect("should generate");
    let model = &file(&result, "users", FileKind::Model).content;
    assert!(model.contains("Id int32 `gorm:\"column:id;primaryKey;autoIncrement;not null\""), "SERIAL -> int32 autoIncrement, got: {model}");
    assert!(model.contains("BigId int64"), "BIGSERIAL -> int64");
    assert!(model.contains("TinyId int16"), "SMALLSERIAL -> int16");
    assert!(model.contains("Seq int32"), "identity base type int -> int32");
    assert!(model.contains("column:seq;autoIncrement"), "identity marks autoIncrement");
}

#[test]
fn postgres_type_mapping_matches_spec() {
    let ddl = "CREATE TABLE t (\
        id BIGSERIAL PRIMARY KEY, \
        flag BOOLEAN NOT NULL, \
        amount NUMERIC(10,2) NOT NULL, \
        extra JSONB, \
        blob_data BYTEA, \
        ts_col TIMESTAMPTZ NOT NULL, \
        uid UUID NOT NULL\
    );";
    let mut req = request(ddl);
    req.dialect = Dialect::PostgreSql;
    let result = generate(&req).expect("should generate");
    let model = &file(&result, "t", FileKind::Model).content;
    assert!(model.contains("Flag bool"), "boolean");
    assert!(model.contains("Amount string"), "numeric -> string");
    assert!(model.contains("Extra json.RawMessage"), "jsonb -> RawMessage (nil expresses null)");
    assert!(model.contains("BlobData []byte"), "bytea -> []byte");
    assert!(model.contains("TsCol time.Time"), "timestamptz -> time.Time");
    assert!(model.contains("Uid string"), "uuid -> string");
}

/// Expected golden output, paired by path. Kept inline so a mismatch prints
/// the exact file content diff.
const GOLDEN: &[(&str, &str)] = &[
    ("internal/model/user.go", include_str!("../fixtures/golden_user.txt")),
    ("internal/repository/user_repo.go", include_str!("../fixtures/golden_user_repo.txt")),
    ("internal/service/user_service.go", include_str!("../fixtures/golden_user_service.txt")),
];

const GOLDEN_COMPOSITE: &[(&str, &str)] = &[
    ("internal/model/order_item.go", include_str!("../fixtures/golden_order_item.txt")),
    ("internal/repository/order_item_repo.go", include_str!("../fixtures/golden_order_item_repo.txt")),
    ("internal/service/order_item_service.go", include_str!("../fixtures/golden_order_item_service.txt")),
];

const GOLDEN_NO_PK: &[(&str, &str)] = &[
    ("internal/model/log.go", include_str!("../fixtures/golden_log.txt")),
];

#[test]
fn golden_pg_sample_ddl_is_locked() {
    let ddl = include_str!("../fixtures/sample_pg_ddl.sql");
    let mut req = request(ddl);
    req.dialect = Dialect::PostgreSql;
    let result = generate(&req).expect("PG golden DDL should parse");
    let actual: Vec<(&str, &str)> = result
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.content.as_str()))
        .collect();
    let golden: Vec<(&str, &str)> = GOLDEN_PG.to_vec();
    assert_eq!(actual, golden, "PG golden output changed");
}

const GOLDEN_PG: &[(&str, &str)] = &[
    ("internal/model/user.go", include_str!("../fixtures/golden_pg_user.txt")),
    ("internal/repository/user_repo.go", include_str!("../fixtures/golden_pg_user_repo.txt")),
    ("internal/service/user_service.go", include_str!("../fixtures/golden_pg_user_service.txt")),
];

#[test]
fn cache_disabled_generates_no_cache_files() {
    let ddl = "CREATE TABLE `users` (`id` BIGINT NOT NULL AUTO_INCREMENT, \
        `n` VARCHAR(10) NOT NULL, PRIMARY KEY (`id`));";
    let result = generate(&request(ddl)).expect("should generate");
    let paths: Vec<_> = result.files.iter().map(|f| f.path.as_str()).collect();
    assert!(!paths.iter().any(|p| p.contains("cache")), "no cache files when disabled");
}

#[test]
fn cache_enabled_generates_cache_file() {
    let ddl = "CREATE TABLE `users` (`id` BIGINT NOT NULL AUTO_INCREMENT, \
        `n` VARCHAR(10) NOT NULL, PRIMARY KEY (`id`));";
    let mut req = request(ddl);
    req.cache_enabled = true;
    req.cache_ttl_seconds = 300;
    let result = generate(&req).expect("should generate");
    let paths: Vec<_> = result.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![
            "internal/model/user.go",
            "internal/repository/user_repo.go",
            "internal/service/user_service.go",
            "internal/cache/user_cache.go",
        ]
    );
    let cache = &result.files.iter().find(|f| f.path.ends_with("user_cache.go")).unwrap().content;
    assert!(cache.contains("package cache"), "cache package");
    assert!(cache.contains("\"github.com/redis/go-redis/v9\""), "imports go-redis");
    assert!(cache.contains("UserCacheTTL = 300 * time.Second"), "TTL constant");
    assert!(cache.contains("type userCacheRepo struct"), "cache impl struct");
    assert!(cache.contains("func NewUserRepoWithCache(repo repository.UserRepository, rdb *redis.Client) repository.UserRepository"), "factory");
    assert!(cache.contains("fmt.Sprintf(\"users:%v\", id)"), "cache key for single PK, got: {cache}");
    assert!(cache.contains("c.rdb.Get(ctx, key)"), "GetByID reads cache");
    assert!(cache.contains("c.rdb.Set(ctx, key, encoded,"), "GetByID writes cache on miss");
    assert!(cache.contains("c.rdb.Del(ctx, key)"), "Update/Delete invalidate");
    assert!(cache.contains("return c.repo.List("), "List passthrough");
    assert!(cache.contains("return c.repo.Create("), "Create passthrough");
}

#[test]
fn cache_composite_pk_key_format() {
    let ddl = "CREATE TABLE `order_items` (`order_id` BIGINT NOT NULL, \
        `item_id` BIGINT NOT NULL, PRIMARY KEY (`order_id`, `item_id`));";
    let mut req = request(ddl);
    req.cache_enabled = true;
    req.cache_ttl_seconds = 600;
    let result = generate(&req).expect("should generate");
    let cache = &result.files.iter().find(|f| f.path.ends_with("order_item_cache.go")).unwrap().content;
    assert!(
        cache.contains("fmt.Sprintf(\"order_items:%v:%v\", orderId, itemId)"),
        "composite key, got: {cache}"
    );
}

#[test]
fn cache_update_uses_model_fields_for_key() {
    let ddl = "CREATE TABLE `users` (`id` BIGINT NOT NULL AUTO_INCREMENT, PRIMARY KEY (`id`));";
    let mut req = request(ddl);
    req.cache_enabled = true;
    req.cache_ttl_seconds = 300;
    let result = generate(&req).expect("should generate");
    let cache = &result.files.iter().find(|f| f.path.ends_with("user_cache.go")).unwrap().content;
    assert!(
        cache.contains("fmt.Sprintf(\"users:%v\", m.Id)"),
        "Update extracts PK from model, got: {cache}"
    );
}

#[test]
fn cache_errors_degrade_to_db() {
    let ddl = "CREATE TABLE `users` (`id` BIGINT NOT NULL AUTO_INCREMENT, PRIMARY KEY (`id`));";
    let mut req = request(ddl);
    req.cache_enabled = true;
    req.cache_ttl_seconds = 300;
    let result = generate(&req).expect("should generate");
    let cache = &result.files.iter().find(|f| f.path.ends_with("user_cache.go")).unwrap().content;
    // GetByID: Redis Get failure falls through to DB
    assert!(cache.contains("m, dbErr := c.repo.GetByID("), "fallback to DB on cache miss/error");
    // Update/Delete: Redis Del failures are silently ignored (` _ = `)
    assert!(cache.contains("_ = c.rdb.Del("), "cache invalidation errors ignored");
    // Set failure also ignored
    assert!(cache.contains("_ = c.rdb.Set("), "cache write errors ignored");
}

#[test]
fn no_pk_table_no_cache_even_when_enabled() {
    let ddl = "CREATE TABLE `logs` (`msg` VARCHAR(200) NOT NULL);";
    let mut req = request(ddl);
    req.cache_enabled = true;
    req.cache_ttl_seconds = 300;
    let result = generate(&req).expect("should generate");
    let paths: Vec<_> = result.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, vec!["internal/model/log.go"], "no cache for no-PK table");
}

#[test]
fn golden_cache_is_locked() {
    let ddl = include_str!("../fixtures/sample_mysql_ddl.sql");
    let mut req = request(ddl);
    req.cache_enabled = true;
    req.cache_ttl_seconds = 300;
    let result = generate(&req).expect("golden DDL should parse");
    let actual: Vec<(&str, &str)> = result
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.content.as_str()))
        .collect();
    let golden: Vec<(&str, &str)> = GOLDEN_CACHE.to_vec();
    assert_eq!(actual, golden, "golden cache output changed");
}

const GOLDEN_CACHE: &[(&str, &str)] = &[
    ("internal/model/user.go", include_str!("../fixtures/golden_user.txt")),
    ("internal/repository/user_repo.go", include_str!("../fixtures/golden_user_repo.txt")),
    ("internal/service/user_service.go", include_str!("../fixtures/golden_user_service.txt")),
    ("internal/cache/user_cache.go", include_str!("../fixtures/golden_user_cache.txt")),
];

