# 方言不匹配是被诊断出来的，不是被猜出来的

## Status

accepted（补充 ADR-0001、ADR-0004）

## Context

方言是 DDL 脚本所属的数据库类型。MySQL 脚本写反引号、`AUTO_INCREMENT`、`ENGINE=`；PostgreSQL 脚本写 `BIGSERIAL`、`COMMENT ON`、`TIMESTAMPTZ`。两种脚本在对方的方言下要么解析失败，要么被悄悄改了语义。

界面只带一份 MySQL 样例，方言下拉框与编辑器内容互不相干：用户把方言切到 PostgreSQL，手上还是那份 MySQL 样例，点击 Generate 就只得到一段 sqlparser 的原始报错（`Expected: identifier, found: ` ` at Line: 1, Column: 14`）。这段报错指向反引号，不指向真正的问题——方言选错了。

反方向的失败更糟：sqlparser 只在 MySQL 方言下保留 `AUTO_INCREMENT` 列选项，PG 方言下这个 token 被吞掉，主键于是悄悄丢掉 `autoIncrement` 标记。同一段 DDL 在两种方言下生成出不同的模型，而界面一个字的提示都没有。

## Decision

**core 区分三种失败，不把方言问题混进「解析失败」。** `GenerateError` 增加 `DialectMismatch { statement_index, message, snippet }`（ADR-0004 的内部标签随之多一个 `dialectMismatch`）。判定办法是重试：脚本在请求的方言下解析失败、在另一个方言下能解析，就是方言不匹配，`message` 点名两个方言并给出改法（换方言或改 DDL）；两个方言都解析失败，才是真的 DDL 坏了，仍报 `Parse`。

**被方言丢弃的语义改成警告。** PG 方言下静默丢弃的 MySQL `AUTO_INCREMENT` 由 core 在分词阶段发现（token 流里有这个词，而列上没有得到 auto-increment 标记），产出脚本级警告，出现在警告列表里。方言问题一律可见。

**每个方言一份样例，切方言时换掉未被编辑的那份。** 界面带 MySQL 与 PostgreSQL 两份样例。方言变化时，只有当编辑器里的脚本仍然是另一方言的样例原文时，才替换成新方言的样例；用户自己写过的脚本一个字不动。读回旧存档时同样处理：PG 存档配 MySQL 样例的老状态会被换成 PG 样例。

**方言语义保持严格。** 不为了「同一段脚本在两种方言下都能跑」而让 PG 方言接受反引号——PG 方言会连着丢掉 `AUTO_INCREMENT`，宽松解析只会把明确报错换成静默的错误模型。把「换方言」做成一次显式的、可诊断的动作，代价小得多。

## Consequences

- 用户在 PG 方言下打开应用就有可用的样例；贴错方言的脚本会得到一句「这段 DDL 是 MySQL 写的，但方言设成了 PostgreSQL」，而不是原始解析器报错。
- 同一段 DDL 在两种方言下生成不同模型这件事，从静默变成有警告。
- core 的错误联合体多一个变体：Tauri 命令直接收发 core 类型（ADR-0004），不需要改；前端 `src/lib/ipc.ts` 的 `GenerateError` 联合体与 `generateErrorTitle` / `generateErrorDetail` 要跟着加一支。
- 样例与方言绑定了：新增方言要同时加样例，否则新方言打开就是空白。
- 换方言不改动用户脚本这条规则依赖「脚本是否逐字等于样例」这一判断；样例文案一改，老存档里的旧样例就认不出来了（只影响换样例的时机，不影响正确性）。
