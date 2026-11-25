# core 的公开接口就是它的 re-export 列表

## Status

accepted

## Context

`lib.rs` 曾同时暴露两套接口：一份 re-export 列表，以及每个实现模块的 `pub mod`。调用方既可以用 `gorm_generate_core::generate(...)`，也可以走进 `gorm_generate_core::parser::ParsedTable`、`gorm_generate_core::types::classify_mysql_type(&DataType)`。后者把 sqlparser 的 AST 类型带进了本 crate 的公开契约：升级 sqlparser 可能是别人的 breaking change，而 `types`、`naming`、`layers` 这些纯内部概念也被算作承诺。

## Decision

实现模块全部私有（`mod export; mod generate; mod layers; mod naming; mod parser; mod resolve; mod types;`），`lib.rs` 的 re-export 列表是 crate 唯一的公开接口，`Dialect` 一并上提。`ping` 脚手架随之下线。

不为「第三方类型不得出现在公开接口」加测试：Rust 的可见性已是编译期守卫，用 grep 断言不可达只会得到一条脆弱的测试。这条约束靠本 ADR 与 `lib.rs` 顶部的注释维持。

## Consequences

- 公开名字收敛为 9 个：`generate`、`GenerateRequest`、`GenerateError`、`Dialect`、`FileKind`、`GeneratedFile`、`GeneratedProject`、`export_project`、`ExportError`。sqlparser 升级不再是对外契约。
- 集成测试只能走公开接口——它们现在正好验证这份接口是否够用；本轮只改了三处导入。
- 新增对外能力时先改 re-export 列表，而不是顺手 `pub` 一个模块。
- 将来的第二个适配器（如 CLI）与 Tauri 一样，从 crate 根调用。
