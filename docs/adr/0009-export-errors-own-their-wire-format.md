# 导出失败也是 core 的线格式

## Status

accepted（补充 ADR-0004、ADR-0005）

## Context

`ExportError` 没有 serde 表示，Tauri 的 `export_project` 因此返回 `ExportResultPayload { conflicts, error }`：一个手工镜像——把 `Conflicts` 拆成数组，把其他变体压成 `to_string()`，未选目录再压成同一个字符串字段。四处字面量构造，前端靠 `conflicts.length` 区分「可确认的冲突」和「失败」。这与 ADR-0004（core 自带宽格式）和 ADR-0005（core 拥有事实）的方向相反：导出的失败分类又变成了 Tauri 的私有约定。

## Decision

`ExportError` 加 serde 内部标签，与 `GenerateError` 同形：`{kind:"unsafePath", path}`、`{kind:"duplicatePath", path}`、`{kind:"conflicts", paths}`、`{kind:"io", message}`。重复路径从 `UnsafePath` 的字符串里分出来，成为独立变体，这样每个字段都是原始事实而不是拼接后的消息。

Tauri 命令改为 `Result<(), ExportFailure>`，`ExportFailure` 是本地 `#[serde(untagged)]` 包装：`Core(ExportError)` 原样透传，`NoRoot { kind: "noRoot", message }` 表示「没有经过 picker 确认的目录」——这是 Tauri 的安全前置条件，core 不该知道。`ExportResultPayload` 与 `impl Display for ExportError` 一并删除：唯一的 Human 文案由前端按 `kind` 组织。

## Consequences

- IPC 上只有一个带 `kind` 的对象，前端用一个判别联合处理四个 core 失败加一个 Tauri 失败。
- 冲突仍是「用户可覆盖的失败」：前端在 `conflicts` 上弹确认，其余直接显示。
- `Conflict` 的确认文案与安全错误文案由前端负责；Rust 侧不再保留第二套文案。
- 新增导出失败原因时：core 加变体、前端加分支，Tauri 层不需要再改映射。
- `wire_format.rs` 钉住四种变体的 JSON，防止字段改名悄悄破坏 IPC。
