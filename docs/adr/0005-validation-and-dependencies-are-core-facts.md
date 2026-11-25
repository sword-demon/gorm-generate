# 校验与依赖事实归 core

## Status

accepted（补充 ADR-0001、ADR-0004）

## Context

前端曾复制 core 的 module path 与缓存 TTL 规则，并按用户开关自行推导 `go get` 依赖；`goGetDeps` 的注释直接要求与 `layers.rs` 手工保持同步。规则漂移只能靠纪律发现。候选 3 让错误带上 `kind`，ADR-0004 让 core 类型自带宽格式，前端已经不再需要解释任何来自 Rust 的值。

## Decision

选项校验只存在于 core 的 `validate_request`；前端不做预校验，直接渲染 `Validation` 错误。生成物所需的第三方依赖由 core 从实际发出的文件推导，经 `GeneratedProject.dependencies` 返回；前端只渲染。方言通过 serde 枚举穿过 IPC（ADR-0004），不再有字符串二次映射。

## Consequences

- 前端不再保存任何 core 事实的副本，改规则只改一处。
- 无效输入要经过一次 IPC 才能看到错误；生成本来就要往返，不增加额外等待。
- 依赖提示与产物一致：“缓存开关打开但所有表都没有主键”不会生成缓存文件，也就不会提示 `go get` redis。
- 前端展示的校验文案改为 core 的英文消息；UI 仍按 `kind` 区分解析错误与输入错误。
