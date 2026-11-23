# 05: PostgreSQL 方言

**What to build:** 界面增加方言选择；选择 PostgreSQL 时能解析 PG 的建表语句与 COMMENT ON 并生成与 MySQL 等价的模型、仓储、服务三类产物；SERIAL/BIGSERIAL/SMALLSERIAL 与 GENERATED AS IDENTITY 识别为自增；PG 类型按 spec 映射；提供 MySQL 与 PG 两份 golden 基线。

**Blocked by:** 04

**Status:** resolved

- [x] 方言选择器在 MySQL 与 PostgreSQL 之间切换
- [x] PG DDL 生成完整三层产物
- [x] COMMENT ON TABLE/COLUMN 的注释出现在生成代码中
- [x] SERIAL/IDENTITY 列生成自增标记
- [x] PG 类型映射符合 spec（boolean、numeric→string、jsonb→RawMessage、bytea→[]byte、timestamptz→time.Time、uuid→string）
- [x] MySQL 与 PG 各有一份 golden 基线

## Comments

2026-11-23: Implemented.

- parser: PostgreSqlDialect branch; `Statement::Comment{TABLE/COLUMN}` merged into parsed tables (case-insensitive identifier matching, PG folds unquoted identifiers); SERIAL family parses as `Custom` types normalized to int/bigint bases.
- auto-increment: MySQL AUTO_INCREMENT token, PG `SERIAL`/`BIGSERIAL`/`SMALLSERIAL` (Custom), and `GENERATED ... AS IDENTITY` (`ColumnOption::Generated`) all set the autoIncrement tag.
- type mapping additions: numeric/bignumeric -> string, bytea -> []byte, uuid -> string, timestamptz rides the existing timestamp -> time.Time path.
- `[]byte` and `json.RawMessage` restored as never-pointer (nil expresses null); MySQL sample golden re-recorded accordingly.
- Golden: PG baseline locked (sample_pg_ddl.sql with COMMENT ON, BIGSERIAL PK, soft delete) alongside the MySQL baselines.
- Frontend: PostgreSQL option enabled; Tauri command accepts "postgres".
- Verified: `cargo test` 24 passed; `npm run build` passed.
- Review fixes: duplicate `COMMENT ON` now last-write-wins (replaces existing option instead of appending); SMALLSERIAL mapped to `smallint` base (`int16`) with a test; dead `timestamptz` arm removed from types.rs; spec's 类型映射 bullet updated to record the `[]byte`/`json.RawMessage` pointer exemption so future agents don't "fix it back".
