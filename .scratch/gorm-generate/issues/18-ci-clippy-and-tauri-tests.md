# 18: CI 加 clippy 门禁与 Tauri 单测

**What to build:** 给 CI 补上 lint 与 `src-tauri` 测试的守卫：`core` job 增加 `cargo clippy -D warnings`，并新增 `tauri` job 跑 Tauri crate 的单元测试与 clippy。

**Status:** resolved
**Category:** enhancement

**Blocked by:** None

**Why:** 此前 CI 只跑 core 测试与前端 typecheck。clippy 从未进入门禁，全 workspace 累计 4 条 `iter().copied().collect()` 警告；`src-tauri` 的测试（`export_requires_a_picker_confirmed_root`、`export_failures_reach_the_ui_as_one_tagged_shape`）在 CI 中从不执行，后者是 ADR-0009 导出线格式的唯一守卫。

**推翻了什么:** issue 15 的「不要在 CI 里构建 src-tauri」。当时理由是「当前测试只覆盖 core」，该前提已随候选 10 失效。

## Steps

1. 清掉 4 条警告（`.to_vec()`）。
2. `core` job 在测试前加 `cargo clippy -p gorm-generate-core --all-targets -- -D warnings`（默认 features；serde 构建由 wire_format 步骤覆盖）。
3. 新 `tauri` job：ubuntu-latest、Rust 1.90.0、apt 安装 `libwebkit2gtk-4.1-dev` / `libayatana-appindicator3-dev` / `librsvg2-dev` / `patchelf`、源码缓存、`timeout-minutes: 25`、clippy + `cargo test -p gorm-generate`。debug 构建走 `devUrl`，无需前端构建（本地移走 `dist/` 验证过）。
4. README 的测试表与 CI 描述同步。

## 验收

- [x] 本地 `cargo clippy --workspace --all-targets -- -D warnings` 通过
- [x] 本地 `cargo test -p gorm-generate` 通过（无 `dist/` 也通过）
- [x] 推送后 GitHub Actions 三个 job 全绿

## Comments

2026-11-25: 起草并落地。真实 CI 运行待推送后确认；如果 apt 包名在 runner 镜像上不可用，退路是用 Tauri 文档的完整 apt 清单。

2026-11-25: 首次运行失败（run 45630538849）：两个 job 都在 clippy 步骤报 `'cargo-clippy' is not installed for the toolchain '1.90.0'`。`rustup toolchain install --profile minimal` 不含 clippy。修复：安装步骤加 `--component clippy`。`tauri` job 的 apt 依赖已通过，未暴露包名问题。

2026-11-25: run 45631313323 三个 job 全绿（`12be731`）。

- core：clippy `-D warnings` 通过；`--lib` 61 passed、review_regressions 12、wire_format 5、compile 矩阵 2 passed（20s）。
- tauri：clippy 通过；`cargo test` 2 passed（含 `export_failures_reach_the_ui_as_one_tagged_shape`）。
- tauri job 从 clone 到测试结束约 2 分钟，成本低于预估。
- run 45630538849 的 clippy 缺组件问题由 `12be731` 修复。
