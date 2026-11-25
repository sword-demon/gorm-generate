# HANDOFF — gorm-generate

交接说明:给接手的全新 agent。请先读本文件,再读 `AGENTS.md`、`.scratch/gorm-generate/spec.md` 与 `docs/adr/`。**所有回复使用中文 + 英文标点**(全局 `~/.claude/CLAUDE.md` 的强制要求)。

## 1. 项目与环境

- 路径:`/Volumes/MOVESPEED/study/gorm-generate`;纯本地 Tauri 2 + Vue 3 + Rust 核心的离线 DDL → Go 分层代码生成器。
- 当前分支 `main`,HEAD `db54731`(与 `fix/generator-review-hardening` 同一提交)。
- 工作区仅一处未提交:`tasks/lessons.md`(第 7 条,回复语言教训)。用户明确说过"先不提交",不要擅自提交。
- 仓库没有配置任何 remote,因此没有 push / PR 流程。

## 2. 本次会话做了什么

1. 用 `/compound-engineering:ce-code-review` 审查了当时未提交的加固改动(7 个 reviewer 并行 + 合并 + 独立校验)。结论 **Ready with fixes**:2 条 P2、5 条 P3、1 条 P1 验证门。
2. 应用了全部 7 条确认问题,并随提交 `db54731` 落库:
   - `crates/core/src/generate.rs`:重复表键同时按文件名键与 `impl_type_name` 使用的小写类型键判重(`api_users` + `apiuser` 会被拒绝);`validate_table` 改为校验生成的标识符首字符(拒绝 `_1` → `1`)。
   - `crates/core/src/parser.rs`:`DataType::Real` 归入 `double`(MySQL REAL 是 DOUBLE 同义词),`FLOAT`/`FLOAT4` 仍为 `float32`。
   - `crates/core/src/layers.rs`:`param_name` 对全大写标识符整体小写(`"ID"` → `id`,不再产生 `iD`)。
   - `crates/core/src/export.rs`:临时文件改用 `Builder::permissions(0o666)`,按 umask 生成权限,不再固定 0600。
   - `.scratch/gorm-generate/spec.md`:gorm tag 规则补上生成列的 `->`;已知缺口改为真实剩余限制。
   - `src/App.vue`:导出中禁用 Generate;重新生成时清空 `exportMsg`/`copyError`。
3. 新增 3 个单测;并把此前未跟踪的 `crates/core/tests/review_regressions.rs`(12 个测试)纳入版本跟踪。
4. 提交为 `db54731`(24 文件),随后 fast-forward 合入 `main`。

历史 review 报告在 `.scratch/gorm-generate/review.md`;本次 review 的逐条证据在临时目录 `/tmp/.../ce-code-review/20261125-035746-71340ed7/`(重启后可能已清理,结论已写入提交信息与本文件)。

## 3. 验证状态

已通过:

```
RUSTC_WRAPPER= CARGO_TARGET_DIR="$TMPDIR/cargo-target-review" cargo test -p gorm-generate-core --lib
# 40 passed

RUSTC_WRAPPER= CARGO_TARGET_DIR="$TMPDIR/cargo-target-review" cargo test -p gorm-generate-core --test review_regressions
# 12 passed

npm run typecheck
# 通过
```

未验证:

- `crates/core/tests/compile.rs` 的 `go build` 矩阵:沙箱内 TLS 访问 `goproxy.cn` 失败(`x509: OSStatus -26276`),需在沙箱外或联网环境重跑。这对应 spec 的 US35。
- `tauri build` 打包未执行:`bundle.active` 已改为 `true`,但 `bundle.icon` 未设置,产物是否可用未知。
- 没有 UI/e2e 测试;前端改动只过了 `vue-tsc`。

## 4. 未完成 / 待决定

1. **Review #1(验证门,已基本关闭)**:`review_regressions.rs` 已跟踪,覆盖重复路径、软链接、staging 失败;仍缺"相对根目录被拒(`!dir.is_absolute()`)"用例。
2. **PE1(既存问题,非本次引入)**:`src-tauri/src/main.rs` 的 `export_project` 只校验 `dir` 是绝对路径,渲染进程可指定任意根目录写入。建议改为 Rust 侧打开目录选择器,或把最近一次 dialog 返回的根目录存入 managed state 后比对。
3. 其余 residual risks(TOCTOU、CSP 在 dev 不生效、`singularize` 把 `movies` 变 `Movy`、O(n²) 片段构造、缓存构造函数破坏性变更等)见 `.scratch/gorm-generate/review.md` 与提交信息。

## 5. 沙箱与构建注意

- 本机 `cargo` 的 target 在 `~/.target`,且配了 sccache;沙箱内两者都会报 `Operation not permitted`。可用:
  `RUSTC_WRAPPER= CARGO_TARGET_DIR="$TMPDIR/cargo-target-review" cargo test -p gorm-generate-core`
- `cargo fmt --check` 全仓库本来就不通过(多个未改动文件也有 diff),格式不是门禁。
- 网络:goproxy.cn 在沙箱内不可用;`npm` 依赖已存在。

## 6. 建议技能 (suggested skills)

- `compound-engineering:ce-code-review`:下一轮代码审查(`/compound-engineering:ce-code-review`)。
- `compound-engineering:ce-plan` / `compound-engineering:ce-work`:实现 PE1、补相对根目录测试。
- `compound-engineering:ce-debug`:排查 `go build` 矩阵或打包失败。
- `run`:启动 Tauri 应用做手动冒烟(粘贴 → 生成 → 勾选 → 预览 → 复制/导出)。
- `compound-engineering:ce-test-browser`:浏览器侧冒烟(需先起 dev server)。
- `code-review`:轻量、单上下文审查。

## 7. 关键文件

- 生成核心:`crates/core/src/parser.rs`、`generate.rs`、`layers.rs`、`naming.rs`、`types.rs`、`export.rs`
- 测试:`crates/core/src/generate/tests.rs`、`crates/core/tests/review_regressions.rs`、`crates/core/tests/compile.rs`
- 前端 / IPC:`src/App.vue`、`src-tauri/src/main.rs`、`src-tauri/tauri.conf.json`、`src-tauri/capabilities/default.json`
- 规格与流程:`.scratch/gorm-generate/spec.md`、`AGENTS.md`、`docs/agents/*.md`、`GLOSSARY.md`、`docs/adr/`
- 教训日志:`tasks/lessons.md`

## 8. 建议下一步

1. 在沙箱外跑一次 `cargo test -p gorm-generate-core`(含 `compile.rs`),确认 US35 未被本次改动破坏。
2. 决定是否处理 PE1(渲染进程指定导出根目录)。
3. 补一条相对根目录的导出测试,正式关闭 review #1。
4. 手动冒烟 UI:生成、导出冲突确认、导出中点击 Generate 的守卫、全大写主键的生成结果。
