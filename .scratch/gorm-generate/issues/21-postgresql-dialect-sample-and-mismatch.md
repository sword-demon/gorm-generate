# 21: PostgreSQL 方言下自带样例无法解析

**What to build:** 方言切到 PostgreSQL 之后，应用自带的样例 DDL（MySQL 写法，带反引号）在 PG 方言下解析失败，界面只给出 sqlparser 的原始报错（`Expected: identifier, found: ` `），既点不出「是方言选错了」，也没有一份能用的 PG 样例。修法：每个方言各带一份样例，切方言时把「仍是样例、没被编辑过」的脚本换成新方言的；core 在一种方言下解析失败、在另一种下能解析时，报「方言不匹配」而不是原始解析错误。

**Blocked by:** None

**Status:** resolved

**Category:** bug

## Comments

2026-11-26: Reproduced and fixed.

- 根因有两层。其一，界面只有一份 MySQL 样例，方言下拉框与编辑器内容互相脱节；其二，core 的错误只分「解析失败」与「语义校验失败」，方言选错时用户拿到的是一段原始解析报错，诊断信息止步于「反引号不是合法的 PG 标识符引号」。
- 另查出一处静默错误：sqlparser 只在 MySQL 方言下保留 `AUTO_INCREMENT` 列选项，PG 方言下这个 token 被吞掉，主键因此悄悄丢掉 `autoIncrement` 标记，界面上没有任何提示。同一段 DDL 在两种方言下会生成出不同的模型。
- 修法（core）：`GenerateError` 增加 `DialectMismatch { statement_index, message, snippet }` 一种，解析失败时用另一个方言重试一次，能解析就报不匹配。`AUTO_INCREMENT` 在 PG 下被丢弃的情况改为脚本级警告，不再静默。
- 修法（前端）：`settings.ts` 存两份样例（MySQL / PostgreSQL），`sampleForDialect` 只在脚本仍是另一方言的样例时才替换；`useSettings` 在方言变化时替换，`loadState` 在读回旧存档时也替换（PG 存档 + MySQL 样例的老状态会被换成 PG 样例）。用户自己写的脚本一律不动。
- 验证：core `--lib` 61 passed、`--test review_regressions` 15 passed、`--test wire_format` 5 passed、`--test compile` 2 passed(沙箱外)；`cargo clippy --workspace --all-features --all-targets` 无告警；前端 `npm run typecheck`、`npm test`(94 passed)与 `npm run build` 全绿；浏览器里验证了「切方言换样例」与「老存档读回时换样例」。**本次改动尚未在 CI 上跑过**。
