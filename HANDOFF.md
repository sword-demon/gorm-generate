# HANDOFF — gorm-generate

交接说明:给接手的全新 agent。请先读本文件,再读 `AGENTS.md`、`README.md`、`.scratch/gorm-generate/spec.md` 与 `docs/adr/`。**所有回复使用中文 + 英文标点**(全局 `~/.claude/CLAUDE.md` 的强制要求,`tasks/lessons.md` 第 7 条记录了违反一次的教训)。

## 1. 项目与环境

- 路径:`/Volumes/MOVESPEED/study/gorm-generate`;纯本地 Tauri 2 + Vue 3 + Rust 核心的离线 DDL → Go 分层代码生成器。
- 当前分支 `main`,记录时 HEAD `21155dd`(MIT license、workflow action 升级、handoff 刷新),与 `origin/main` 一致,工作区干净。
- 远端:`origin git@github.com:sword-demon/gorm-generate.git`,**public**;About 为中文描述,topics 已配置。
- 唯一的 release:`v0.1.0`,注释 tag 打在 `21155dd`(与 main 一致),附 `gorm-generate_0.1.0_aarch64.dmg` 与 `.app.zip`,未签名未公证;正文在 GitHub 上,没有入库的副本。
- `.cargo/config.toml` 是本机的 rsproxy 稀疏镜像覆盖,被 `.git/info/exclude` 排除,不属于仓库内容;不要提交它。
- 本地保留了 13 个已合并分支(`chore/tickets-and-ci`、`test/export-relative-root`、`docs/readme`、`build/bundle-icon`、`fix/tokenizer-statement-index`、`fix/export-root-trust`、`docs/verify-compile-matrix`、`docs/publish-record`、`docs/close-ci-ticket`、`chore/mit-license`、`fix/generator-review-hardening`、`docs/handoff-refresh`、`chore/bump-workflow-actions`),都指向 main 历史,可安全删除。

## 2. 本次会话做了什么(2026-11-25)

1. `/triage`:发现 10 个旧 ticket 全部 `resolved`,但剩余工作只存在于 HANDOFF/review/spec 里;新建 ticket 11–17 把缺口落进 tracker。
2. 逐个完成并关闭:

| ticket | 提交 | 内容 |
| --- | --- | --- |
| 13 | `959655e` | 补相对根目录导出测试 |
| 16 | `0ff08b8` | 根目录 README |
| 14 | `49cc1f9` | 图标集 + `bundle.icon`,首次 `tauri build` 验证 |
| 17 | `fdcf0ec` | 分词错误上报真实语句序号 |
| 12 | `5cd33fd` | 导出目录改由 Rust 侧 picker 决定 |
| 11 | `e78529f` | 沙箱外复跑 `go build` 矩阵,通过 |
| 15 | `2fc5920` + `4575e3b` + `47d8999` | CI workflow、发布到 GitHub、首次绿灯记录 |

3. 另加 MIT `LICENSE`(`b474c6d`),并设置 GitHub About 与 topics。
4. 收尾:重写本文件(`8758644`);把 CI 里的 action 升到 Node 24 大版本 —— `checkout@v7`、`setup-node@v7`、`cache@v6`、`setup-go@v7`(`d9c9384`),推送后 CI 复跑绿灯,弃用注解消失;最后重新 `tauri build` 并发布首个 release `v0.1.0`。

## 3. 验证状态

已通过:

- CI(`.github/workflows/ci.yml`)六次运行全绿:`45596095866`、`45596604627`、`45597088323`,加上 `45605761508`、`45608235921`(action 升级后,frontend 15s + core 1m6s)与 `45608894475`。两个 job:`frontend (typecheck)`、`core (unit + regressions + go build matrix)`,action 现为 `checkout@v7` / `setup-node@v7` / `cache@v6` / `setup-go@v7`。
- 日志确认三个测试目标都真实执行:`--lib` 45 passed、`--test review_regressions` 12 passed、`--test compile` 2 passed(4 格矩阵 + 幂等)。
- 本地:`npm run typecheck`、`cargo test -p gorm-generate-core --lib`、`--test review_regressions` 均通过;`--test compile` 需在沙箱外运行(见 §5)。
- 打包:macOS aarch64 `.app`(17.38 MiB)与 `.dmg`(6.06 MiB),图标已嵌入;`.dmg` 需在沙箱外生成。`v0.1.0` release 里的两个资产就是这组构建(另打包了 6.07 MiB 的 `.app.zip`)。

未验证:

- **GUI 手动冒烟完全没有做**(当前环境起不了窗口):粘贴 → 生成 → 勾选 → 预览 → 复制 → 导出,尤其 ticket 12 改过的"Rust 选择器 → 写盘 → 冲突确认"链路,只有编译期保证。
- CI 的 `pull_request` 触发没有真实 PR 走过。
- "故意改坏模板验证 CI 变红"没有做(ticket 15 已注明)。
- 产物未签名、未公证。

## 4. Ticket 与已知缺口

- `.scratch/gorm-generate/issues/` 下 17 张 ticket **全部 `resolved`**;ticket 11–17 是本次新增,详情在各文件末尾的 Comments。
- 仍存在的产品限制(已写进 README 的"已知限制"):只支持 MySQL/PG;语义校验错误(module path、标识符、主键)不归属具体语句,以序号 0 表示;界面流程没有自动化测试。
- 历史 review 报告在 `.scratch/gorm-generate/review.md`,其中 A/B 段的修复主题已落库;该文档是快照,别当成当前状态。

## 5. 沙箱与构建注意

- 本机 cargo 的 target 在 `~/.target`,且配了 sccache;沙箱内两者都会报 `Operation not permitted`。可用:
  `RUSTC_WRAPPER= CARGO_TARGET_DIR="$TMPDIR/cargo-target-review" cargo test -p gorm-generate-core --lib`
- `cargo fmt --check` 全仓库本来就不通过,格式不是门禁。
- 网络:沙箱内 `goproxy.cn` TLS 校验失败(`x509: OSStatus -26276`),`--test compile` 必须去沙箱外跑;`hdiutil`(做 `.dmg`)在沙箱内也失败。
- `gh` 在沙箱内读不到 keyring 且网络被拦;**GitHub 相关命令走用户的终端标签**。`gh auth refresh` 在当前版本没有 `-u` 参数,用 `gh auth refresh -h github.com`。

## 6. 建议技能 (suggested skills)

- `run`:启动 Tauri 应用做手动冒烟(当前唯一未做的关键验证)。
- `compound-engineering:ce-code-review` / `code-review`:下一轮审查。
- `compound-engineering:ce-test-browser`:前端浏览器侧冒烟(需先起 dev server)。
- `compound-engineering:ce-debug`:排查打包签名或跨平台问题。

## 7. 关键文件

- 生成核心:`crates/core/src/parser.rs`(分词错误定位)、`generate.rs`、`layers.rs`、`naming.rs`、`types.rs`、`export.rs`
- 测试:`crates/core/src/generate/tests.rs`、`parser.rs` 内嵌测试、`crates/core/tests/review_regressions.rs`、`crates/core/tests/compile.rs`
- 前端 / IPC:`src/App.vue`、`src-tauri/src/main.rs`(`ExportRoot` + `pick_export_dir`)、`src-tauri/tauri.conf.json`、`src-tauri/capabilities/default.json`、`src-tauri/icons/`
- CI:`.github/workflows/ci.yml`
- 规格与流程:`.scratch/gorm-generate/spec.md`、`AGENTS.md`、`README.md`、`GLOSSARY.md`、`docs/adr/`
- 教训日志:`tasks/lessons.md`

## 8. 建议下一步

1. 手动冒烟 UI,重点验证导出链路(选择器权限已收窄到 Rust,前端不再持有 `dialog:allow-open`)。
2. 若要对外分发:配置 Developer ID 签名与公证,并补 `tauri build` 的发布流程。
3. 可选:补一条 `pull_request` 触发的真实验证(建临时 PR),或给前端加最小 e2e。
4. 可选:删除已合并的本地分支,保持 `git branch` 干净。
