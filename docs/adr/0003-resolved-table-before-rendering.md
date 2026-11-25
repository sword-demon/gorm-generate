# 渲染前先解析成已解析表

## Status

accepted（补充 ADR-0001）

## Context

类型映射、Go 命名、import 需求与缓存 key 格式原先分散在 `generate.rs` 的模型渲染、`layers.rs` 的 `pk_columns` 与三个层渲染器里，各算一遍；缓存 key 甚至由 `GetByID` 与 `Update` 两处分别拼接。每加一条映射规则或改一次命名，都要同步多个位置，而且没有任何一个值能回答「这一列在 Go 里是什么」。

## Decision

`parse_ddl` 之后、任何渲染之前，`resolve_table` 把 `ParsedTable` 与生成选项解析成一张已解析表（`Table` / `Column`）：Go 名、Go 类型、gorm 标记、原始注释、主键索引、条件 import、缓存 key 格式与逐表警告。四个渲染器、命名冲突检测与缓存 key 都只读这一份事实，不再自行推导。`resolve_table` 必须是全函数；标识符合法性与命名冲突留给其后的校验步骤。

渲染格式（注释折叠、gorm tag 拼接、JSON tag、import 块顺序）仍留在渲染器，不进已解析表。

## Consequences

- 类型映射与命名只有一个 owner，层渲染器变成薄的文本发射器。
- 已解析表是 crate 私有的 module interface，可以直接单测；golden 继续锁完整产物。
- 新增列类型或命名规则时，只需改 `resolve` 与 `types`，不必再碰三个层模板。
