# 软件工厂设计 — gorm-generate

按 `/software-factory` 的术语与设计法走一遍本仓库。这是一份设计稿，不是待办。

**状态（2026-11-27）**：全部仍是设计。A 自动 review 试过一版（PR #6），因仓库没有可供 GitHub Actions 使用的 Anthropic 凭证（API key 与订阅 token 都没有）而关闭，分支已删除。review 改为推送前在会话里跑 `/code-review`，不经过 CI。其余任务若要落地，同样先解决「run 在哪里、用什么凭证」这一前提。

## 0. 要解决的问题

单人维护的公开仓库，当前工作方式是「人开 session → agent 干活 → 人合并」。已具备的资产：

- 仓库标准可被冷读：`AGENTS.md`、`CONCEPTS.md`、`GLOSSARY.md`、`docs/adr/`（10 篇）、`docs/solutions/`、`tasks/lessons.md`（10 条教训）。
- 真实门禁：CI 三个 job（frontend / core 含 `go build` 矩阵 / tauri），clippy `-D warnings`。
- PR 流程真实存在（#1–#5），全部由人合并。

瓶颈与缺口：

1. 每张 ticket 的探索、实现、review 都占用 maintainer 的注意力，没有一件事是自动的。
2. review 靠人逐行找问题；仓库自己的标准文档其实足以让一个 cold run 先审一遍。
3. 检查强度停滞：`cargo fmt` 全仓不过、前端无 E2E、GUI 只能人工冒烟。
4. 公开仓库，但外部报障没有入口（README 未引导 Issues，应用内无入口）。

目标（按优先级）：PR 自动审查 → 已 triage 的小 ticket 自动实现 → 用 ratchet 把严格检查爬上去 → 外部 bug 自动 triage。

## 1. 槽位

| 槽位 | 选型 | 理由 |
| --- | --- | --- |
| Ticket tracker | GitHub Issues | 公开仓库，外部用户天然能提；label 变动可触发 workflow；与 git host 同一事件源。现有五个 triage 角色可直接成为 label。 |
| Ticket source | issue forms（bug / feature）+ 应用内「报告问题」链接 + run 失败 | 应用离线，不能上报，只能用浏览器预填 issue URL；CI 失败由 Actions 自身作为来源。 |
| Git host | GitHub | 已是现状。 |
| Orchestrator | GitHub Actions + 一个 GitHub App token | 单人项目没有理由自建调度器；链式触发需要 App 身份（见 §5 的 `GITHUB_TOKEN` 陷阱）。 |
| Runner | GitHub Actions 标准 runner | 主要用 ubuntu（便宜、够快）；macOS 只给 release 与可选的 GUI smoke。 |
| Observability platform | **故意 none** | 离线桌面应用无遥测；run 的可观测产物就是 PR 评论、review 与日志 artifact，Actions run 页面够用。 |

明确的取舍：

- **不保留本地 markdown tracker 作为工厂入口**：`.scratch/` 没有事件流，工厂要自己做轮询或自建 orchestrator，成本远超收益。`.scratch/` 继续当 spec / wayfinding 工作区，工作记录迁到 Issues。
- **不上 Linear**：单人开源项目，多一个 SaaS、多一条事件流，不加刹车。
- **不买 microVM / agent 平台**：仓库的活是 Go/Rust/Vue 编译与测试，Actions 已覆盖；macOS 产物本来就在 Actions 上构建。

## 2. 任务

### A. 自动 review（第一个上线）

1. **Trigger**：PR 标记 ready-for-review（draft 不触发）；同一 head SHA 只跑一次。
2. **做什么**：冷读 diff，对照关联 ticket、`AGENTS.md`、`CONCEPTS.md`、`tasks/lessons.md` 与 `docs/solutions/`，按仓库的 `/code-review` 口径出 findings，贴成 PR review。只评论，不 approve、不 merge、不 push。
3. **接着触发**：fail → 打 `review-failed` 并触发 Rework；pass → 结束，等人。
4. **人重新介入**：合并永远是人；`ready-for-human` 的 PR 不自动审。
5. **防重复**：concurrency group = PR 号 + head SHA；已有同 SHA 的 bot review 则跳过。

### B. Implementer（只接小 ticket）

1. **Trigger**：issue 打上 `ready-for-agent`。
2. **做什么**：建分支、按 ticket 实现、本地跑全部门禁（`npm run typecheck`、`npm test`、`vite build`、`cargo clippy -D warnings`、`--lib`/`review_regressions`/`wire_format`/`compile`），开 draft PR 引用 issue。不改版本号、不改 ADR、不 merge、不打 tag。
3. **接着触发**：PR 转 ready → A。
4. **人重新介入**：triage 决定哪些 ticket 配这个 label；涉及 ADR / 分发策略 / 对外发布的一律 `ready-for-human`。
5. **防重复**：一个 issue 同时只允许一个 run；分支名带 issue 号；已有未关 PR 则跳过。

### C. Rework

1. **Trigger**：`review-failed` 标签。
2. **做什么**：只修 review 点名的项，push 到同一分支。
3. **接着触发**：A 再跑一次。
4. **人重新介入**：同一 PR 最多 2 轮，之后改打 `ready-for-human`，不再自动修。
5. **防重复**：concurrency group = PR 号；review 未变则不再触发。

### D. Explore / triage

1. **Trigger**：issue opened 且无状态 label；或人给一个 issue 打 `needs-triage`。
2. **做什么**：对照代码复现或证伪，定 `bug`/`enhancement`，查 `.out-of-scope/` 与 `docs/solutions/`，写复现步骤与判定依据，落到五个 triage 角色之一。不改代码、不 push。
3. **接着触发**：小改动 → `ready-for-agent` → B；其余停在 `needs-info` / `ready-for-human` / `wontfix`。
4. **人重新介入**：`needs-info` 等人回复；人回复后在该 issue 上重跑一次。
5. **防重复**：同一 issue 只留一条 triage 评论；已有 bot 评论则跳过。

### E. Ratchet（一天一次）

1. **Trigger**：每日 clock。
2. **做什么**：从 `.factory/ratchet.md` 的例外清单取第一项，修到更严格的检查通过，开一个小 PR。清单顺序：① `cargo fmt`（全仓现在不过，正是「迁移到更严格检查」的教科书写法）→ ② 前端最小 E2E → ③ `render.rs` 按层拆文件（handoff 候选 16）→ ④ SqlEditor / CodePreview 的竞态守卫抽成 composable（候选 15）。
3. **接着触发**：A。
4. **人重新介入**：一天最多一个 PR，人 review；`fmt` 只有在清单清空后才进 CI 门禁。
5. **防重复**：清单文件是唯一状态，取项即标 `in-flight`，PR 合并后勾掉；已有未合并的 ratchet PR 则跳过当天。

### F. Retro（最后上，节流 1/10）

1. **Trigger**：PR merged，按 merge 序号取模 10。
2. **做什么**：读 PR、ticket、review 与 run 日志，向 `tasks/lessons.md` / `docs/solutions/` 提改动 PR。
3. **接着触发**：A。
4. **人重新介入**：全部人 review；`tasks/lessons.md` 是 maintainer 的更正记录，改动必须人拍板。
5. **防重复**：节流本身就是去重；同一模式已记录则不重复提。

### G. Fix（可延后）

1. **Trigger**：orchestrator 报告 run 失败。
2. **做什么**：失败的日志归档为 issue；只有失败落在仓库可控范围（workflow 语法、测试、依赖版本）时才尝试开 PR。
3. **接着触发**：A。
4. **人重新介入**：改工厂自身的 PR 一律人 review。
5. **防重复**：fix run 自身失败不再链式触发。

## 3. Ticket source 去重

- **issue forms**：bug 表单强制带版本、方言、是否开缓存、最小 DDL 片段（脱敏）；标题用固定前缀便于合并重复。
- **应用内「报告问题」**：浏览器预填 title/body（含版本与当前方言），不自动提交；同一版本 + 同类报错在 triage 阶段合并而不是新建。
- **run 失败**：同一 workflow + 同一失败签名 24 小时内只开一张 issue，重复的追加评论。

## 4. 上线顺序与油门

每个 trigger 都是加速器，刹车依次加：

1. **A 自动 review**：人的 review 照旧。跑满 10 个 PR 再评估误报率。
2. **D triage**：新 issue 先只处理 bug 表单，观察一周。
3. **B Implementer**：只对 `ready-for-agent` 的小 ticket 开放（单文件 / 单测试 / 文案级）。
4. **C Rework**：等 A 的失败率稳定后再开，上限 2 轮。
5. **E Ratchet**：每日一条，先做 `fmt`。
6. **F Retro**：1/10。

刹车清单：CI 全绿才允许开 PR（A/B/C/E/F 共用）；A 是独立 run；合并不自动化。B/C 的 run 一律不碰 `release.yml`、版本号、`docs/adr/`。

## 5. 缺口与风险

- **`GITHUB_TOKEN` 陷阱**：用它打 label / 评论 / push 不会触发下一个 workflow，链会静默断掉。从第一天起用 GitHub App 身份（或 `claude-code-action` 自带的 App）。
- **公开仓库 + PR 代码**：review / rework run 会检出 PR head 并可能执行其测试 —— 用 `pull_request`（fork 无 secret），禁止 `pull_request_target` 检出 PR head；issue / PR 正文一律视为不可信数据，不当作指令。
- **成本**：public 仓库的标准 runner 不扣分钟，但这些 run 会消耗 wall-clock；ubuntu 优先，macOS 仅 release。
- **macOS runner 退役**：`macos-15-intel` 可用到 2027-08，x86_64 产物届时要有替代策略（cross-compile 或放弃）。
- **GUI 冒烟仍靠人**：Tauri 的导出链路（写盘、选择器权限）在 runner 上难以稳定复现，短中期保持人工，或另开一个 `tauri-driver` 原型 ticket。
- **`.scratch/` 与 Issues 双轨**：过渡期需要明确「工作记录看 Issues，spec / ADR 看仓库」；否则两份状态会漂移。

## 7. 优先级（2026-11-27）

约束变了：GitHub Actions 里没有可用凭证，凡是「run 在 CI 上自动执行」的 task 都卡在前提之外；能立刻做的只有会话驱动的那部分。按此重排：

| 优先级 | Task | 理由 | 前提 |
| --- | --- | --- | --- |
| P0 | **E Ratchet**，第一项 `cargo fmt` | 唯一不需要远程 orchestrator 的高价值任务：一次一个小 PR、人 review，把「现在不过的检查」变成门禁。 | 无 |
| P0 | **A 的替代**：推送前在会话里跑 `/code-review` | 已定，补成固定习惯即可。 | 无 |
| P1 | **F Retro**（1/10） | 成本低：`tasks/lessons.md` 是仓库最强的资产，让它在合并后自己长。会话里手动触发。 | 无 |
| P1 | **D Explore / triage** | 目前没有外部 issue 流量，做了也没输入；等第一个真实报障再开。 | issue forms + README 入口 |
| P2 | **B Implementer** | 无凭证时与「人开一个会话」没有区别，零增量。 | 先定 run 位置 + 凭证 |
| P2 | **G Fix** | 已被 app 的 Auto-fix 覆盖（不占 Actions 配额、不需要凭证），单独再建是重复。 | 无 |
| — | **C Rework** | 依赖 A，A 已取消，不要建。 | 依赖已死 |

能改变 P2 的只有一个决定：run 在哪里、用什么凭证。三个选项成本递增：① 只做本地工厂（P2 永不做）；② 本机自托管 runner + 已有的 `claude` CLI 登录（不需要 API key，但 Mac 必须在线，且公开仓库必须拒绝 fork PR）；③ API key 或订阅 token（唯一能真正无人值守的路径）。

建议：先做 P0 的 `fmt` ratchet，跑两周看人工 review 成本；收益未验证前不投 P2。

**进展（2026-11-27）**：`fmt` ratchet 已完成 —— 全仓 rustfmt-clean，CI 的 gate 从「例外清单」收敛为 `cargo fmt --all --check`，`rustfmt-exceptions.txt` 与 `scripts/fmt-gate.sh` 已删除。前端最小 E2E 也已落地：`e2e/app.spec.ts`（Playwright + Chromium，Tauri IPC 在浏览器里打桩），CI 的 `frontend` job 会跑。

## 6. 待拍板

1. Ticket tracker 切换到 GitHub Issues（本地 markdown 是否保留为工作区）？
2. 是否接受 bot 以 App 身份 push 分支（这是 B/C 的前提）。
3. 公开 issue 的隐私边界：用户贴的 DDL 会公开，应用内「报告问题」需要提示。
4. 第一个落地物：A 试过并已关闭（原因见顶部状态），当前从 E ratchet 开始，见 §7。
