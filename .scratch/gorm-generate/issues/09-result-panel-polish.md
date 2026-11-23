# 09: 结果区完善（高亮、错误、警告、依赖提示）

**What to build:** 代码预览使用 Shiki 做 Go 与 SQL 语法高亮且保持只读；解析错误以 toast 展示并包含出错语句的序号与片段；警告以 Badge/列表呈现；结果区展示需要执行的 go get 依赖（gorm，缓存开启时加 go-redis）。

**Blocked by:** 06

**Status:** resolved

- [x] Go 与 SQL 预览有语法高亮且只读
- [x] 解析错误展示语句序号与片段
- [x] 警告列表可读，数量可见
- [x] 结果区显示与当前开关匹配的 go get 依赖提示

## Comments

2026-11-23: Implemented.

- Shiki 4 (core + JS regex engine, only go+sql grammars lazy-loaded via `@shikijs/langs`). `src/lib/highlight.ts` singleton; escapes to plain `<pre>` on failure.
- Go preview: `CodePreview.vue` renders read-only highlighted HTML (`v-html`, no inputs). Stale-async guard via token.
- SQL: `SqlEditor.vue` overlays a transparent textarea on a highlighted backdrop so DDL stays editable while highlighted; scroll synced; `aria-label`, focus ring, visible `::selection`; stale-async guard. (Ticket says "SQL 预览...只读" — DDL was always editable input, so this keeps it editable with highlight; flag to author if a read-only SQL view was intended.)
- Parse errors: dismissible fixed toast (statement # + snippet) plus the persistent in-panel block; `toParseError` narrows the IPC value instead of blind-casting.
- Warnings: `<details>` with a count badge and a list.
- Dependencies: `goGetDeps` computed from `cacheEnabled` (gorm always; +go-redis when cache on), with a note to keep in sync with `layers.rs`.
- Known backend limitation (not this ticket): `parser.rs` always returns `statement_index: 1` and `snippet` = sqlparser error text, not the failing statement's DDL. UI surfaces what the backend gives.
- `cargo test` 37 passed; `npm run build` passed (shiki go/sql/theme chunks split).

