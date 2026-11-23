# 07: 导出到目录

**What to build:** 用户选择一个本地目录后，把当前生成产物按 internal/... 的目录结构写入磁盘；写入由 Rust 命令完成，不依赖前端文件系统插件；目标已存在同名文件时先弹出确认，确认后覆盖；拒绝绝对路径与 `..` 路径穿越。临时目录测试覆盖结构、内容与覆盖行为。

**Blocked by:** 02

**Status:** resolved

- [x] 选择目录后按预览中的目录结构写出文件
- [x] 写入内容与预览完全一致
- [x] 同名文件存在时先确认再覆盖
- [x] 非法路径（绝对路径、`..`）被拒绝
- [x] 临时目录测试验证结构、内容与覆盖行为

## Comments

2026-11-23: Implemented.

- `core::export::export_project(dir, files, overwrite)` writes under the chosen dir, creating parents; validates every path via `safe_join` (rejects non-`Normal` components: absolute, `..`, `.`) before touching disk, so a bad path leaves nothing half-written.
- Conflict pre-check returns `ExportError::Conflicts(relative paths)` when `overwrite` is false; caller confirms then retries with `overwrite: true`.
- `ExportError` implements `Display` for readable UI messages.
- Tauri `export_project` command is a thin wrapper; reuses `GeneratedFilePayload` for both directions.
- Frontend: `@tauri-apps/plugin-dialog` `open({directory:true})` → export → on conflicts `window.confirm` → retry overwrite. Capability limited to `dialog:allow-open`.
- Tests: 5 in `export.rs` (structure+content, conflict, overwrite, absolute reject, `..` reject) via `std::env::temp_dir`. `cargo test` 36 passed, `cargo build -p gorm-generate` and `npm run build` pass.
- Note for ticket 08: export currently sends all files; when table-checkbox filtering lands, filter before sending.
