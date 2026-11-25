//! End-to-end: generated code must compile with `go build ./...`, and the
//! same input must generate byte-identical output (idempotency) across the
//! MySQL/PostgreSQL × cache on/off matrix.
//!
//! Run with: `cargo test --test compile`
//! Requires a Go toolchain on PATH (the ticket pins Go 1.27.3). `go mod tidy`
//! resolves gorm/go-redis from the module cache or proxy, so the first run on
//! a clean machine needs network access.

use std::path::{Path, PathBuf};
use std::process::Command;

use gorm_generate_core::{export_project, generate, Dialect, GenerateRequest};

/// One matrix cell: a fixture, its dialect, and the cache switch.
struct Case {
    name: &'static str,
    ddl: &'static str,
    dialect: Dialect,
    cache_enabled: bool,
}

fn cases() -> Vec<Case> {
    let mysql = include_str!("../src/fixtures/sample_mysql_ddl.sql");
    let pg = include_str!("../src/fixtures/sample_pg_ddl.sql");
    vec![
        Case { name: "mysql_cache_off", ddl: mysql, dialect: Dialect::MySql, cache_enabled: false },
        Case { name: "mysql_cache_on", ddl: mysql, dialect: Dialect::MySql, cache_enabled: true },
        Case { name: "pg_cache_off", ddl: pg, dialect: Dialect::PostgreSql, cache_enabled: false },
        Case { name: "pg_cache_on", ddl: pg, dialect: Dialect::PostgreSql, cache_enabled: true },
    ]
}

fn request(case: &Case) -> GenerateRequest {
    GenerateRequest {
        ddl: case.ddl.to_string(),
        dialect: case.dialect,
        module_path: "github.com/example/app".to_string(),
        cache_enabled: case.cache_enabled,
        cache_ttl_seconds: 300,
        tinyint_as_bool: false,
    }
}

/// A temp project dir that is removed on drop — even if an assert panics.
struct TempProject(PathBuf);

impl TempProject {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "gorm-generate-compile-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp project dir");
        TempProject(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Writes the generated files plus a go.mod into `dir`.
fn write_project(dir: &Path, case: &Case) {
    let project = generate(&request(case)).expect("generate");

    // Guard against a vacuous pass: an empty module builds and is trivially
    // idempotent, so require the expected layers to exist.
    let paths: Vec<&str> = project.files.iter().map(|f| f.path.as_str()).collect();
    assert!(!paths.is_empty(), "generate produced no files");
    assert!(paths.iter().any(|p| p.starts_with("internal/model/")), "missing model file: {paths:?}");
    assert!(paths.iter().any(|p| p.ends_with("_repo.go")), "missing repository file: {paths:?}");
    assert!(paths.iter().any(|p| p.ends_with("_service.go")), "missing service file: {paths:?}");
    assert_eq!(
        paths.iter().any(|p| p.ends_with("_cache.go")),
        case.cache_enabled,
        "cache file presence must match the switch: {paths:?}"
    );

    export_project(dir, &project.files, true).expect("export generated files");

    // go.mod: module path matches the generator's module_path so the internal
    // imports resolve. Requires are pinned to versions known to resolve.
    let go_mod = "module github.com/example/app\n\ngo 1.27\n\nrequire (\n\tgorm.io/gorm v1.31.2\n";
    let go_mod = if case.cache_enabled {
        format!("{go_mod}\tgithub.com/redis/go-redis/v9 v9.7.0\n)\n")
    } else {
        format!("{go_mod})\n")
    };
    std::fs::write(dir.join("go.mod"), go_mod).unwrap();
}

/// Runs a command in `dir`, returning (success, combined output).
fn run(dir: &Path, program: &str, args: &[&str]) -> (bool, String) {
    let out = Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to run `{program}`: {e}"));
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), combined)
}

#[test]
fn generated_code_compiles_across_matrix() {
    for case in cases() {
        let project = TempProject::new(case.name);
        write_project(project.path(), &case);

        // `go mod tidy` resolves go.sum from the module cache/proxy; then build.
        let (tidy_ok, tidy_out) = run(project.path(), "go", &["mod", "tidy"]);
        assert!(tidy_ok, "[{}] go mod tidy failed:\n{tidy_out}", case.name);

        let (build_ok, build_out) = run(project.path(), "go", &["build", "./..."]);
        assert!(build_ok, "[{}] go build failed:\n{build_out}", case.name);
    }
}

#[test]
fn generation_is_idempotent_across_matrix() {
    for case in cases() {
        let first = generate(&request(&case))
            .unwrap_or_else(|_| panic!("[{}] first generate", case.name));
        let second = generate(&request(&case))
            .unwrap_or_else(|_| panic!("[{}] second generate", case.name));

        // Guard against a vacuous pass: empty == empty is trivially identical.
        assert!(!first.files.is_empty(), "[{}] generated no files", case.name);

        let first_files: Vec<_> = first.files.iter().map(|f| (&f.path, &f.content)).collect();
        let second_files: Vec<_> = second.files.iter().map(|f| (&f.path, &f.content)).collect();
        assert_eq!(first_files, second_files, "[{}] output not byte-identical", case.name);
    }
}
