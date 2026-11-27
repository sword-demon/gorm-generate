# HANDOFF — gorm-generate

交接说明:给接手的全新 agent。请先读本文件,再读 `AGENTS.md`、`README.md`、`.scratch/gorm-generate/spec.md` 与 `docs/adr/`。**所有回复使用中文 + 英文标点**(全局 `~/.claude/CLAUDE.md` 的强制要求,`tasks/lessons.md` 第 7 条记录了违反一次的教训)。

## 1. 项目与环境

- 路径:`/Volumes/MOVESPEED/study/gorm-generate`;纯本地 Tauri 2 + Vue 3 + Rust 核心的离线 DDL → Go 分层代码生成器。
- 默认分支 `main`;本文件记录对应的提交是 `d88d938`(把版本号 bump 到 0.2.0 的提交,也是 `v0.2.0` 的 tag 目标)。
- 远端:`origin git@github.com:sword-demon/gorm-generate.git`,**public**;About 为中文描述,topics 已配置。
- 发布记录(全部未签名、未公证;自 `v0.1.1` 起由 `.github/workflows/release.yml` 在 tag 上自动发布,资产名形如 `gorm-generate_<版本>_<架构>.<扩展名>`):

  | tag | tag 目标 | aarch64 `.dmg` | aarch64 `.app.tar.gz` | x64 `.dmg` | x64 `.app.tar.gz` |
  | --- | --- | --- | --- | --- | --- |
  | `v0.1.0` | `21155dd` | 6.06 MiB(另附 6.07 MiB `.app.zip`) | 无 | 无 | 无 |
  | `v0.1.1` | `08571d6` | 6.07 MiB | 6.08 MiB | 6.41 MiB | 6.43 MiB |
  | `v0.1.2` | `20d935f`(PR #3) | 6.07 MiB | 6.08 MiB | 6.41 MiB | 6.43 MiB |
  | `v0.1.3` | `5c8a7ba`(PR #5) | 6.07 MiB | 6.08 MiB | 6.42 MiB | 6.44 MiB |
  | `v0.2.0` | `d88d938`(PR #26) | 6.16 MiB | 6.17 MiB | 6.50 MiB | 6.52 MiB |

  各次的 release run 号见 §3。
- `.cargo/config.toml` 是本机的 rsproxy 稀疏镜像覆盖,被 `.git/info/exclude` 排除,不属于仓库内容;不要提交它。
- 更早的 39 个已合并旧分支(含 `chore/bump-0.1.3`)与其远端分支已删除;`main` 之外还留着 PR #20–#26 的七个分支(本地与远端同名):`test/frontend-e2e`、`refactor/render-layers`、`refactor/use-highlight`、`docs/retro-1`、`feat/report-entry`、`docs/report-smoke`、`chore/release-0.2.0`。它们都是 squash 合并后的残留(内容已在 `main`),可随时 `git branch -D` + `git push origin --delete` 清掉(squash 合并的分支对 `-d` 不算已合并)。

## 2. `21155dd` 之后做了什么(2026-11-25 → 11-27)

1. 架构审查:按 `/improve-codebase-architecture` 出报告,逐个实现候选 1–7(已解析表 IR、层契约、生成错误拆变体、校验与依赖进 core、删 payload 镜像、core 自带宽格式、导出先规划后写),每个都配 ADR-0003 至 ADR-0009。
2. 候选 8–13:收窄 core 公开接口(只留 re-export)、渲染器自持表产出、导出错误自持线格式、CI 加 clippy 与 Tauri 测试、前端 IPC 边界抽出 `src/lib/ipc.ts`、App.vue 拆成四个 composable。
3. 候选 14–19(承接前端):TTL 输入归一化(修真 bug)、App.vue 挂载测试、README/spec/HANDOFF 去漂移、CI 加 `vite build`。
4. 更早一次会话(记录在旧版本文件里)完成了 ticket 11–17、MIT LICENSE、发布到 GitHub 与首个 release `v0.1.0`,并把 workflow action 升到 Node 24 大版本。
5. 2026-11-27 按 `.scratch/software-factory/design.md` 推进工厂:E ratchet 四项(逐文件 rustfmt 收口 → 前端 Playwright E2E → `render.rs` 按层拆文件 → 高亮竞态守卫抽成 `useHighlight`)、第一次 F retro(PR #23)、外部报障入口(issue forms + README + 应用内问号按钮,PR #24/#25),最后发 `v0.2.0`(PR #26 + tag)。

## 3. 验证状态

已通过:

- CI(`.github/workflows/ci.yml`)现在三个 job:`frontend (typecheck + tests + e2e)`、`core (unit + regressions + go build matrix)`、`tauri (unit tests + clippy)`。最近一次全绿是 `45997184842`(frontend 50s / core 1m03s / tauri 3m53s)。前端的步骤是 `npm ci` → typecheck → `npm test` → `npx vite build` → 装 Chromium 后跑 `npm run test:e2e`;`core` job 装好 Rust 后的第一个检查是 `cargo fmt --all --check`。action 为 `checkout@v7` / `setup-node@v7` / `cache@v6` / `setup-go@v7`。
- 日志确认各测试目标真实执行:`--lib` 61 passed、`--test review_regressions` 15 passed、`--test wire_format` 5 passed、`--test compile` 2 passed(4 格矩阵 + 幂等)、Tauri 单测 2 passed、前端 108 passed(12 个文件)、e2e 5 passed。
- 本地:`npm run typecheck`、`npm test`、`npm run build` 与上述 Rust 目标均通过;`--test compile` 需在沙箱外运行(见 §5)。
- **ticket 21 改动本地验证(2026-11-26)**:`cargo test -p gorm-generate-core --all-features --lib --test review_regressions --test wire_format` 全绿(61 + 15 + 5);`cargo clippy --workspace --all-features --all-targets` 无告警;`cargo test -p gorm-generate`(Tauri)2 passed;前端 `npm run typecheck`、`npm test`(94 passed)与 `npm run build` 全绿;`cargo test -p gorm-generate-core --test compile` 在沙箱外跑通(2 passed);浏览器 dev server 里验过「切方言换样例」与「老存档读回时换样例」。**该改动已随 PR #4 在真实 CI 上跑绿,并发布为 `v0.1.3`**。
- 打包:macOS aarch64 `.app`(17.38 MiB)与 `.dmg`(6.06 MiB),图标已嵌入;`.dmg` 需在沙箱外生成。`v0.1.0` release 里的两个资产就是这组构建(另打包了 6.07 MiB 的 `.app.zip`)。
- 新增 `.github/workflows/release.yml`:tag 触发 → `macos-15`(aarch64)与 `macos-15-intel`(x86_64)各构建一次 → `tauri-apps/tauri-action@v1` 上传 GitHub Release;tag 必须与 `Cargo.toml` 版本一致(构建前校验);签名/公证的 secrets 可选,不配就是未签名产物。
- **发布流水线已在真实 CI 上跑通四次**:run `45657109208`(2026-11-26)发布 `v0.1.1`;run `45700406121`(2026-11-26)发布 `v0.1.2`;run `45786047539`(2026-11-26)发布 `v0.1.3`;run `45997285730`(2026-11-27)发布 `v0.2.0`,四次的双格(macos-15 aarch64 / macos-15-intel x64)都绿、资产齐全。第一次跑(`45652646653`)挂在 `tauri-action` 插的 `--` 与我自带的 `--` 叠加,已修(`tauriScript: npm run tauri`,不带 `--`),教训记在 `tasks/lessons.md` 第 8 条。
- **界面改动(2026-11-26)**:标题栏只留标题、当前配置摘要、生成按钮与最右端的配置按钮(方言/module path/缓存/TTL/`tinyint(1)` 移进表单);SQL 与结果之间加可拖动分隔条(方向键也能调,宽度记进 localStorage);底部依赖可一键复制 `go get` 命令。前端 83 个测试全绿,浏览器 dev server 里验过头部单行、配置表单(勾 Redis 后出现 TTL)、拖动后宽度与持久化。
- **全仓简化(PR #1 / #2,2026-11-26)**:删掉渲染、解析与前端监听里的重复工作(层契约每张表只算一次、PK import 片段共用一个 helper、解析错误片段改为惰性构造、`ident_value` 只留一份、主键 membership 用集合、窗口监听清理复用同一组函数引用),生成的 Go 产物逐字节不变(黄金 fixture 全绿);顺带修掉两处「测试声称覆盖但其实没断言」的用例。经验记在 `docs/solutions/design-patterns/shared-implementation-versus-shared-computation.md`,术语在 `CONCEPTS.md`。
- **PG 方言修复(ticket 21,2026-11-26)**:用户报障「切到 PostgreSQL 后自带样例解析不了」。根因两层:界面只有一份 MySQL 样例且与方言下拉框无联动;core 的错误分类把方言问题压进了「解析失败」。修法:core 加 `DialectMismatch { statement_index, message, snippet }`(用另一个方言重试一次判定),PG 下被 sqlparser 吞掉的 MySQL `AUTO_INCREMENT` 改为脚本级警告而非静默;前端每个方言一份样例,切方言时只替换「仍是样例原文」的脚本(读回旧存档同样替换)。决策与理由记在 `docs/adr/0010`,ticket 在 `.scratch/gorm-generate/issues/21-*.md`,教训记在 `tasks/lessons.md` 第 10 条。

补充验证:

- **GUI 手动冒烟已完成(2026-11-26)**:用 Screen Recording / 辅助功能权限驱动本地构建的 `.app`。已验证:窗口渲染、Generate 真的生成了 4 个文件(model / repository / service / cache)、文件树、Go 高亮预览、配置表单(修改选项生效)、SQL 与结果分隔条拖动、单条依赖复制、Copy all、Export to directory,以及底部依赖行(`gorm.io/gorm`、`github.com/redis/go-redis/v9`)。用户确认上述真实交互全部成功。
- **报障入口冒烟(2026-11-27)**:release `.app`(含 PR #24)点标题栏问号,先弹出公开性确认,确认后系统浏览器打开预填版本、方言与缓存开关的 GitHub bug 表单;用户确认成功。取消分支未在真机点过(单测覆盖)。
- CI 的 `pull_request` 触发已随 PR #4 与 #5 真实走过(两者的 CI 都绿)。

遗留:

- "故意改坏模板验证 CI 变红"没有做(ticket 15 已注明)。

## 4. Ticket 与已知缺口

- `.scratch/gorm-generate/issues/` 下 21 张 ticket **全部 `resolved`**;ticket 18 记录了 CI clippy 与 Tauri 单测的落地,ticket 19 记录了本轮界面改动与真实 GUI 验收,ticket 20 记录了「不提供 Apple 签名」的决策,ticket 21 记录了 PG 方言样例与方言不匹配诊断的修复,详情在各文件末尾的 Comments。
- 仍存在的产品限制(已写进 README 的"已知限制"):只支持 MySQL/PG;语义校验错误(module path、标识符、主键)不归属具体语句;导出写盘链路仍以真实 GUI 手动冒烟覆盖。前端已有浏览器 E2E(`e2e/app.spec.ts`,Playwright + Chromium,Tauri IPC 打桩),但它覆盖界面与 IPC 调用参数,不覆盖真实写盘。
- **分发约束(ticket 20,已定,不是待办)**:产物刻意未签名、未公证,没有 Apple Developer ID 证书。用户按 README「首次打开未签名 App」的步骤放行(拖到 Applications → Control-点击「打开」→ 必要时「隐私与安全性 → 仍要打开」)。`release.yml` 已不再引用任何 `APPLE_*` secret。
- 历史 review 报告在 `.scratch/gorm-generate/review.md`,其中 A/B 段的修复主题已落库;该文档是快照,别当成当前状态。

## 5. 沙箱与构建注意

- 本机 cargo 的 target 在 `~/.target`,且配了 sccache;沙箱内两者都会报 `Operation not permitted`。可用:
  `RUSTC_WRAPPER= CARGO_TARGET_DIR="$TMPDIR/cargo-target-review" cargo test -p gorm-generate-core --lib`
- `cargo fmt --all --check` 现在是 CI 门禁(`core` job 装好 Rust 后的第一个检查);全仓已 rustfmt-clean,改完 Rust 先跑 `cargo fmt --all`。
- 网络:沙箱内 `goproxy.cn` TLS 校验失败(`x509: OSStatus -26276`),`--test compile` 必须去沙箱外跑;`hdiutil`(做 `.dmg`)在沙箱内也失败。
- `gh` 在沙箱内读不到 keyring 且网络被拦;**GitHub 相关命令走用户的终端标签**。`gh auth refresh` 在当前版本没有 `-u` 参数,用 `gh auth refresh -h github.com`。

## 6. 建议技能 (suggested skills)

- `run`:启动 Tauri 应用做手动冒烟(导出写盘链路仍只能这样验)。
- `code-review`:推送前在会话里跑(仓库没有可用于 CI 的 Anthropic 凭证,见 `.scratch/software-factory/design.md` 顶部状态)。
- `compound-engineering:ce-test-browser`:前端浏览器侧交互冒烟;自动化回归交给 `npm run test:e2e`(Playwright + 打桩)。
- `compound-engineering:ce-debug`:排查打包签名或跨平台问题。

## 7. 关键文件

- 生成核心:`crates/core/src/lib.rs`(唯一公开接口)、`parser.rs`(分词错误定位)、`resolve.rs`(已解析表)、`validate.rs`、`render.rs` + `render/{contract,model,repository,service,cache}.rs`(渲染器按层拆文件,`render::table` 仍是唯一 crate 内接口)、`generate.rs`、`naming.rs`、`types.rs`、`export.rs`
- 测试:`crates/core/src/generate/tests.rs`、`parser.rs` 内嵌测试、`crates/core/tests/{review_regressions,wire_format,compile}.rs`、`src-tauri/src/main.rs` 内嵌测试、`src/**/*.test.ts`
- 前端 / IPC:`src/App.vue`(装配)、`src/composables/`(设置 / 生成 / 导出 / 复制 / 高亮 / 分栏六个)、`src/lib/ipc.ts`(线格式收窄)、`src/lib/report.ts`(报障 URL 预填)、`e2e/app.spec.ts`(Playwright)、`src-tauri/src/main.rs`(`ExportRoot` + `pick_export_dir`)、`src-tauri/tauri.conf.json`、`src-tauri/capabilities/default.json`、`src-tauri/icons/`
- CI:`.github/workflows/ci.yml`
- 规格与流程:`.scratch/gorm-generate/spec.md`、`AGENTS.md`、`README.md`、`GLOSSARY.md`、`docs/adr/`
- 教训日志:`tasks/lessons.md`

## 8. 建议下一步

1. 等第一个真实报障进 GitHub Issues,在会话里跑一次 D triage(入口已备:issue forms + README + 应用内问号按钮);issue / triage / domain 三份 agent 文档的分工见 `docs/agents/issue-tracker.md`。
2. 工厂剩余项仍卡在「run 在哪里、用什么凭证」上,见 `.scratch/software-factory/design.md` §7;凭证落定前不要建 B Implementer / C Rework。
3. 对外分发:维持未签名/未公证策略,用户按 README「首次打开未签名 App」放行。下次发版照 `d88d938` 的做法走 PR:`Cargo.toml` 的 workspace 版本、`package.json`、`Cargo.lock`、`package-lock.json` 与 `bug_report.yml` 的版本占位一起改,合并后再打 tag。
4. ratchet 清单(设计稿 §E)的 ①–④ 都已落地(进展见设计稿 §7);下一次 F retro 按 merge 序号 1/10 在会话里手动跑。
