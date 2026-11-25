# 渲染器拥有每张表的产出

## Status

accepted

## Context

`generate()` 曾同时做编排、警告拼接、依赖推导和模型渲染。「没有主键只出模型」这条规则出现在三个地方：编排循环决定跳过哪几层、`validate_tables` 用它决定哪些表参与类型冲突检查、`dependencies` 用它决定是否提示 gorm/redis。依赖推导还要手工镜像渲染器的判断（`has_layers() || imports.gorm`、`cache_enabled && has_layers`），注释里写着「与实际发出的文件保持一致」——一致性只能靠纪律。模型渲染器又留在 `generate.rs`，与另外三个渲染器隔着一个文件。

## Decision

`render::table(&Table, &GenerateRequest) -> TableOutput` 是渲染模块的唯一 crate 内接口。`TableOutput` 携带一张表的全部产出：

- `files`：模型，加上（有主键时）仓储、服务与缓存文件；
- `warnings`：已带表名前缀的表级警告，以及无主键说明；
- `dependencies`：产出文件实际需要的第三方模块，由各渲染器声明，例如仓储恒需 gorm、缓存需 redis、模型仅在有 `deleted_at` 时需 gorm。

`generate()` 只做流水线：校验 → 解析 → 解析表 → 逐表摊平 → 去重汇总依赖。校验搬进 `validate.rs`；模块 `layers` 更名为 `render`，因为模型渲染器也进来了。

## Consequences

- 一条规则只在一处：无主键表的产出在 `render::table` 内决定，`generate()` 不再知道任何层的条件。
- 依赖随产出声明，`dependencies()` 与其镜像注释删除；缓存文件存在意味着主键、仓储、gorm 都已存在，因此 gorm 总在 redis 之前，现有顺序断言保持不变。
- 表级规则可直接对 `render::table` 单测，不必经过整个 `generate()`。
- `generate.rs` 现在只有线格式与流水线；新增层或调整某层的 import，只需改对应渲染器。
- `GeneratedProject.warnings` 与 `dependencies` 的字符串内容不变，黄金测试与前端无需改动。
