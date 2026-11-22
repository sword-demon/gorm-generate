# 02: 首条生成路径（MySQL 模型 → 预览 → 复制）

**What to build:** 用户粘贴一段 MySQL DDL 后点击生成，右侧出现按表拆分的模型文件列表；点开文件能看到只读的纯文本代码预览；支持复制当前文件与复制全部。生成由 Rust 核心库的纯函数完成，Tauri 命令只是薄包装。解析错误时给出提示而不崩溃。

**Blocked by:** 01

**Status:** resolved

- [x] 粘贴含一张或多张表的 MySQL DDL 能生成对应的模型文件
- [x] 模型包含结构体、`TableName()` 与基础 GORM/JSON tag
- [x] 文件列表按表展示，预览显示完整文件内容
- [x] 单文件复制与全部复制可用
- [x] 解析失败时界面显示错误提示，应用不崩溃
- [x] 存在一份 golden 测试锁死该样例 DDL 的完整生成产物

## Comments

2026-11-23: Implemented.

- Core: `parser` (sqlparser 0.64, MySQL dialect, AUTO_INCREMENT as DialectSpecific token), `naming` (singular PascalCase struct + snake_case file), `types` (spec type mapping incl. decimal->string, tinyint(1) switch, unsigned by width, unknown->any+warning), `generate` renders `internal/model/<singular>.go` with gorm/json tags and `deleted_at` -> `gorm.DeletedAt`.
- Tests at the `generate` seam: 8 unit tests (type map, naming, nullable pointers, tags, multi-table, parse error, tinyint switch, unknown type) + golden test locking `src/fixtures/sample_mysql_ddl.sql` against `src/fixtures/golden_user_model.go`.
- Tauri: `generate_project` command, thin wrapper, structured ParseErrorPayload (statement_index + snippet).
- Frontend: left DDL textarea, right file list + readonly `<pre>` preview, copy file / copy all, warnings banner, parse error panel.
- Verified: `cargo test` 9 passed; `npm run build` (vue-tsc + vite) passed; `npm run tauri dev` launched (process + vite alive, no panic). UI click-through is the manual smoke item per spec.
