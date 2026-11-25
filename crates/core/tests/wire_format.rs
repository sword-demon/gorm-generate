//! The IPC wire format is the core types' own serde representation
//! (ADR-0004). These tests pin the JSON the Tauri layer and the UI rely on,
//! so removing the payload mirrors cannot silently change the contract.
#![cfg(feature = "serde")]

use gorm_generate_core::generate::Dialect;
use gorm_generate_core::{generate, GenerateRequest, GeneratedFile};

fn request(ddl: &str) -> GenerateRequest {
    GenerateRequest {
        ddl: ddl.into(),
        dialect: Dialect::MySql,
        module_path: "example.com/app".into(),
        cache_enabled: false,
        cache_ttl_seconds: 300,
        tinyint_as_bool: false,
    }
}

#[test]
fn request_deserializes_from_the_camel_case_frontend_shape() {
    let json = r#"{
        "ddl": "CREATE TABLE t (id BIGINT PRIMARY KEY);",
        "dialect": "postgres",
        "modulePath": "example.com/app",
        "cacheEnabled": true,
        "cacheTtlSeconds": 300,
        "tinyintAsBool": true
    }"#;

    let parsed: GenerateRequest = serde_json::from_str(json).expect("request wire format");
    assert_eq!(parsed.dialect, Dialect::PostgreSql);
    assert_eq!(parsed.module_path, "example.com/app");
    assert!(parsed.cache_enabled);
    assert!(parsed.tinyint_as_bool);

    let mysql: GenerateRequest =
        serde_json::from_str(&json.replace("\"postgres\"", "\"mysql\"")).expect("mysql dialect");
    assert_eq!(mysql.dialect, Dialect::MySql);
}

#[test]
fn project_serializes_files_with_their_kind() {
    let project = generate(&request(
        "CREATE TABLE users (id BIGINT PRIMARY KEY, name TEXT NOT NULL);",
    ))
    .expect("should generate");

    let json = serde_json::to_value(&project).expect("project serializes");
    for key in ["files", "tables", "warnings", "dependencies"] {
        assert!(json.get(key).is_some(), "missing {key}");
    }
    assert_eq!(json["dependencies"][0], "gorm.io/gorm");

    let file = &json["files"][0];
    for key in ["path", "content", "table", "kind"] {
        assert!(file.get(key).is_some(), "missing {key} in file");
    }
    assert_eq!(file["kind"], "model");
    assert_eq!(file["table"], "users");
}

#[test]
fn errors_serialize_as_a_tagged_union() {
    let parse = generate(&request("CREATE TABLE broken (")).expect_err("invalid DDL");
    let json = serde_json::to_value(&parse).expect("error serializes");
    assert_eq!(json["kind"], "parse");
    assert!(json["statementIndex"].is_u64(), "got {json}");
    assert!(json["snippet"].is_string(), "got {json}");

    let mut bad_request = request("CREATE TABLE t (id BIGINT PRIMARY KEY);");
    bad_request.module_path = String::new();
    let validation = generate(&bad_request).expect_err("invalid module path");
    let json = serde_json::to_value(&validation).expect("error serializes");
    assert_eq!(json["kind"], "validation");
    assert!(json["message"].is_string(), "got {json}");
}

#[test]
fn generated_file_round_trips_through_the_exporter_payload() {
    let project = generate(&request(
        "CREATE TABLE users (id BIGINT PRIMARY KEY, name TEXT NOT NULL);",
    ))
    .expect("should generate");
    let file = project.file("users", gorm_generate_core::FileKind::Model).expect("model");

    let json = serde_json::to_string(file).expect("file serializes");
    let back: GeneratedFile = serde_json::from_str(&json).expect("file deserializes");
    assert_eq!(&back, file);
}
