# gorm-generate

把 SQL DDL 转成 Go GORM 分层代码的桌面应用。粘贴 MySQL 或 PostgreSQL 的建表语句，离线解析并生成模型、仓储、服务，以及可选的 Redis 缓存装饰器；在界面里预览、按表勾选、复制或按目录结构导出。

全部在本地完成：解析与生成都在 Rust 侧，不连接数据库，不发起网络请求。

## 功能

- 方言：MySQL、PostgreSQL（含 `COMMENT ON` 列注释）
- 生成产物：`internal/model`、`internal/repository`、`internal/service`，缓存开启时另有 `internal/cache`
- 可配置：Go module path、缓存开关与 TTL、`tinyint(1)` 是否映射为 `bool`
- 预览：按层分组的文件树 + Go 语法高亮；表勾选只过滤预览与导出，不重新生成
- 输出：单文件复制、一键复制全部、按目录结构导出到本地目录，同名文件先确认再覆盖
- 输入与选项记在 localStorage；解析错误定位到语句序号；不支持的列类型以 `any` 呈现并给警告
- 幂等：相同输入产生字节级相同的输出

需求与契约细节见 [.scratch/gorm-generate/spec.md](.scratch/gorm-generate/spec.md)。

## 已知限制

- 只支持 MySQL 与 PostgreSQL 的 `CREATE TABLE` / `COMMENT ON`；不连接数据库、不内省 schema。
- 解析错误定位到语句序号并附出错片段；module path、标识符等语义校验错误不归属具体语句。
- 不生成 DTO、事务模板、关联预加载与自定义模板。
- 界面逻辑有挂载测试（`src/App.test.ts`），但导出写盘链路仍只有手动冒烟。

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
  src/render.rs       模型 / 仓储 / 服务 / 缓存渲染器
    render/contract.rs  三层共用的方法契约
  src/generate.rs     串起解析 → 解析表 → 渲染 → 生成产物
  src/types.rs        列类型映射
  src/naming.rs       标识符与命名规则
  src/export.rs       写盘：先规划后写入，路径校验、冲突检测、软链接拒绝
src-tauri/            Tauri 命令（generate_project / export_project / pick_export_dir）
src/                  Vue 界面
  App.vue             装配 composable 与模板
  components/         SqlEditor、CodePreview
  composables/        设置持久化、生成、导出、复制的独立状态模块
  lib/                IPC 线格式与设置加载的纯函数
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
| `npm test` | 前端单测：IPC 错误收窄、设置加载、四个 composable（设置持久化、生成、导出、复制），以及 App.vue 的模板接线（挂载测试） |

`compile` 第一次运行会在临时项目里执行 `go mod tidy` 拉取 gorm 与 go-redis，因此需要能访问 module proxy。

CI 见 [.github/workflows/ci.yml](.github/workflows/ci.yml)，push 到 `main` 与 PR 时跑三个 job：`frontend`（typecheck + 单测 + 打包）、`core`（clippy + 全部 core 测试）、`tauri`（安装 webkit 依赖后跑 Tauri 单测与 clippy）。仓库托管在 [github.com/sword-demon/gorm-generate](https://github.com/sword-demon/gorm-generate)（public）。

## 打包

```bash
npm run tauri build                    # .app + .dmg
npm run tauri build -- --bundles app   # 只出 .app
```

产物默认在 `target/release/bundle/`（若设置了 `CARGO_TARGET_DIR`，则在那个目录下）。

- macOS aarch64 已验证:aarch64 `.app` 约 17 MiB,`.dmg` 约 6 MiB,图标取自 `src-tauri/icons/`。
- 需要换图标时:`npx tauri icon src-tauri/icons/icon.png`,再更新 `bundle.icon`。
- 产物**未签名、未公证**:只带 linker 的 adhoc 签名,分发给他人时 Gatekeeper 会拦截。
- 沙箱环境里 `.dmg` 打包会因 `hdiutil` 无法访问磁盘设备而失败,需在沙箱外执行。

## 文档

- [.scratch/gorm-generate/spec.md](.scratch/gorm-generate/spec.md) —— 需求、实现决策、测试策略
- [GLOSSARY.md](GLOSSARY.md) —— 领域用词
- [docs/adr/](docs/adr/) —— 架构决策记录
- [.scratch/gorm-generate/issues/](.scratch/gorm-generate/issues/) —— issue 与 ticket（本地 markdown tracker）
- [HANDOFF.md](HANDOFF.md) —— 上一次会话的交接说明

## 本地开发提示

- `cargo fmt --check` 目前在整仓库不通过，不是门禁。
- macOS 沙箱里 cargo 若报 `Operation not permitted`（target 目录或 sccache），用：

  ```bash
  RUSTC_WRAPPER= CARGO_TARGET_DIR="$TMPDIR/cargo-target" cargo test -p gorm-generate-core --lib
  ```

- `.cargo/config.toml` 是本机的稀疏镜像覆盖，已被 `.git/info/exclude` 排除，不属于仓库内容。
- 沙箱内 `--test compile` 会在 `goproxy.cn` 上遇到 TLS 校验失败（`x509: OSStatus -26276`），需要在沙箱外或联网环境运行。

## License

MIT，见 [LICENSE](LICENSE)。
