# 08: 表选择与本地持久化

**What to build:** 生成结果中列出表清单，用户可勾选/取消表，文件树、预览与导出随之过滤但不重新生成；关闭并重开应用后，上次的 DDL、方言、module path、缓存开关、TTL、tinyint(1) 开关被恢复。

**Blocked by:** 05, 06

**Status:** resolved

- [x] 表清单出现且默认全选
- [x] 取消勾选后文件树、预览与导出同步过滤
- [x] 勾选变化不触发重新生成
- [x] 重开应用后 SQL 与全部选项恢复

## Comments

2026-11-23: Implemented.

- `GeneratedFile` gains `table: String` (core sets it on model/repo/service/cache files; Tauri payload carries it). This is the seam the UI filters on — deriving it in TS would duplicate Rust's singularize logic.
- Frontend: table checkboxes from `project.tables`; `generate()` sets `selectedTables = new Set(tables)` (default all). `visibleFiles` filters tree/preview/Copy-all/export; toggling never re-invokes `generate`.
- Persistence: all options (DDL, dialect, modulePath, cacheEnabled, cacheTtlSeconds, tinyintAsBool) in one `localStorage` key via a `watch`; `loadState` validates each field so corrupt/stale values fall back to defaults instead of failing the next generate. Table selection is intentionally not persisted (ticket scopes restore to SQL + options).
- Tests: `every_file_carries_its_source_table`; `cargo test` 37 passed, `npm run build` and `cargo build -p gorm-generate` pass.
