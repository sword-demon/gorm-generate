# 03: MySQL 模型完整规则

**What to build:** 把 MySQL 模型生成补全到 spec 约定的全部规则：可空列指针、decimal/numeric 用 string、json 用 RawMessage、enum/set 用 string、二进制用 []byte、时间用 time.Time、unsigned 按宽度、未知类型用 any；deleted_at 生成软删除类型；表/列注释变成 Go 注释；tinyint(1) 默认整型、界面开关切换为 bool；生成文件带 DO NOT EDIT 头；未知类型与被忽略语句进入警告列表并在界面可见；相同输入连续生成结果一致。

**Blocked by:** 02

**Status:** resolved

- [x] 复杂表 DDL（含各类类型、可空、注释、软删除）生成的模型符合 spec 的类型映射表
- [x] tinyint(1) 默认按 `int8`/`uint8`，打开开关后为 `bool`
- [x] 表注释与列注释出现在生成代码中
- [x] 未知类型生成 `any` 并出现在警告列表，界面可看到警告
- [x] 非建表语句被忽略并计数进警告
- [x] 生成文件带 DO NOT EDIT 头
- [x] 幂等测试：同一输入连续两次生成结果完全一致

## Comments

2026-11-23: Implemented on top of ticket 02's model path.

- Table `COMMENT='...'` extracted from `CreateTableOptions::Plain` (MySQL path, not the Hive-only `comment` field); column `COMMENT '...'` from `ColumnOption::Comment`; both become Go comments verbatim (table comment above struct, column comment above field).
- Non-CREATE-TABLE statements counted: `parser::ParsedScript.ignored_count` -> warning "ignored N non-CREATE-TABLE statement(s)".
- Idempotence test: same input twice produces identical file contents (pure function by construction, test locks it).
- Frontend: `tinyint(1) as bool` checkbox wired into the generate request.
- Golden fixture re-recorded for the table comment (`// 用户表`).
- Verified: `cargo test` 13 passed; `npm run build` passed.
- Known deferrals (recorded for later tickets): PG `COMMENT ON` statements are counted as ignored here (PG dialect is ticket 05); warnings render as a plain banner instead of a Badge component (ticket 02 UI form, spec wants Badge).
