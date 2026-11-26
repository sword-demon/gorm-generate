# Spec: GORM 代码生成器 (SQL DDL → Go 分层代码)

Status: resolved

## Problem Statement

Go 开发者拿到建表语句后，需要手写模型、仓储、服务接口与实现，以及可选的缓存代码；字段多时既枯燥又容易抄错。现有工具不解决这个问题: gormt 必须连接数据库且只生成模型与查询函数；GORM 官方 CLI 以 Go struct 为输入，`gorm.io/gen` 以 DAO 为中心，都不产出仓储/服务分层，也不能离线粘贴 DDL 生成。

用户视角: 我有一段建表 SQL，想要一份能直接放进项目的 Go 代码（模型、仓储、服务接口与实现、可选 Redis 缓存），不想连数据库、不想手抄字段。

## Solution

一个纯本地的桌面应用（Tauri 2 + Vue 3 + TypeScript + Tailwind CSS 4 + shadcn-vue）。用户粘贴 MySQL 或 PostgreSQL 的 DDL 脚本，选择选项后生成 Go 代码，在文件树中预览、勾选需要的表，然后复制或按目录结构导出到本地项目。全部解析与生成在 Rust 侧离线完成。

## User Stories

1. 作为 Go 开发者，我希望直接粘贴 MySQL 建表语句就能生成代码，以便不用为了生成代码去连接数据库。
2. 作为 Go 开发者，我希望支持 PostgreSQL DDL（含 `COMMENT ON`），以便 PG 项目同样可用。
3. 作为 Go 开发者，我希望在界面里选择方言，以便同一段脚本按正确方言解析。
4. 作为 Go 开发者，我希望表名自动转换为单数 PascalCase 结构体名，以便生成符合 Go 惯例的模型。
5. 作为 Go 开发者，我希望每个模型都生成 `TableName()`，以便表名与结构体名解耦。
6. 作为 Go 开发者，我希望可空列生成指针类型，以便正确表达 NULL。
7. 作为 Go 开发者，我希望 `decimal`/`numeric` 生成 `string`，以便金额不失真。
8. 作为 Go 开发者，我希望 `tinyint(1)` 默认按整型映射，并可开关为 `bool`，以便既安全又灵活。
9. 作为 Go 开发者，我希望 `deleted_at` 自动映射为 `gorm.DeletedAt`，以便获得软删除语义。
10. 作为 Go 开发者，我希望表注释和列注释变成 Go 注释，以便模型自带文档。
11. 作为 Go 开发者，我希望生成仓储接口和 GORM 实现，以便业务层依赖接口而非实现。
12. 作为 Go 开发者，我希望生成服务接口和薄透传实现，以便直接填充业务逻辑。
13. 作为 Go 开发者，我希望缓存开关打开时生成 Redis 缓存装饰器，以便缓存不侵入仓储实现。
14. 作为 Go 开发者，我希望缓存 TTL 可配置，以便按业务调整。
15. 作为 Go 开发者，我希望缓存故障时降级查库，以便 Redis 抖动不影响业务。
16. 作为 Go 开发者，我希望复合主键生成多参数方法，以便关联表也能使用。
17. 作为 Go 开发者，我希望无主键表只生成模型并给出警告，以便不产出错误代码。
18. 作为 Go 开发者，我希望填写 Go module path 后 import 路径正确，以便生成物可直接编译。
19. 作为 Go 开发者，我希望看到按层分组的文件树，以便理解生成产物结构。
20. 作为 Go 开发者，我希望代码预览带语法高亮，以便快速检查生成结果。
21. 作为 Go 开发者，我希望支持单文件复制和一键复制全部，以便粘贴到编辑器。
22. 作为 Go 开发者，我希望选择目录后按目录结构写盘，以便直接放入项目。
23. 作为 Go 开发者，我希望导出时对同名文件先确认再覆盖，以便不误伤手改过的代码。
24. 作为 Go 开发者，我希望通过勾选表来过滤预览与导出，以便只取需要的表。
25. 作为 Go 开发者，我希望不支持的列类型以 `any` 加警告呈现，以便知道哪里需要手改。
26. 作为 Go 开发者，我希望解析错误定位到第几条语句并显示片段，以便快速修复 DDL。
27. 作为 Go 开发者，我希望上次的 SQL 与选项被记住，以便下次继续。
28. 作为 Go 开发者，我希望结果区提示需要 `go get` 的依赖，以便补齐 gorm 与 redis 依赖。
29. 作为 Go 开发者，我希望生成文件带 `DO NOT EDIT` 头，以便团队知道这是产物。
30. 作为 Go 开发者，我希望相同输入产生字节级相同的结果，以便入库时没有噪声 diff。
31. 作为 Go 开发者，我希望 MySQL 的 unsigned 宽度被保留，以便避免溢出。
32. 作为 Go 开发者，我希望 `SERIAL`/`BIGSERIAL`/`IDENTITY` 被识别为自增，以便 PG 主键行为正确。
33. 作为 Go 开发者，我希望非建表语句被忽略并计数，以便整段 dump 可以直接粘贴。
34. 作为 Go 开发者，我希望 `enum`/`json`/`blob` 等类型有合理默认映射，以便覆盖常见表结构。
35. 作为 Go 开发者，我希望生成的代码能通过 `go build`（由测试保障），以便拿到即可用。

## Implementation Decisions

- **架构**: 纯本地 Tauri 2 应用。Rust 核心库负责解析与生成，Tauri 命令是薄包装；Vue 前端只负责表单、预览与导出交互。遵循 ADR-0001（DDL 文本输入与自研分层生成）。
- **输入**: DDL 文本 + 方言枚举（MySQL | PostgreSQL）。只处理 `CREATE TABLE` 与 PG 的 `COMMENT ON`；其他语句忽略并计入警告。解析失败返回结构化错误（语句序号 + 片段）。应用每个方言带一份样例 DDL（ADR-0010）：方言变化时，只有当编辑器里的脚本仍是另一方言的样例原文时才换成新方言的样例，用户写过的脚本不动。
- **生成核心 API**: `generate(request) -> GeneratedProject`。请求含 DDL 文本、方言、module path、缓存开关、缓存 TTL、`tinyint(1)` 开关；结果含文件集合（相对路径 + 内容）、表清单、警告列表、依赖清单（由实际生成的文件推导，供 `go get` 提示）。纯函数、无 IO、同输入字节级同输出。
- **产物目录**: `internal/model`、`internal/repository`、`internal/service`、`internal/cache`（仅缓存开启时）。文件按表命名：`user.go`、`user_repo.go`、`user_service.go`、`user_cache.go`；接口与实现在同一文件。module path 决定 import 前缀。
- **模型规则**: 严格按 DDL，不嵌 `gorm.Model`，不添加 DDL 之外的字段；总是生成 `TableName()`；gorm tag 仅保留 `column`/`primaryKey`/`autoIncrement`/`not null`，生成列（`GENERATED ... AS (...)`）另加 `->`（只读）；JSON tag 使用列名原样；不生成外键关联字段；不生成 `index`/`uniqueIndex`/`default`/`type`/`size` tag（DDL 是索引与默认值的事实源）。
- **类型映射**: 可空列用指针（`[]byte` 与 `json.RawMessage` 例外：nil 已表达 null，不再取指针）；`decimal`/`numeric` → `string`；`json`/`jsonb` → `json.RawMessage`；`enum`/`set` → `string`；`blob`/`binary`/`bytea` → `[]byte`；时间类型 → `time.Time`；MySQL unsigned 按宽度映射 `uint8/16/32/64`；未知类型 → `any` + 警告。`tinyint(1)` 默认 `int8`/`uint8`，开关打开后为 `bool`。`deleted_at` → `gorm.DeletedAt`（软删除）。`SERIAL`/`BIGSERIAL`/`SMALLSERIAL`/`GENERATED ... AS IDENTITY` → `autoIncrement`。
- **仓储契约**: `Create(ctx, *T) error`；`GetByID(ctx, pk...) (*T, error)`；`List(ctx, page, pageSize) ([]*T, int64, error)`（按主键升序，返回总数）；`Update(ctx, *T) error`（全量 Save）；`Delete(ctx, pk...) error`。主键参数类型跟随 DDL；复合主键生成多参数；查不到原样返回 `gorm.ErrRecordNotFound`。无主键表不生成仓储/服务/缓存。
- **服务契约**: 与仓储完全同签名的薄透传；不生成 DTO；构造函数注入仓储接口。
- **缓存契约**: 遵循 ADR-0002。仅缓存 `GetByID`，key 为 `表名:主键值...`；`Update`/`Delete` 删除 key；`List`/`Create` 不处理缓存；使用 `go-redis/v9` + `encoding/json`；TTL 在生成时成为常量；任何缓存错误降级查库且不向上抛；生成缓存装饰器与 `NewXxxRepoWithCache(repo, rdb)` 工厂，TTL 以 `<Struct>CacheTTL` 包级常量固化在生成物中；开关关闭时不生成 cache 包。服务只依赖仓储接口，注入哪个实现由调用方决定。
- **表选择**: 生成始终覆盖全部表；界面上的表勾选只过滤预览与导出，不触发重新生成。
- **导出契约**: 目录由 Rust 侧的原生选择器确认后存进 managed state，渲染进程只触发选择、不传路径；写盘由 Rust 命令完成（`std::fs`），不引入 fs 插件；目标存在同名文件时先经用户确认，确认后覆盖。
- **界面形态**: 单页，左右两栏，中间一条可拖动的分隔条（也可用方向键调整，宽度会记住）。顶部工具栏只放标题、当前配置摘要、生成按钮与最右端的配置按钮；配置按钮打开表单，方言、module path、缓存开关、TTL、`tinyint(1)` 开关都在表单里改。左栏是可编辑的 SQL 输入（透明 textarea 叠在高亮背景上）；右栏自上而下是错误块或警告折叠块、文件计数与复制/导出按钮、表勾选行、文件列表与只读高亮预览并排、底部依赖提示（每条 `go get` 命令可一键复制，也能整组复制）；解析错误同时出现在可关闭的 toast 与内联错误块里；localStorage 记住输入、选项与分栏宽度。
- **生成代码约定**: 生成代码注释用英文；DDL 注释原样保留为 Go 注释；文件头包含 `Code generated ... DO NOT EDIT`。
- **版本基线**: Tauri 2.12.x、Tailwind CSS 4.3.x（CSS-first 配置）、shadcn-vue 2.9.x、sqlparser 0.64。

## Testing Decisions

- **好测试的定义**: 只测外部行为——给定 DDL 与选项，断言完整生成产物；不断言字符串拼接细节。已解析表（`resolve` 的输出）是 module interface，允许直接单测。
- **主 seam**: Rust `generate` 的 golden-file 测试。MySQL 与 PostgreSQL 各一份基线 DDL，另加针对性矩阵：类型映射、可空指针、注释（含 `COMMENT ON`）、复合主键、无主键、软删除、缓存开关、`tinyint(1)` 开关、警告列表、幂等（连续两次生成结果一致）。
- **次 seam**: `export_project` 用临时目录验证目录结构、文件内容与同名覆盖行为。
- **前端 seam**: 四个 composable（设置持久化、生成、导出、复制）各有单测，`App.vue` 另有挂载测试覆盖模板接线（`v-model`、按钮禁用态、表勾选、toast 与内联错误）；两个 Shiki 编辑器在挂载测试里用 stub 替换。Tauri IPC 本身不测（命令是薄包装）。
- **仍靠手动冒烟**: 导出写盘链路（原生选择器 → 写盘 → 冲突确认）与打包产物，发布前手动走一遍。
- **先例**: 无，全新仓库。

## Out of Scope

- 连接数据库或内省 schema；SQLite、MSSQL 及其他方言。
- `gorm.io/gen` 风格 DAO、DTO/分页响应包装、事务模板、关联预加载代码。
- 缓存 `List`、负缓存、写穿透、进程内缓存。
- 用户自定义模板、在线编辑生成代码、生成 `go.mod`、多项目管理、国际化、打包签名与自动更新。
- MySQL 表级 `PARTITION BY`：不在支持范围；若导致整条语句解析失败，按解析错误处理。

## Further Notes

- 参考: GORM 文档、`gorm.io/cli`、gormt；设计决策见 `docs/adr/0001`、`docs/adr/0002` 与 `GLOSSARY.md`。
- sqlparser 注意点: `AUTO_INCREMENT` 以 DialectSpecific token 形式出现，需特判；SQL 文本注释会被丢弃；表级 `PARTITION BY` 不被解析。
- 生成产物依赖: `gorm.io/gorm`；缓存开启时另需 `github.com/redis/go-redis/v9`。界面结果区展示对应 `go get` 提示。
- 错误分类: `generate` 返回 `GenerateError`，判别式是类型而非哨兵值。`Parse { statement_index, snippet }` 只用于 DDL 解析失败（语句级解析错误与分词错误：未闭合字符串/注释/引号标识符），携带 1 起算的真实语句序号、出错片段（最多 500 字符）与出错行；`Validation(String)` 用于 module path、标识符、命名冲突与主键等语义校验错误；`DialectMismatch { statement_index, message, snippet }` 用于脚本在请求的方言下解析失败、在另一个方言下能解析的情形（ADR-0010），`message` 点名两个方言并给出改法。前端按 `kind`（`parse` | `validation` | `dialectMismatch`）分别渲染，不再解释序号 0。
- 方言丢弃的语义不静默: sqlparser 只在 MySQL 方言下保留 `AUTO_INCREMENT` 列选项，PG 方言下该 token 被吞掉。core 在分词阶段发现这种情况，产出脚本级警告（主键不会被标记为自增），而不是给出一份与 DDL 不符的模型。
