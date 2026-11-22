# 05: PostgreSQL 方言

**What to build:** 界面增加方言选择；选择 PostgreSQL 时能解析 PG 的建表语句与 COMMENT ON 并生成与 MySQL 等价的模型、仓储、服务三类产物；SERIAL/BIGSERIAL/SMALLSERIAL 与 GENERATED AS IDENTITY 识别为自增；PG 类型按 spec 映射；提供 MySQL 与 PG 两份 golden 基线。

**Blocked by:** 04

**Status:** ready-for-agent

- [ ] 方言选择器在 MySQL 与 PostgreSQL 之间切换
- [ ] PG DDL 生成完整三层产物
- [ ] COMMENT ON TABLE/COLUMN 的注释出现在生成代码中
- [ ] SERIAL/IDENTITY 列生成自增标记
- [ ] PG 类型映射符合 spec（boolean、numeric→string、jsonb→RawMessage、bytea→[]byte、timestamptz→time.Time、uuid→string）
- [ ] MySQL 与 PG 各有一份 golden 基线
