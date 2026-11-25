# core 类型自带线格式

## Status

accepted（补充 ADR-0001）

## Context

Tauri 层曾为 `GenerateRequest`、`GeneratedProject`、`GeneratedFile`、`FileKind` 与 `GenerateError` 各维护一个逐字段镜像，并在两侧手写转换。给 core 加一个字段要同步三处：漏在请求上编译不过，漏在响应上被 `#[serde(default)]` 静默吞掉。镜像类型与 core 类型同构，删除它们不会丢失任何行为，只会让「谁来定义跨 IPC 的 JSON」这个问题重新出现。

## Decision

core 的公开类型通过可选的 `serde` feature 派生自身的线格式：`GenerateRequest` 只 `Deserialize`，`GeneratedProject` 与 `GenerateError` 只 `Serialize`，`GeneratedFile` 与 `FileKind` 两个方向都要。Tauri 命令直接收发 core 类型，不再拥有 payload 类型。feature 默认关闭，core 的默认构建不依赖 serde；serde 不是 Tauri，且 `src-tauri` 显式启用该 feature，不违反 ADR-0001 的独立性。

线格式是接口的一部分，由 `crates/core/tests/wire_format.rs` 锁定：camelCase 的请求、小写的 `kind`、内部标签的 `kind: parse | validation`。校验错误因此改为命名字段 `Validation { message }`，以便内部标签序列化。

## Consequences

- 新增字段只改 core 一处，IPC 与前端自动跟随。
- 前端 JSON 形状不变：键名、`kind` 取值与错误标签都与镜像时代一致。
- 未知方言不再映射为结构化 `Validation`，而是 serde 反序列化失败；方言来自固定下拉框，不可达。
- core 的发布物多一个可选 feature，未启用时没有 serde 依赖。
