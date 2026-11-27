# gorm-generate

把 SQL DDL 转成 Go GORM 分层代码的桌面应用。粘贴 MySQL 或 PostgreSQL 的建表语句，离线解析并生成模型、仓储、服务，以及可选的 Redis 缓存装饰器；在界面里预览、按表勾选、复制或按目录结构导出。

全部在本地完成：解析与生成都在 Rust 侧，不连接数据库，不发起网络请求。

## 功能

- 方言：MySQL、PostgreSQL（含 `COMMENT ON` 列注释）
- 生成产物：`internal/model`、`internal/repository`、`internal/service`，缓存开启时另有 `internal/cache`
- 可配置：Go module path、缓存开关与 TTL、`tinyint(1)` 是否映射为 `bool`；选项收在标题栏右侧的配置表单里，标题栏只显示当前取值
- 预览：按层分组的文件树 + Go 语法高亮；表勾选只过滤预览与导出，不重新生成
- 输出：单文件复制、一键复制全部、按目录结构导出到本地目录，同名文件先确认再覆盖
- 依赖：底部列出 `go get` 命令，点一下即可复制，可整行复制
- 分栏：SQL 与结果之间的分隔条可左右拖动，也可聚焦后用方向键调整，位置会记住
- 输入与选项记在 localStorage；解析错误定位到语句序号；方言与脚本对不上时直接提示换方言；不支持的列类型以 `any` 呈现并给警告
- 幂等：相同输入产生字节级相同的输出

需求与契约细节见 [.scratch/gorm-generate/spec.md](.scratch/gorm-generate/spec.md)。

## 已知限制

- 只支持 MySQL 与 PostgreSQL 的 `CREATE TABLE` / `COMMENT ON`；不连接数据库、不内省 schema。
- 方言必须与脚本一致：MySQL 写的反引号与 `AUTO_INCREMENT` 在 PostgreSQL 方言下不生效（后者会被忽略并给警告）。
- 解析错误定位到语句序号并附出错片段；module path、标识符等语义校验错误不归属具体语句。
- 不生成 DTO、事务模板、关联预加载与自定义模板。
- 界面有挂载测试（`src/App.test.ts`）与浏览器 E2E（`e2e/app.spec.ts`，Tauri IPC 打桩；导出写盘链路不在其中）；导出写盘链路已通过真实 macOS `.app` 手动冒烟，仍靠人工验证。

## 技术栈

| 层 | 技术 |
| --- | --- |
| 桌面壳 | Tauri 2 |
| 前端 | Vue 3 + TypeScript + Tailwind CSS 4 + shadcn-vue |
| 生成核心 | Rust（`crates/core`，基于 sqlparser 0.64） |
| 测试 | `cargo test`；`go build` 矩阵需要 Go 工具链 |

## 目录结构

```text
crates/core/          解析、生成、导出；不依赖 Tauri，可独立测试
  src/lib.rs          唯一的公开接口：re-export 列表（ADR-0007）
  src/parser.rs       DDL → 表结构
  src/resolve.rs      表结构 → 已解析表（类型、命名、import、主键）
  src/validate.rs     语义校验：module path、标识符、命名冲突、主键
  src/render.rs                 `table`：一张表产出哪些文件、警告与依赖
    src/render/model.rs         模型
    src/render/repository.rs    仓储
    src/render/service.rs       服务
    src/render/cache.rs         缓存装饰器
    src/render/contract.rs      三层共用的方法契约
  src/generate.rs     串起解析 → 解析表 → 渲染 → 生成产物
  src/types.rs        列类型映射
  src/naming.rs       标识符与命名规则
  src/export.rs       写盘：先规划后写入，路径校验、冲突检测、软链接拒绝
src-tauri/            Tauri 命令（generate_project / export_project / pick_export_dir）
src/                  Vue 界面
  App.vue             装配 composable 与模板
  components/         SqlEditor、CodePreview
  composables/        设置持久化、生成、导出、复制、分栏、高亮的独立状态模块
  lib/                IPC 线格式与设置加载的纯函数
e2e/                  Playwright 浏览器 E2E（Tauri IPC 打桩）
```

## 环境要求

- Rust 1.90（通过 `rustup`）
- Node.js 22 + npm
- Go 1.27.3 —— 仅 `--test compile` 需要
- Tauri 2 的系统依赖（如 Linux 的 WebKitGTK）—— 开发与打包时需要，见 Tauri 官方文档

## 开发

```bash
npm ci
npm run tauri dev
```

只跑前端：`npm run dev`（Vite，默认端口 1420）。

## 测试

| 命令 | 覆盖 |
| --- | --- |
| `cargo test -p gorm-generate-core --lib` | golden 文件 + 单元测试：类型映射、命名、导出路径（含相对根目录） |
| `cargo test -p gorm-generate-core --test review_regressions` | review 回归：重复路径、软链接、staging 失败 |
| `cargo test -p gorm-generate-core --test wire_format --features serde` | IPC 线格式：请求、项目、生成错误与导出错误的 JSON |
| `cargo test -p gorm-generate-core --test compile` | MySQL/PG × 缓存开关 4 格 `go build` 矩阵 + 幂等（需要 Go 与网络） |
| `cargo test -p gorm-generate` | Tauri 命令单测：导出根确认、导出失败线格式 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 全 workspace lint |
| `npm run typecheck` | 前端类型检查 |
| `npx vite build` | 前端打包（CI 也跑，用来抓打包期错误） |
| `npm test` | 前端单测：IPC 错误收窄、设置加载、六个 composable（设置持久化、生成、导出、复制、分栏、高亮），以及 App.vue 的模板接线（挂载测试） |
| `npm run test:e2e` | 浏览器 E2E（Playwright + Chromium，首次先跑 `npx playwright install chromium`）：示例脚本启动、生成与表过滤、设置跨刷新持久化、方言不匹配提示；Tauri IPC 在浏览器里打桩，真实 composable 与模板照常运行 |

`compile` 第一次运行会在临时项目里执行 `go mod tidy` 拉取 gorm 与 go-redis，因此需要能访问 module proxy。

CI 见 [.github/workflows/ci.yml](.github/workflows/ci.yml)，push 到 `main` 与 PR 时跑三个 job：`frontend`（typecheck + 单测 + 打包 + 浏览器 E2E）、`core`（fmt + clippy + 全部 core 测试）、`tauri`（安装 webkit 依赖后跑 Tauri 单测与 clippy）。仓库托管在 [github.com/sword-demon/gorm-generate](https://github.com/sword-demon/gorm-generate)（public）。发布 workflow 见 [.github/workflows/release.yml](.github/workflows/release.yml)（tag 触发，见下面的「发布」）。

## 打包

```bash
npm run tauri build                    # .app + .dmg
npm run tauri build -- --bundles app   # 只出 .app
```

产物默认在 `target/release/bundle/`（若设置了 `CARGO_TARGET_DIR`，则在那个目录下）。

- macOS aarch64 已验证:aarch64 `.app` 约 17 MiB,`.dmg` 约 6 MiB,图标取自 `src-tauri/icons/`。
- 需要换图标时:`npx tauri icon src-tauri/icons/icon.png`,再更新 `bundle.icon`。
- 产物**未签名、未公证**:只带 linker 的 adhoc 签名。项目不提供 Apple Developer ID 签名,用户首次打开时需要按下面的「首次打开未签名 App」放行。
- 沙箱环境里 `.dmg` 打包会因 `hdiutil` 无法访问磁盘设备而失败,需在沙箱外执行。

## 发布

[.github/workflows/release.yml](.github/workflows/release.yml) 把一个 tag 变成 GitHub Release。

本项目的 Release 默认是未签名、未公证的 macOS 产物,不会提供 Apple Developer ID 签名。用户可以正常使用,但首次打开需要手动允许 macOS 放行。

**首次打开未签名 App**

1. 从 Release 下载与你的 Mac 架构对应的 `.dmg`：Apple Silicon 选 `aarch64`,Intel 选 `x64`。
2. 打开 `.dmg`,将 `gorm-generate` 拖到「应用程序」,然后先退出磁盘映像。
3. 在 Finder 的「应用程序」中找到 `gorm-generate`,按住 Control 点击,选择「打开」,再在确认对话框中点击「打开」。不要直接双击反复尝试。
4. 如果仍被拦截:打开「系统设置 → 隐私与安全性」,向下找到被阻止的 `gorm-generate`,点击「仍要打开」,然后再次启动。
5. 仅从本项目 GitHub Release 下载产物;不要对未知来源的 `.app` 使用上述放行步骤。

**发一个版本**

1. 改 `Cargo.toml` 里 `[workspace.package] version`（`package.json` 的 `version` 一并同步）。
2. 确认该 commit 在 `main` 上,且 CI 是绿的。
3. 打 tag 并推送:`git tag v0.2.0 && git push origin v0.2.0`。
4. workflow 在 `macos-15`（arm64）与 `macos-15-intel`（x86_64）上各构建一次,把 `.dmg` 作为 Release 资产上传（`gorm-generate_0.2.0_aarch64.dmg` 等）,并附 GitHub 自动生成的 release notes。

tag 必须与 `Cargo.toml` 的版本一致,否则 workflow 在构建前就会失败。手动重跑:`Actions → Release → Run workflow`,填入 `tagName`（tag 不存在时由 workflow 在当前 commit 上创建）。目前只出 macOS 产物（arm64 与 x86_64 各一份）：前者跑在 `macos-15`,后者跑在 `macos-15-intel`（GitHub 最后一个 x86_64 runner 镜像,可用到 2027-08,之后需要换策略）。

**签名与公证**

本项目不配置 Apple Developer ID 证书,Release 保持未签名、未公证。workflow 仍会正常构建并发布两个架构的 `.dmg`;用户按上面的「首次打开未签名 App」操作即可。

## 文档

- [.scratch/gorm-generate/spec.md](.scratch/gorm-generate/spec.md) —— 需求、实现决策、测试策略
- [GLOSSARY.md](GLOSSARY.md) —— 领域用词
- [docs/adr/](docs/adr/) —— 架构决策记录
- [.scratch/gorm-generate/issues/](.scratch/gorm-generate/issues/) —— issue 与 ticket（本地 markdown tracker）
- [HANDOFF.md](HANDOFF.md) —— 上一次会话的交接说明

## 本地开发提示

- `cargo fmt --all --check` 是门禁，CI 的 `core` job 会跑。仓库已全仓格式化（2026-11-27 用 ratchet 逐文件收尾，见 `.scratch/software-factory/design.md` §7）。
- macOS 沙箱里 cargo 若报 `Operation not permitted`（target 目录或 sccache），用：

  ```bash
  RUSTC_WRAPPER= CARGO_TARGET_DIR="$TMPDIR/cargo-target" cargo test -p gorm-generate-core --lib
  ```

- `.cargo/config.toml` 是本机的稀疏镜像覆盖，已被 `.git/info/exclude` 排除，不属于仓库内容。
- 沙箱内 `--test compile` 会在 `goproxy.cn` 上遇到 TLS 校验失败（`x509: OSStatus -26276`），需要在沙箱外或联网环境运行。

## License

MIT，见 [LICENSE](LICENSE)。
