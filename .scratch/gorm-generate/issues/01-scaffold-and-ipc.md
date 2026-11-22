# 01: 脚手架与 IPC 打通

**What to build:** 一个可启动的 Tauri 2 桌面应用，前端是 Vue 3 + TypeScript + Tailwind CSS 4 + shadcn-vue，Rust 侧把生成核心库与 Tauri 命令层分开。应用启动后点击按钮能调用一个 ping 命令并把返回值显示在页面上，证明前后端通道与构建链路可用。版本基线按 spec（Tauri 2.12.x、Tailwind 4.3.x、shadcn-vue 2.9.x）。

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] 应用可通过一条命令启动，窗口显示单页界面
- [x] 点击按钮调用 Rust 命令并把结果回显到界面
- [x] Rust 核心库与 Tauri 命令层分离，命令层只做参数转发
- [x] `cargo test`、`vue-tsc`、前端生产构建全部通过
- [x] 依赖版本与 spec 的版本基线一致

## Comments

2026-11-22: Implemented as the initial scaffold commit.

- Rust workspace: `crates/core` (`gorm-generate-core`) owns logic; `src-tauri` is a thin Tauri command layer that only forwards `ping`.
- TDD: `ping` test written red first, then implemented green.
- Frontend: Vue 3.5 + TS + Tailwind CSS 4.3 + shadcn-vue 2.9 Button; single-page ping UI.
- Versions: tauri 2.12.7, @tauri-apps/cli 2.12.6, tailwindcss 4.3.5, shadcn-vue 2.9.0.
- Verified: `cargo test` (workspace) pass; `npm run build` (vue-tsc --noEmit + vite build) pass; `npm run tauri dev` launched the app process with the dev server.
- Manual smoke (click Ping and see the echo) is intentionally not automated per the spec's testing decisions.
- A local-only `.cargo/config.toml` (git-excluded via `.git/info/exclude`) pins a sparse registry mirror on this machine because the global TUNA git index was months stale and blocked cargo resolution; it is intentionally not part of the repo.
