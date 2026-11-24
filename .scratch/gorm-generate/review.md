# gorm-generate 代码 review 报告（r3 修订版）

> r2 按 reviewer 的 6 条评审意见做了修订；r3 并入第 4 个专项 reviewer（rev-io）的返回并按 reviewer 的 4 条补充意见再修订，逐条对照见 §0.3。**未改动任何生产代码，未提交、未发布。**

审查范围：整个仓库（Rust 核心 `crates/core`、Tauri IPC `src-tauri/src`、Vue 前端 `src/`、构建与工程化配置）。
方法：4 个专项 reviewer 并行覆盖（**4 个全部返回**；第 4 个 rev-io 负责的 `export.rs` / IPC / 命名范围，我本人另做了逐行复核 + 实测复核，并把它返回的 3 条新发现并入本报告）+ 我对每条高危结论的独立复核（重新跑生成 → `go build` / 运行时实证）。

结论：**生成器对「教科书式 DDL」是通的，但对真实 DDL 的容错面很窄**。现有测试矩阵只覆盖「单列 `id BIGINT` 主键、列名无冲突、无保留字」的理想输入，因此下面绝大多数缺陷都能躲过现有测试矩阵（本仓库**没有 CI**，见 C2，所以不存在「逃过 CI」这一说法）。**30 个报告条目归入 13 个修复主题**（主题之间可重叠，是修复归类而不是互斥的根因划分，见 §0.2），其中 **4 条会让 `go build` 直接失败**（A1 / A3 / A8 / A11），**7 条编译通过但运行时行为错误**（A2 / A4 / A9 / A10 / A12 / A16 / A17）。r1 的 handoff 标题把 A2 算进了「go build 失败级」，本轮已实测更正（A2 的产物 `go build` exit 0）；r3 并入 rev-io 返回的 3 条新发现（A16 / A17 / A18）。

级别分布（30 条缺陷，B7 是「已核实无问题」清单，不计入）：**高 7 / 中高 1 / 中 14 / 中低 1 / 低 6 / 低-中 1**。

---

## 0. 交付证据

### 0.1 冻结状态

| 项 | 值 |
|---|---|
| HEAD SHA | `ec013e34123ae9bf0bc5471ecba8172028a620d2` |
| HEAD commit | `ec013e3 refactor: bundle matrix case into Case and clean temp dirs on drop`（分支 `main`） |
| `cargo test` | **exit 0** —— 39 passed / 4 suites（37 + 2 + 0 + 0，0 failed） |
| `npm run typecheck`（`vue-tsc --noEmit`） | **exit 0** —— 无输出 |
| 依赖版本（`Cargo.lock` 锁定） | tauri 2.12.7 / tauri-macros 2.7.6 / tauri-runtime-wry 2.12.5 / wry 0.57.3 / sqlparser 0.64.0 |
| 外部工具链 | Go 1.27.3、gorm.io/gorm v1.31.2、glebarez/sqlite v1.11.0、Xcode WebKit（macOS 27.2.1） |

`git status --short`：

```
 M .scratch/gorm-generate/spec.md
 M AGENTS.md
 M docs/agents/issue-tracker.md
 M docs/agents/triage-labels.md
?? .out-of-scope/
?? .scratch/gorm-generate/review.md
?? tasks/
```

- 4 个 `M` 是**上一轮已获批准的文档漂移修正**（triage 任务），与本次 review 无关，本次 review 未改任何生产代码。
- 未跟踪文件有三项：`.out-of-scope/.gitkeep`（上一轮新建的空目录占位，内容为空）、`.scratch/gorm-generate/review.md`（本报告的仓库副本）、`tasks/lessons.md`（本轮按项目规则追加的教训日志，见 §0.1 末条）。
- **没有任何未跟踪的源文件**。本轮实证用的临时探针（`crates/core/tests/tmp_review_probe.rs`、`tmp_verify_review.rs`、`tmp_verify_pg.rs`、`tmp_ab_a2.rs`、`tmp_rev_io_verify.rs`、`zz_probe.rs`、`zz_parser_review*.rs`）已全部删除；新增的实证工具都在仓库之外：`/tmp/wk_confirm_probe.swift`（B8）、`/tmp/genbench/`（B9 耗时测量，path 依赖 `crates/core`，构建产物落在仓库已 gitignore 的 `target/`）。
- **本报告涵盖的未跟踪文件**：`.scratch/gorm-generate/review.md`（本文件自身）。`.out-of-scope/.gitkeep` 为空文件，不涉及任何缺陷；`tasks/lessons.md` 是本轮 review 的教训记录（记录 r1 被指出的 4 处「结论超出证据」问题），与产品代码无关。

### 0.2 修复主题归并：30 个报告条目 → 13 个修复主题

30 是**报告条目数**。下表把条目归并到 13 个**修复主题**并标出修复位置 —— 这张表是**修复归类**，不是互斥的根因划分：同一主题里的条目可能成因不同（R1 就混了「与固定方法撞名」「非法字符」「类型名撞车」「列名重复」「批内路径重复」五种成因），不同主题之间也可能共享同一处改动（A9 与 A14 是同一现象在生成层与导出层的两个防御点，已在表内合并标注）。因此不要把 R1–R13 当成 13 个独立成因来排期。

| 修复主题 | 条目 | 修复位置 |
|---|---|---|
| R1 标识符未合法化 / 未消歧 | A1, A8, A9, A11, A14 | 生成前校验层（`naming.rs` + `generate.rs`） |
| R2 主键标识符比较未按方言与引号规则 | A2 | 解析层归一化 + `layers.rs` / `generate.rs` 两处口径统一 |
| R3 类型信息在层间丢失 | A4, A5 | 类型映射单一来源（`types.rs` → 模型/仓储/缓存） |
| R4 生成物 import 与类型映射不联动 | A3 | 生成器的 import 收集 |
| R5 方言特性未覆盖 | A6, A10 | 解析层（PG 注释）+ 生成层（标识符引用） |
| R6 生成列 / 自增语义判定 | A7 | 解析层 `is_identity` |
| R7 输入未校验 | A12, A17, B3 | 前端 + 生成入口（分页参数、TTL 下界、module path） |
| R8 导出层防御不足（路径解析 / 原子性 / 批内冲突 / 错误信息） | A14, A15, A16, A18 | `export.rs` |
| R9 前后端 IPC 契约不一致 | B1, B2 | DTO 命名 + 前端取值 |
| R10 假设宿主提供浏览器原生对话框 | B8 | 前端改用 dialog 插件 |
| R11 命令执行线程模型 | B9 | `src-tauri/src/main.rs` 命令声明 |
| R12 前端健壮性 / 错误反馈缺失 | B4, B5, B6 | 前端 |
| R13 工程化与文档 | A13, C1, C2, C3, C4 | 仓库配置 |

### 0.3 逐轮修订对照

> 下表 ①–⑧ 是**历史记录**，用的是当轮的措辞（例如当时写「13 个根因」）；**当前口径以 §0.2 为准** —— 13 个**修复主题**（可重叠，非互斥根因）。本轮 ⑨ 记录的是按 reviewer 意见对当前文本做的修订。

| reviewer 意见 | r2 的处理 |
|---|---|
| ① 冻结 HEAD / git status / 测试命令与退出码 / 说明报告涵盖的未跟踪文件 | 补 §0.1，全部为实测值 |
| ② C1 因果不成立：本机全局镜像问题不能推出新克隆或 CI 依赖 rsproxy | **撤回**「构建必需」结论与「新克隆/CI 会卡死」的推断；C1 改写为「本机本地、未共享、未文档化的镜像绕过」，级别由 [中] 降为 [低]，并明确它与 C2/C4 才是仓库层真问题（见 C1） |
| ③ A9/A14 是同一问题的两层防御点；27 是条目数不是根因数 | 新增 §0.2 归并表（13 个根因）；A9/A14 改为同一根因条目，两处修复点分别写明（见 A9、A14） |
| ④ B8 保留「源码推断 vs 实跑」边界，不写已实证恒 false | B8 改为**已实证的 WebKit 层行为**（独立 WKWebView A/B 对照实验）+ **仍属源码推定的应用层结论**，并显式标注未验证边界（见 B8、F 段） |
| ⑤ B9 需主线程路径证据；无性能测量不得写已发生冻结 | B9 补 6 跳主线程路径取证 + **命令体耗时实测表**；撤回「用户观感就是卡死了」，降级为 [低-中]（见 B9） |
| ⑥ A2 按方言与引号规则区分标识符匹配，不能统一大小写不敏感；A3 应补 import 而非归入命名校验 | A2 改写为方言 × 引号规则的比较规则并给出可实现方案；A3 的根因改为「import 与类型映射不联动」，与命名校验脱钩（见 A2、A3） |
| ⑦ r2 自查补充：r1 handoff 标题里的「5 条 go build 失败级」含 A2，实测不成立 | 实测 A2 产物 `go build ./...` **exit 0**（Go 允许参数列表结尾逗号）→ A2 改列「编译通过但运行时行为错误」，失败级改为 4 条（A1/A3/A8/A11）（见 A2 补充实证、§0.3 结论段） |
| ⑧ r3：第 4 个专项 reviewer（rev-io，覆盖 `export.rs` / IPC / 命名）返回 | 其 9 条里 5 条与本报告既有条目重合（A14 / B1 / B2 / B3 / B9），已吸收其补强证据；**3 条为新发现并入**：A16（软链接未解析 → 写到所选目录之外 / 可覆盖目录外文件）、A17（TTL=0 无下界 → Redis key 永不过期）、A18（I/O 错误信息不含路径 + 前端截断无提示）；A15 按其证据**升级为 [中]**并补入「O_TRUNC 先抹掉原文件内容」的数据丢失事实。条目数 27 → 30，仍归入 13 个修复主题（三条新发现分别归入 R8 与 R7）。rev-io 自身标注「未验证」的 `window.confirm` 一条，本报告已用独立 WKWebView A/B 对照实验实证（见 B8），两者不冲突 |
| ⑨ r3 复核意见：① A16 的 canonicalize / 元数据检查只反映检查时状态，不能宣称消除检查与写入间的竞态；修复验收须明确软链接策略与平台边界 ② A15 的同目录临时文件 + 原子替换只提供**单文件**保证，整批依次 rename 仍非事务；失败清理不得删除原有文件，内存备份回写也不保证故障时恢复 ③ A17 的零 TTL 语义成立，但「永久缓存」本身并不矛盾，「0 = 立即过期」是未证实的推测，应表述为「0 的语义未明确约定 + 需校验」；与 A4 叠加导致旧值长期保留需端到端验证 ④ 报告标题仍是 r2、冻结状态缺 `tasks/`、30 条应归 13 个修复主题而非互斥根因、本仓库无 CI 不能写「逃过 CI」 | 标题改为 r3 修订版；§0.1 的 `git status` 补上 `?? tasks/`；§0.2 改称「13 个修复主题（可重叠，非互斥根因）」，§0.2 引言显式说明 R1 内含五种不同成因；结论段把「逃过 CI」改为「躲过现有测试矩阵（本仓库没有 CI，见 C2）」；A16 修法补 TOCTOU 边界与平台边界要求；A15 修法补单文件 vs 整批事务的边界与「不得删原文件 / 内存备份不保证恢复」；A17 改写为「0 的语义未明确约定 + 需校验」，删去对用户意图的未证实假设，并补与 A4 叠加的端到端验证要求；E 段同步口径 |

---

## A. 核心生成器（crates/core）

### A1 [高] 列名 `table_name` 生成的字段与模型固定方法 `TableName()` 同名 → 编译失败

- 位置：`crates/core/src/generate.rs:200`（模板无条件生成 `func ({recv} {Struct}) TableName() string`）、`crates/core/src/generate.rs:162`（字段名 = `field_name(&col.name.value)`）、`crates/core/src/naming.rs:10-12`（`pascal_case("table_name")` = `TableName`）
- 触发：`CREATE TABLE logs (id BIGINT NOT NULL, table_name VARCHAR(32) NOT NULL, PRIMARY KEY (id));`
- 现象（我已实测）：模型同时有字段 `TableName string` 和方法 `func (l Log) TableName() string`
  ```
  internal/model/log.go:10:14: field and method with the same name TableName
  internal/model/log.go:7:2: other declaration of TableName
  ```
- 根因：生成前没有「字段名 vs 保留方法名」的冲突校验。`table_name` 在日志/审计表里极常见，是高频踩雷项；`tableName`、`TABLE_NAME` 同样触发。

### A2 [高] 主键列名与列定义匹配失败 → 主键被静默丢弃 → GetByID 无参数、Delete 必失败、缓存 key 常量化

- 位置：`crates/core/src/layers.rs:19`（`table.columns.iter().find(|c| c.name.value == *pk)?` —— 按原始字符串精确比较，且 `?` 静默丢弃）、`crates/core/src/generate.rs:69`（只检查 `table.primary_key.is_empty()`，不看解析出的主键列数）、`crates/core/src/generate.rs:118`（`is_pk` 同样按精确字符串比较）
- 触发：`CREATE TABLE t (id BIGINT NOT NULL, PRIMARY KEY (ID));` —— 在 **MySQL 与 PostgreSQL 上都合法**（MySQL 列名本身大小写不敏感；PG 未加引号的 `ID` 会被折叠成 `id`）
- 现象（我已实测）：
  ```go
  func (r *tRepository) GetByID(ctx context.Context, ) (*model.T, error) {
      err := r.db.WithContext(ctx).Where("", ).First(&m).Error   // GORM 里 Where("") 不产生条件
  ```
  运行时实测：`GetByID` 退化成「取表中第一行」（`SELECT * FROM t ORDER BY t.id LIMIT 1`，与调用方想要的 PK 无关）；`Delete` 被 GORM 安全检查拦下 → `WHERE conditions required`，一条都删不掉；缓存 key 变成常量 `"t:"`，所有 `GetByID` 共用一条缓存项。
  另外模型字段的 gorm tag 也会丢 `primaryKey`：`Id int64 \`gorm:"column:id;not null" json:"id"\``。
- **补充实证（本轮新做）**：这一份产物 **能通过 `go build ./...`（exit 0）** —— Go 的参数列表允许结尾逗号（`ParameterList [ "," ]`），所以 `GetByID(ctx context.Context, )`、`Where("", )`、`fmt.Sprintf("t:", )` 全是合法语法。**因此 A2 属于「编译通过、运行时行为错误」，不是编译失败类**（r1 的 handoff 标题把它算进「go build 失败级」，此处更正）。
- 根因：**标识符等价性被简化成「字符串完全相等」**。SQL 里标识符是否等价由「方言 + 引号风格」共同决定，不是一律大小写敏感、也不是一律不敏感：

  | 方言 | 未加引号 | 加引号（`"x"` / `` `x` ``） |
  |---|---|---|
  | PostgreSQL | 折叠为小写 → `ID` ≡ `id` | 精确匹配 → `"ID"` ≠ `id` |
  | MySQL（列名） | 列名一律不敏感 | 反引号列名同样不敏感（MySQL 无「大小写敏感的带引号标识符」） |
  | MySQL（表名） | 取决于 `lower_case_table_names`（Linux 默认区分大小写） | 同左 |

  本仓库用的 sqlparser 0.64 在 `Ident` 上保留了 `quote_style: Option<char>`（`sqlparser-0.64.0/src/ast/mod.rs:267-275`），**列名**（`ColumnDef.name`）与**主键约束里的列名**（`TableConstraint::PrimaryKey { columns: Vec<IndexedColumn> }` → `TableFactor::Column(ObjectName)`）两边都带这个信息 → 可以按方言 × 引号算出「比较键」，而不是无条件 `to_lowercase()`。
- 修法（r2 修正后）：
  1. 在解析层为每个标识符算一个**归一化比较键**：PG 未加引号 → `to_lowercase()`；PG 加引号 → 原样；MySQL 列名 → 一律忽略大小写；MySQL 表名 → 按 `lower_case_table_names` 语义处理。
  2. 主键约束里的标识符与列定义用**同一把尺子**比较（当前 `layers.rs:19` 与 `generate.rs:118` 两处口径还不一致）。
  3. 匹配失败必须产出 warning，不允许 `?` 静默丢弃；`crates/core/src/generate.rs:69` 应依据「解析出的主键列数」而非「原始约束非空」决定是否生成 repo/service/cache。
- 同源触发点：`PRIMARY KEY (id(10))`（前缀索引）→ `crates/core/src/parser.rs:196` 的 `_ => String::new()` → 主键名变成空串 → 同样丢失。

### A3 [高] DATETIME/TIMESTAMP 主键 → 仓储与服务用了 `time.Time` 但没 import `"time"` → 编译失败

- 位置：`crates/core/src/layers.rs:49-55`（仓储 import 块写死 `"context"` / `"gorm.io/gorm"` / model）、`crates/core/src/layers.rs:143-148`（服务 import 块同理写死），类型来自 `crates/core/src/layers.rs:22` 的 `map_type`
- 触发：`CREATE TABLE soft (deleted_at DATETIME NOT NULL, PRIMARY KEY (deleted_at));`
- 现象（我已实测）：
  ```
  internal/repository/soft_repo.go:16:41: undefined: time
  internal/repository/soft_repo.go:19:40: undefined: time
  internal/repository/soft_repo.go:36:65: undefined: time
  internal/repository/soft_repo.go:60:64: undefined: time
  ```
  服务包因依赖仓储包未通过而未被类型检查，它的 `time.Time` 同样缺 import，修好仓储后必炸。
- 根因（r2 修正）：**生成物的 import 块与类型映射不联动** —— 模型侧有 `needs_time_import` 机制（`crates/core/src/generate.rs:146-148`），仓储/服务侧没有，import 是写死的固定列表。这与命名/标识符校验（A1/A8/A11）**无关**，是生成器自洽性的另一类问题。
- 修法：类型映射阶段产出「该文件需要哪些 import」的集合（`time`、`fmt`、`context` 等），由生成器统一写入 import 块，而不是在各模板里写死常量列表。

### A4 [高] `tinyint(1)` 主键 + 「tinyint(1) 当 bool」开关 → 模型与仓储类型不一致 + 缓存 key 读写不对称 → 更新后读脏数据

- 位置：`crates/core/src/layers.rs:21-22`（`map_type(&base, width, unsigned, false, false)` —— 最后两个参数写死，`tinyint_as_bool` 被丢弃）vs `crates/core/src/generate.rs:121`（模型侧用 `request.tinyint_as_bool`）
- 触发：`CREATE TABLE flags (id tinyint(1) NOT NULL, label varchar(64) NOT NULL, PRIMARY KEY (id));` + 开关「tinyint(1) 当 bool」= 开 + 缓存 = 开
- 现象（我已实测生成物）：
  ```go
  // 模型
  Id bool `gorm:"column:id;primaryKey;not null" json:"id"`
  // 仓储 / 服务
  func (r *flagsRepository) GetByID(ctx context.Context, id int8) (*model.Flags, error)
  func (r *flagsRepository) Delete(ctx context.Context, id int8) error
  // 缓存
  key := fmt.Sprintf("flags:%v", id)    // GetByID: int8  -> "flags:1"
  key := fmt.Sprintf("flags:%v", m.Id)  // Update:  bool -> "flags:true"
  key := fmt.Sprintf("flags:%v", id)    // Delete:  int8  -> "flags:1"
  ```
- 后果 1：`GetByID` 写入的键是 `flags:1`，`Update` 去删 `flags:true` → **更新后缓存永远不被淘汰，读到的全是脏数据**。
- 后果 2：模型主键是 `bool`、仓储 API 收 `int8`，类型贯通断了，用户代码没法把 `m.Id` 传给 `GetByID`。
- 复合主键同理：`TenantId bool` vs `GetByID(ctx, tenantId int8, itemNo int32)`，缓存 key `items:%v:%v` 两侧取值类型不同。
- 同根因第二种表现：`crates/core/src/generate.rs:142-145` 把 `deleted_at` 强制映射为 `gorm.DeletedAt`，但 `layers.rs:22` 的主键类型仍是 `time.Time` → 缓存 key 读侧 `time.Time`、Update 侧 `gorm.DeletedAt`，渲染结果不同。

### A5 [中高] PostgreSQL 的 `int8/int4/int2/float8/float4` 全部退化成 `any`

- 位置：`crates/core/src/parser.rs:274` 的兜底 `format!("{other:?}")` → 产出 `"int8(none)"` → `crates/core/src/types.rs:47-52` 未知分支 → `any` + warning
- 现象（我已实测）：PG 的 `bigint`→int64、`integer`→int32、`double precision`→*float64 正常，但 `int8`/`int4`/`int2`/`float8`/`float4` 全部 `any`，警告原文：
  ```
  t: unsupported column type `int8(none)`, mapped to `any`
  ```
- 影响：PG 用户的 DDL 里几乎必然出现 `int8`（= `bigint` 的别名），生成物直接退化为 `interface{}`，等于没有类型。
- 同一个兜底还吞掉 MySQL `INT(11) ZEROFILL`（应为 uint32）、`BIT(1)`、PG `TEXT[]`。

### A6 [中] pg_dump 风格的 `COMMENT ON COLUMN public.users.name` → 列注释整条丢失

- 位置：`crates/core/src/parser.rs:106` / `:110-115` 的 `apply_comment` 用 `object_name.0.first()` 当表名、`get(1)` 当列名；而同文件 `ident_of`（`crates/core/src/parser.rs:170-175`）用 `.last()`
- 现象（我已实测）：`COMMENT ON COLUMN public.users.name IS 'the name'` → 表名取到 `"public"` → 注释被丢弃，模型里没有 `// the name`
- 影响：pg_dump 是 PG 最常见的导出方式，它的 `COMMENT ON COLUMN` 一律带 schema 前缀 → 注释功能在 PG 上等于不存在。
- 附带：`crates/core/src/parser.rs:117-123` 要求 COMMENT ON 必须出现在 CREATE TABLE 之后，顺序换了就丢。

### A7 [中] 表达式生成列被误判成自增，或完全不被标记 → GORM 会 INSERT 生成列 → MySQL 3105

- 位置：`crates/core/src/parser.rs:279-286` 的 `is_identity` 用 `ColumnOption::Generated { generated_keyword: true, .. }` 判定
- 现象（我已实测）：`total int GENERATED ALWAYS AS (id + 1) STORED` → `Total *int32 \`gorm:"column:total;autoIncrement"\``（`GENERATED ALWAYS` 关键字形式被误标成自增）；而简写 `AS (expr)`（`generated_keyword: false`）不被标记 → GORM 会把该列写进 INSERT → MySQL 报错 3105（列不允许被赋值）。

### A8 [中] 非法标识符字符原样进入 Go 源码 → 编译失败（且无任何校验或转义）

- 位置：`crates/core/src/generate.rs:161-165`（字段名与 `json` tag 用原始列名）、`crates/core/src/generate.rs:200`（表名直接放进 `return "..."`）、`crates/core/src/naming.rs:39-50`（`pascal_case` 不过滤非法字符）
- 触发：列名 `we"ird` → `We"ird *string \`gorm:"column:we"ird" json:"we"ird"\`` → `syntax error: unexpected name column in struct type` + `newline in string`；表名 `we"ird` → 文件名就是 `internal/model/we"ird.go` → `syntax error: unexpected literal "ird struct { in type declaration`
- 附带：表名里的 `%` 会同时污染结构体名（`type Pct%t struct` → `syntax error: unexpected % in type declaration`）和缓存 key 的 fmt 格式串（`fmt.Sprintf("pct%t:%v", id)` → 渲染成 `pct%!t(int64=1):%!v(MISSING)`）。
- 说明：注释里的用户文本是安全的 —— `crates/core/src/generate.rs:158` 与 `:196` 把 `\n`/`\r` 压成空格，用户文本无法逃出 `//` 行注释。

### A9 [中] 表名复数化后撞名（`user` 与 `users`）→ 文件互相覆盖 → 整张表静默丢失

- 位置：`crates/core/src/naming.rs:4-7`（`struct_name = pascal_case(singularize(table))`）+ `crates/core/src/naming.rs:30-36`（`singularize` 只砍一个尾 `s`）、路径来源 `crates/core/src/generate.rs:211-214`、`crates/core/src/layers.rs:121-124/202/343`、`crates/core/src/export.rs:40-72`（逐个写文件，不检测重复路径）
- 触发：同一份 DDL 里有 `CREATE TABLE user (...)` 和 `CREATE TABLE users (...)`
- 现象（我已实测）：两表都 → `struct_name = "User"` → 同一批路径（`internal/model/user.go`、`internal/repository/user_repo.go`、`internal/service/user_service.go`、`internal/cache/user_cache.go`）；`internal/model/user.go` 最终内容是 **`users` 表**的，`user` 表的 model/repo/service/cache 全部消失；**`generate()` 不产生任何 warning**，`go build` 通过 → 静默数据丢失。
- 同类可触发：`order`/`orders`、`product`/`products`、`customer`/`customers`、`user`/`User`。
- **与 A14 是同一现象（R1 修复主题）的两个防御点**：本条是**生成层**（名字没消歧，同一路径被生成两次），A14 是**导出层**（同批重复路径没被检测）。修 A9 不修 A14，一旦以后引入新的路径重复来源（比如文件名前缀、手写子目录）仍会静默覆盖；修 A14 不修 A9，用户会看到「导出冲突」而不是「表名撞了」。两层都要修，但它们**不是两个独立缺陷**。

### A10 [中] 保留字列名未加引号 → 运行时 SQL 语法错误

- 位置：`crates/core/src/layers.rs:381-386` 的 `where_chain` 用原始列名拼 `"{col} = ?"`；`crates/core/src/layers.rs:114` + `:96` 的 `Order("{order}")` 同样原样拼接
- 触发：`CREATE TABLE t (order BIGINT NOT NULL, x INT NULL, PRIMARY KEY (order));`
- 现象：`go build` 通过，但运行时 MySQL 报语法错误（`ORDER` 是保留字）：
  ```go
  err := r.db.WithContext(ctx).Where("order = ?", order).First(&m).Error
  err := r.db.WithContext(ctx).Order("order").Offset(offset).Limit(pageSize).Find(&items).Error
  ```
- 修复方向：原始列名进 SQL 字符串时按方言加反引号/双引号（MySQL `` `order` ``、PG `"order"`）。

### A11 [中] 列名经 PascalCase 后撞名 / 列名重复 → 模型重复字段 → 编译失败；复合主键时 WHERE 还把同一参数绑两次

- 位置：`crates/core/src/naming.rs:10-12`（`field_name` 只做 `pascal_case`，无去重/冲突检测）、`crates/core/src/generate.rs:160-165`、`crates/core/src/layers.rs:208-214`（`params_list` 用 `param_name`）、`crates/core/src/layers.rs:381-386` + `:221-223`（`where_chain`/`where_args` 用同名变量）
- 触发：`CREATE TABLE t (user_id BIGINT NOT NULL, userId BIGINT NOT NULL, PRIMARY KEY (user_id, userId));` 或重复列名 `CREATE TABLE t (id BIGINT NOT NULL, id BIGINT NOT NULL, PRIMARY KEY (id));`
- 现象（我已实测）：
  ```
  internal/model/t.go:7:2: UserId redeclared
  internal/model/t.go:6:2: other declaration of UserId
  ```
  即便能编译，两个 `?` 会绑到同一个 Go 变量 → `WHERE "user_id = ? AND userId = ?"` 两个条件都取第一列的值。

### A12 [低] 分页参数非法时静默返回第一页

- 位置：`crates/core/src/layers.rs:95` `offset := (page - 1) * pageSize`
- 现象：`page <= 0` 时 offset 为负，GORM v1.31.2 的 `clause/limit.go` 里 `if limit.Offset > 0` 会直接跳过 OFFSET → `List(ctx, 0, 10)` 静默返回第一页而不是报错。

### A13 [低] 命名与注释的文档漂移

- `crates/core/src/layers.rs:388` 注释写 `user_id -> userID`，实现（`crates/core/src/layers.rs:389-403`）只把首字母小写 → 实际产出 `userId`（实测 `tenantId`/`itemNo`）。
- `crates/core/src/naming.rs:30-36` 的 `singularize` 英文不准：`categories`→`Categorie`、`addresses`→`Addresse`、`statuses`→`Statuse`（不影响编译，但生成的类型名不自然）。

### A14 [中] 同一次导出内的重复路径不被检测 → 静默互相覆盖（导出层防御点，与 A9 同一根因）

- 位置：`crates/core/src/export.rs:52-63` —— 冲突检查统一在写入前做 `path.exists()`
- 现象：**同一批文件里的重复路径**永远查不出来（检查发生在任何写入之前，两个目标路径此刻都不存在）→ 后写的直接覆盖先写的。这正是 A9（`user`/`users` 撞名）在落盘阶段的放大器：`internal/model/user.go` 被 `users` 表的内容覆盖，而界面照常显示「导出成功」。
- **界面还会报一个假的成功数字**（rev-io 补强）：DDL 里同时有 `user` 和 `users` 两张表时生成 8 个文件、但只有 4 个不同路径，`src/App.vue:218` 的提示用的是 `files.length` → 用户看到「Exported 8 file(s) to &lt;dir&gt;」，磁盘上实际只有 4 个文件；`src/App.vue:390` 的 `:key="f.path"` 还会出现重复 key。
- 定位：**这是「路径撞名」在导出层的防御点，与 A9 属同一修复主题（R1），不是独立缺陷** —— 详见 A9 的说明与 §0.2 归并表。
- 修复方向：`export_project` 增加批内重复路径检测并报冲突（与 A9 的类型名消歧配套，两层互补）。

### A15 [中] 覆盖写不是原子的 → 写失败时**原文件内容已被截断抹掉**（数据丢失）+ 留下半成品目录

- 位置：`crates/core/src/export.rs:69`（`std::fs::write` = `File::create`（O_TRUNC）+ `write_all`，无 temp+rename、无回滚）、`crates/core/src/export.rs:65-70`（逐个写，中途失败不回滚）
- 两条后果：
  1. **原文件内容先被清空**：`fs::write` 的 O_TRUNC 发生在写入之前，一旦写入失败（磁盘满 / 配额超限 / EIO），目标文件里**原来的内容已经没了**，只剩半截新内容。rev-io 实测（`RLIMIT_FSIZE=8192`、写 100000 字节）：`write_all` 返回 `File too large (os error 27)`，目标文件残留 8192 字节，原来的 `// ORIGINAL USER CONTENT` 已被抹掉。
  2. **半成品目录**：同一批里先写成功的文件不回滚，目标目录处于半新半旧状态，`go build` 失败。
- 文档注释 `crates/core/src/export.rs:45-46` 写的 "Validate all paths before touching the filesystem so a bad path never leaves a half-written tree" 只对**路径校验**成立，对 I/O 失败不成立，容易被误读为原子性保证。
- 修法与它的边界（**不能按「整批原子」验收**）：
  - **单文件层面**：写同目录临时文件（`.tmp`）+ `fs::rename` 原子替换，可保证「旧文件完整」或「新文件完整」二选一，单个文件不会留半截。
  - **整批不是事务**：整批依次 rename 只能保证**每个文件各自**原子；整批中途失败时目录处于混合状态（部分新、部分旧、部分半成品），没有任何回滚语义。
  - **失败清理不得删原有文件**：清理只能删「本次新建的临时文件」和「本次新建的目标文件」；对**导出前就已存在**的同名目标文件（用户确认过要覆盖的那些），绝不能在失败路径上删除 —— 删了等于把「原文件完好」变成「原文件丢失 + 目录半成品」。
  - **内存备份回写不保证恢复**：把旧内容读进内存、失败时写回只是尽力而为 —— 回写本身也会失败（磁盘满 / 配额超限正是触发条件），因此**不能**宣称「失败时可恢复原内容」。
  - 若需要整批级保证，只能走「先写到全新临时目录，整批成功后整体替换目标目录」这类目录级替换（目录 rename 在跨卷等场景同样会失败，仍非严格事务），且会改变现有覆盖语义 —— 属产品决策，需维护者拍板。

### A16 [高] 导出路径不解析软链接 → 文件写到所选目录**之外**，并可能覆盖目录外的无关文件

- 位置：`crates/core/src/export.rs:76-85`（`safe_join` 只做**词法**组件检查：`Component::Normal` 之外一律拒绝，但**不做 canonicalize、不判定软链接**）、`crates/core/src/export.rs:57`（`path.exists()` 跟随软链接）、`crates/core/src/export.rs:69`（`std::fs::write` 跟随软链接）
- **我已实测复现三种形态**（探针在仓库外，临时文件已删）：
  - **A. 目录组件是软链接**：`dest/internal` → 指向 `dest` 之外的目录 → 4 个文件（model / repository / service / cache）**全部落在目标目录之外**，`export_project` 返回 `Ok(())`，UI 照常提示导出成功；用户以为写进了自己选的目录。
  - **B. 文件本身是软链接**：`internal/model/order.go` → 指向目录外的 `victim.go` → `exists()` 跟随软链接返回 true，冲突确认框里显示的却只是**项目内相对路径** `internal/model/order.go`，用户点确认后**目录外的 `victim.go` 被生成代码覆盖**。
  - **C. 悬空软链接**：`internal/model/order.go` → 指向不存在的 `victim.go` → `exists()` 返回 false → **冲突检测完全失效**（连确认框都不弹），`fs::write` 跟随软链接直接在目录外创建出 `victim.go`。
- 后果：**写到非预期位置 / 破坏用户所选目录之外的文件**，C 情况下用户完全不知情。这与 `spec.md` 的「导出到用户选择的目录」契约冲突，也与 `export.rs:3-4` 自己的注释「Anything that escapes the target directory (absolute paths, `..`) is rejected」所承诺的安全边界不符 —— 该注释只覆盖了词法路径，没覆盖软链接。
- 修法（**先定策略，再定实现；验收不能声称消除了竞态**）：
  - **第一步是定软链接策略**（产品决策，验收标准取决于它）：目标目录里出现软链接时是**拒绝导出**（fail fast，把问题暴露给用户）、**跟随**（维持现状但需用户知情），还是**就地写真实文件**（不跟随已有软链接）？这三条互斥，修法不能先写代码再补策略。
  - **实现方向**：解析最终路径（`canonicalize`，或逐级 `symlink_metadata` 判定）确认其仍以目标目录为前缀；目标目录在写入前 canonicalize 一次并缓存；悬空软链接必须用 `symlink_metadata` 判定（`Path::exists()` 对悬空链接返回 false，会漏掉）。
  - **TOCTOU 边界（不能宣称消除）**：以上检查只反映**检查时刻**的文件系统状态。检查通过之后、实际写入之前，路径组件仍可能被替换为软链接（被并发进程替换，或用户在他处操作）→ 存在检查与写入之间的竞态窗口。要真正消除它需要基于 fd 的写法（打开目标目录 fd → 用 `openat` / `O_NOFOLLOW` 相对写入），或「临时文件 + 原子替换」且临时文件也建在同一目录内。本仓库当前是「拼路径 + `fs::write`」，改造成本属结构性改动 —— 因此本条的验收目标应写成「**消除已知的软链接逃逸路径**」，而不是「消除检查与写入间的竞态」。
  - **平台边界**：`symlink_metadata` 对 Windows 的 junction（目录联接）与符号链接的行为与 Unix 不一致；Windows 上创建符号链接还需要特权或开发者模式。验收用例需在 Unix 与 Windows 分别执行，并明确 junction 是否纳入策略。
  - 另外 `dir` 参数本身也应校验（见 F 段备注）。

### A17 [中] TTL 没有下界校验 → `0` 被接受 → 生成的 Redis 缓存**永不过期**

- 位置：`src-tauri/src/main.rs:14`（`cache_ttl_seconds: u32`，只挡负数）、`src-tauri/src/main.rs:61`（原样透传）；前端 `src/App.vue:302-307` 的 `min="1"` 只是 HTML 输入提示，不阻止输入 0；生成侧 `crates/core/src/layers.rs:269`（`const {Struct}CacheTTL = {ttl} * time.Second`）、`crates/core/src/layers.rs:302`（`c.rdb.Set(ctx, key, encoded, {Struct}CacheTTL)`）
- **我已实测**：传 `cache_ttl_seconds: 0` → 生成物为
  ```go
  const OrderCacheTTL = 0 * time.Second
  ...
  _ = c.rdb.Set(ctx, key, encoded, OrderCacheTTL).Err()
  ```
  而 go-redis v9.7.0 的 `Set` 实现（`string_commands.go:166-184`）是 `if expiration > 0 { ...追加 ex/px... } else if expiration == KeepTTL { ... }` —— **expiration == 0 时一个过期参数都不追加**，该函数的文档注释也写明 "Zero `TTL` or `Expiration` means that the key has no expiration time"。→ **Redis key 永久保留**。
- **已确立的事实**：`cache_ttl_seconds = 0` 会被原样接受，生成物里是 `const XxxCacheTTL = 0 * time.Second`，而 go-redis v9.7.0 的 `Set` 在 expiration == 0 时不追加任何过期参数 → **该 key 不会自动过期**（实测 + 源码双重确认，见上）。
- **未确立的部分（此前写错，已更正）**：本条报告先前写「用户以为 0 = 立即过期 / 不缓存」——这是**推测**，spec / 界面文案 / 代码注释里都没有关于 `0` 语义的约定，无文档、无测试、无提示可佐证，该表述已删除。**永久缓存本身不是自相矛盾的配置**：有意设置长 TTL、乃至不过期，是合法需求。真正的问题是 `0` 这个取值在语义上不可区分 —— 它既可能被理解为「关闭缓存」（缓存装饰器仍在跑、仍走 Redis、只是不过期），也可能被理解为「永久缓存」，而契约没写。界面输入框的 `min="1"` 只是 HTML 提示，不阻止输入 0（`src-tauri/src/main.rs:14` 的 `u32` 也只挡负数）。
- **与 A4 叠加的后果（待端到端验证，尚未确立）**：A4 的缓存 key 读写不对称（Update 删的是另一个 key）叠加「key 不过期」时，被写脏的条目可能长期驻留。这条组合后果**未做端到端验证**（需真实 Redis + 一次 Update 后回读确认），不应作为已确立结论；修完 A4 / A17 后需要补这一条端到端验证。
- 修法（**先把语义写进契约，再做校验**）：
  1. 明确 `0` 的语义，三选一：(a) `0` = 关闭缓存装饰器；(b) `0` = 显式不过期（生成物里用 `XxxCacheTTLNeverExpire` 之类**名字**表达，而不是留一个 `0 * time.Second`）；(c) `0` 非法并报错。(a)/(b) 都自洽，关键是别让「关闭」与「永久」共用同一个数值。
  2. 按选定的语义在入口归一化或拒绝（`src-tauri/src/main.rs:14`），生成侧对归一化后的值再断言一次（`crates/core/src/layers.rs:269`）。
  3. 前端把 `min="1"` 换成真正的校验，让非法值在提交前就报错。

### A18 [低] I/O 错误信息不含路径 + 前端把长错误截断且无提示 → 用户定位不到失败文件

- 位置：`crates/core/src/export.rs:30`（`ExportError::Io(msg)` 的 Display 是 `write failed: {msg}`）、`crates/core/src/export.rs:67`、`:69`（`ExportError::Io(e.to_string())` —— `std::io::Error` **不带路径**）、`src-tauri/src/main.rs:131`（原样透传）、`src/App.vue:216-218`（拼成 `Export failed: write failed: ...`）
- 实测：`fs::write` 失败的文本是 `No space left on device (os error 28)` / `Not a directory (os error 20)`，**均不含是哪个文件**；`create_dir_all` 同样。
- 后果：导出 8 个文件时用户无法定位失败项；叠加 `src/App.vue:368-370` 的错误展示（overflow hidden + 省略号，**没有 `title` 属性**），长错误在 UI 上还会被截断，用户连完整错误都读不到。
- 修法：包一层带路径的 IO 错误（`format!("{}: {}", path.display(), e)`）；前端错误展示补 `title` 全量提示，或改成可展开。

---

## B. 前端与 IPC

### B1 [高] 后端发 `statementIndex`，前端读 `statement_index` → 解析错误的语句序号永远显示不出来

- 位置：`src-tauri/src/main.rs:86-91`（`#[serde(rename_all = "camelCase")]` on `ParseErrorPayload { statement_index, snippet }` → JSON key 是 `statementIndex`）、`src/App.vue:22-25`（`interface ParseError { statement_index: number; snippet: string }`）、`src/App.vue:180`（`statement_index: typeof c.statement_index === "number" ? c.statement_index : 0`）、`src/App.vue:256` 与 `:327`（`v-if="error.statement_index"`）
- 链路（我已逐段核实）：`tauri-2.12.7/src/ipc/mod.rs:240` `impl<T: Serialize> From<T> for InvokeError` → `src/ipc/protocol.rs:409` `format_result` 把 `Err` 值序列化给 error 回调 → `tauri/scripts/core.js` `registerCallback((e) => reject(e))` reject 的是**已解析的 JSON 对象**。所以 JS 侧收到 `{ statementIndex: 1, snippet: "..." }`，而 `c.statement_index` 恒 `undefined` → 恒为 `0`。
- 现象：`v-if="error.statement_index"` 拿到 `0`（falsy）→ **永远走 v-else 的 "Parse error"**，"Parse error in statement #N" 这行永远不渲染。后端明明送来了 `statementIndex: 1`（`crates/core/src/parser.rs:37` 固定为 1），被前端丢弃。
- rev-io 独立复现：同一个结构体序列化结果为 `{"statementIndex":3,"snippet":"boom"}` —— 键名确实是驼峰，前端读的蛇形键恒不存在。
- 意义：这条**独立于**已知的 story 26 缺口 —— 即使后端把序号算对，界面也显示不出来。story 26 实际是两个缺陷叠加。
- 违反 `spec.md:42` 的 US26「解析错误定位到第几条语句」。

### B2 [中] TTL 输入框清空即可触发 IPC 反序列化失败，且被误报成 "Parse error"

- 位置：`src/App.vue:160`（`cacheTtlSeconds: cacheTtlSeconds.value`）、`src/App.vue:176-185`（`toParseError`）、`src-tauri/src/main.rs:14`（`cache_ttl_seconds: u32`）
- 机理（Vue 源码 `@vue/shared` 的 `looseToNumber`）：`parseFloat("")` 是 NaN → **原样返回字符串** → `v-model.number` 在输入框被清空时得到 `""` 而不是 `0`
- 现象：勾选 Redis cache → 把 TTL 输入框内容全选删除 → 点 Generate → 后端 u32 反序列化失败 → tauri reject 字符串 `"invalid args \`request\` for command \`generate_project\`: invalid type: string \"\", expected u32"` → 前端 `"snippet" in cause` 不匹配（该串不含 "snippet"）→ 落到兜底分支 `{ statement_index: 0, snippet: String(cause) }` → 标题仍然是 **"Parse error"**
- 用户被告知 DDL 解析失败，实际问题在 TTL 输入框。`min="1"` 不起任何拦截作用（没有 form 提交，浏览器不阻止）；填 `0` 会被后端当合法 u32 接受并生成 `const XCacheTTL = 0 * time.Second`（后果见 A17）。
- rev-io 补强：填 `-1` 时用户看到标题 `Parse error`、正文 `invalid args \`request\` for command \`generate_project\`: invalid value: integer \`-1\`, expected u32 at line 1 column 2` —— 内部序列化框架术语直接透给用户；Vue 的 `looseToNumber`（`node_modules/@vue/shared/dist/shared.cjs.js:103`）对空串返回原字符串 `""`，同样触发 `invalid type: string "", expected u32`。

### B3 [中] module path 不校验 → 生成编译不过的 Go 代码，界面毫无提示；且含引号/换行的值能注入 import 块

- 位置：`src/App.vue:284-287`（输入框无任何校验）、`src/App.vue:158`（原样透传）、`crates/core/src/layers.rs:54`、`:146-147`、`:265-266`（`request.module_path` 原样拼进 import 字符串）、`src-tauri/src/main.rs:57-64`（未校验直接透传）
- 现象：清空 module path → 生成 `import "/internal/model"` → 界面照常显示代码和 "Dependencies: go get gorm.io/gorm"，**没有任何 warning**；导出后 `go build ./...` 才炸：`invalid import path "/internal/model"`。错误在离生成动作很远的地方才暴露。
- **rev-io 补强的两点**（均已实测）：
  - `module_path = "my module"`（含空格）→ 生成 `"my module/internal/model"` → `go build` 报 `invalid import path: my module/internal/model`；
  - 值里带引号或换行时，内容会被**注入进 import 块**，生成语法非法的 Go（不只是路径非法）。因为 module path 是自由文本 + 原样拼接，这已经接近代码注入面，值得按输入边界来校验。

### B4 [中] 复制按钮失败时完全静默

- 位置：`src/App.vue:187-193`：`await navigator.clipboard.writeText(text)` 无 try/catch、无特性检测
- 现象：被拒绝（权限、文档失焦）时 `copied.value = label` 永不执行 → 按钮不会变成 "Copied!"；rejection 冒出到 `@click` 处理器外，Vue 只在控制台打印 "Unhandled error during execution of native event handler"。用户看到的就是「点了没反应」。
- 补充：本项目没装 Tauri clipboard 插件（`src-tauri/capabilities/default.json` 只有 `core:default` 与 `dialog:allow-open`，`package.json` 无 `@tauri-apps/plugin-clipboard-manager`），完全依赖 Web API，而 `navigator.clipboard` 要求安全上下文。

### B5 [中低] localStorage 写路径没有保护（读路径有），且每次按键都同步写盘

- 位置：`src/App.vue:57-75`（读，有 try/catch，注释还写着「ignore corrupt state」）vs `src/App.vue:110-125`（写，裸 `localStorage.setItem`）
- 现象：`setItem` 抛 QuotaExceededError（DDL 很大时）→ 异常从 watcher 回调冒出，Vue 只在控制台打印 "Unhandled error during execution of watcher callback" → **持久化静默失效**，界面毫无提示，用户以为设置已保存。
- 连带：watch 源含 `ddl` → 每敲一个字符都同步执行一次 `JSON.stringify` + `setItem`（主线程），大 DDL 时是明显的输入延迟来源。
- 连带：B2 产生的 `""` 会被持久化，下次启动 `src/App.vue:68` 的 `Number.isInteger("")` 为 false → 静默回退 300，用户设的值无声丢失。

### B6 [低] 列表 key 重复

- 位置：`src/App.vue:346`（`:key="w"`）、`src/App.vue:376`（`:key="t"`）、`src/App.vue:390`（`:key="f.path"`）
- 现象 1：`crates/core/src/generate.rs:122-124` 的 warning 格式是 `"{table.name}: unsupported column type \`{base}\`, mapped to \`any\`"` → 同一张表里两列同类型未映射列 → 两条**完全相同**的字符串 → Vue 控制台 "Duplicate keys found during update:"，keyed diff 复用错误节点。
- 现象 2：`crates/core/src/generate.rs:64` 的 `table_names` 未去重 → 同一段 CREATE TABLE 粘贴两遍 → 表清单出现两个同名 "users" 勾选框，文件列表出现两个相同的 `internal/model/user.go`，右上角 "N / M files" 计数重复。

### B7 [已核实无问题] 几处看着危险但实际安全的点

- **XSS**：`src/components/CodePreview.vue:28`、`src/components/SqlEditor.vue:40` 两处 `v-html` 不是注入点 —— Shiki 的 `codeToHast` 产出 text 节点，经 `hast-util-to-html` 转义，实测输出 `&#x3C;` / `&#x26;`，无裸 `<script>`；`src/lib/highlight.ts` 兜底分支也转义了 `& < >`。
- **异步竞态**：`src/components/CodePreview.vue:15-23` 与 `src/components/SqlEditor.vue:14-23` 用单调 token 丢弃过期高亮结果，不会乱序覆盖；`generate()` 在 `src/App.vue:154-162` 同步快照全部输入，Generate 按钮在 pending 期间禁用。
- **表勾选/预览/导出一致性**：`visibleFiles`（`src/App.vue:127-129`）、预览树（`:390`）、Copy all（`:226-228`）、导出（`:202`）用的是同一个 computed；`project.tables` 与 `files[].table` 同源于 `table.name`，过滤无错配；取消勾选后 `toggleTable`（`:135-144`）会把 `selectedPath` 重新指向首个可见文件。
- **导出路径安全（仅限词法路径）**：`crates/core/src/export.rs:76-85` 的 `safe_join` 拒绝一切非 Normal 路径分量，绝对路径、`..`、`.`、Windows 前缀 `C:` 在 Unix 与 Windows 上都被拦截；Unix 下 `internal\model\x.go` 只是单个 Normal 组件（反斜杠不是分隔符），最多产生一个奇怪文件名，**不会逃出目录**。但**词法检查不覆盖软链接** —— 见 A16。
- **「先确认后覆盖」的顺序是对的（rev-io 复核）**：`export.rs:52-63` 的 `overwrite=false` 分支在**写任何文件之前**就返回 `Conflicts`，不存在「没确认就写了」的情况。
- **目录创建 / 权限失败没有被吞掉（rev-io 复核）**：都会变成 `ExportError::Io`（`export.rs:67`、`:69`），只是错误信息里没有路径（见 A18）。
- **`dir` 参数本身未校验（rev-io 复核，低危）**：非绝对路径 / 不存在 / 不是目录都没拦，但唯一来源是 dialog 插件的返回值，用户不可达；若将来出现别的调用方，空串会让写入落到进程 CWD。
- **次要提示**：`src/App.vue:195-202` 在 `await open(...)` 之后才置 `exporting = true`，选择器打开期间 Export 按钮仍可点，会再弹一个选择器并触发第二次导出。

### B8 [高] 覆盖确认用了 `window.confirm` → macOS 上 WKWebView 不弹框、confirm 返回 false → 导出覆盖功能在 macOS 上不可用

- 位置：`src/App.vue:206`（全项目唯一一处浏览器原生对话框：`grep -rn "window\.confirm" src/` 只命中这一行）
- **【已实证】WebKit 层行为**（本轮新增，A/B 对照实验，工具在仓库外：`/tmp/wk_confirm_probe.swift`）：一个独立 WKWebView harness，delegate 的方法集刻意与 wry 0.57.3 的 `WryWebViewUIDelegate` 对齐（只实现 `runOpenPanelWith` / `createWebViewWith` / `requestMediaCapturePermission`），跑 `window.confirm('probe')` 并回读 `document.title`：
  - **RUN 1（wry 同构，无 confirm 面板实现）**：`RESULT: confirm_result=false`；delegate 的 confirm 回调**从未被触发**；4 秒内 `evaluateJavaScript` 即返回，**没有弹出任何面板**（未命中 20 秒超时）。
  - **RUN 2（对照组：delegate 实现 `runJavaScriptConfirmPanelWithMessage`）**：delegate 回调被调用（`DIALOG: delegate confirm panel was invoked with message=probe`），结果 `confirm_result=true`。
  - → 结论：「**WKUIDelegate 未实现 confirm 面板方法时，`window.confirm()` 静默返回 false 且不弹框**」在本机（macOS 27.2.1 / Xcode WebKit）**已实证**，对照实验排除了 harness 本身失效的可能。
- **【源码推定，未在应用内端到端验证】应用层**：Tauri 2 的 macOS 端是 wry → WKWebView；wry 0.57.3 的 `src/wkwebview/class/wry_web_view_ui_delegate.rs` 中 `unsafe impl WKUIDelegate for WryWebViewUIDelegate` **只实现三个方法** —— `runOpenPanelWith`（文件上传）、`requestMediaCapturePermission`（媒体权限）、`createWebViewWithConfiguration`（新窗口）；**没有** `runJavaScriptConfirmPanelWithMessage:initiatedByFrame:completionHandler:`（`grep -rln "runJavaScriptConfirm\|runJavaScriptAlert"` 在整个 wry crate 里零命中）。该 delegate 由 `src/wkwebview/mod.rs:652-658` 装到 WKWebView 上；`tauri-2.12.7` 与 `tauri-runtime-wry-2.12.5` 也没有另装一个补位的 delegate。
  - **边界**：「本应用在 macOS 上导出覆盖确认必然失效」是由上述源码结构**推得**的，**没有启动本项目的 Tauri 应用跑完整导出流程**（见 F 段）。WebKit 层的行为已实证，应用层的连接是源码推定。
- 后果（按上述推定）：把代码导出到一个已经有同名文件的目录 → 后端返回 conflicts → `window.confirm` 返回 false → 走 `src/App.vue:210-213` 的「Export cancelled」分支 → **用户永远无法确认覆盖**，冲突提示形同虚设；想覆盖只能先手动删文件。
- 影响面：macOS 是本项目当前的主要开发/运行平台（`npm run tauri dev`），这条是**功能性失效**，不只是体验问题。Windows(WebView2)/Linux(WebKitGTK)/Android 的 JS 对话框由宿主原生处理，不受影响。
- 违反 `spec.md:39` 的 US23「导出时对同名文件先确认再覆盖」。
- 修法二选一：改用 Tauri 官方 dialog 插件的 `ask()`/`confirm()`（Rust 侧 `src-tauri/src/main.rs:138` 已经 `.plugin(tauri_plugin_dialog::init())`，前端 `src/App.vue:3` 已经 `import { open } from "@tauri-apps/plugin-dialog"`，只差在 `src-tauri/capabilities/default.json` 补 `dialog:allow-confirm`（必要时加 `dialog:allow-message`）并在 `src/App.vue:206` 换成 `await confirm(...)`），或在页面里做一个自绘的确认弹窗；**不要**继续用 `window.confirm`/`alert`/`prompt`。

### B9 [低-中] `generate_project` / `export_project` 是同步命令 → 命令体在主线程同步执行（结构性反模式；实测耗时不构成用户可见冻结）

- 位置：`src-tauri/src/main.rs:46`（`fn generate_project(...)`，非 `async fn`、无 `#[tauri::command(async)]`）、`src-tauri/src/main.rs:106`（`fn export_project(...)`，同样）
- **主线程路径（逐跳取证，版本取自 `Cargo.lock`：tauri 2.12.7 / tauri-macros 2.7.6 / tauri-runtime-wry 2.12.5 / wry 0.57.3）**：
  1. `tauri-macros-2.7.6/src/command/wrapper.rs:52` — `execution_context` 默认是 `ExecutionContext::Blocking`
  2. `tauri-macros-2.7.6/src/command/wrapper.rs:423-427`（`body_blocking`）— 展开后是 `let result = $path(args); kind.block(result, resolver); return true` → **在调用线程上直接调用命令函数，不 spawn**
  3. `tauri-macros-2.7.6/src/lib.rs:46-49` — 框架自己的属性文档：「`async` — Runs the command on the async runtime instead of **blocking the main thread**」，并注「this option is only needed for synchronous functions that **should not run on the main thread**」→ 默认的同步命令就是在主线程执行
  4. `wry-0.57.3/src/wkwebview/class/wry_web_view_delegate.rs:33-60` — `WKScriptMessageHandler` 的 `did_receive` 里直接 `ipc_handler(r)`（无线程切换）
  5. `tauri-runtime-wry-2.12.5/src/lib.rs:5281-5300` — `create_ipc_handler` 的闭包直接调用 tauri 的 handler（无线程切换）
  6. `tauri-2.12.7/src/webview/mod.rs:2165` → `src/manager/mod.rs:459` → `(self.webview.invoke_handler)(invoke)` → 命令包装器

  ⇒ 整条链路没有任何线程切换：命令体在 WebKit 投递 IPC 消息的那个线程上同步执行，macOS 上该回调由主线程处理。
- **耗时实测（本轮新增，工具在仓库外 `/tmp/genbench/`：独立 crate + path 依赖 `crates/core`，debug profile，缓存开启，写盘到本地临时目录）**：

  | tables | 生成文件数 | generate_ms | export_ms | 合计_ms |
  |---|---|---|---|---|
  | 1 | 4 | 2.4 | 1.2 | 3.6 |
  | 10 | 40 | 2.8 | 7.0 | 9.7 |
  | 50 | 200 | 5.5 | 12.1 | 17.6 |
  | 100 | 400 | 7.9 | 23.4 | 31.3 |
  | 200 | 800 | 17.0 | 49.3 | 66.3 |

  rev-io 独立测得同一量级（2000 张表生成约 105 ms、8000 个文件导出约 400 ms），与本表互相印证。

- **修正后的结论（r2）**：「命令体占用主线程」是**结构事实**（路径已逐跳取证）；但在本机实测的规模下命令体耗时为 **3–66 ms**，**未观测到用户可见的冻结**。上一版写的「用户观感就是卡死了」缺乏测量依据，**撤回**。级别由 [中] 降为 **[低-中]**，定级依据是「结构性反模式 + 实测量级」，不是「已发生冻结」。风险只在异常输入下才可能显现（超大 DDL、慢盘/网络盘、海量文件），本轮**未构造**该场景（见 F 段）。
- 修法不变且代价很低：给这两个命令加 `#[tauri::command(async)]`（宏会把命令体放到 async runtime 上），或把函数改成 `async fn`。核心 crate 的 `generate` 是纯函数、不做 IO。

---

## C. 仓库配置与工程化

### C1 [低] 本机 cargo 源被一个未跟踪、未共享、未文档化的本地覆盖绕开（**r2 撤回 r1 的「构建必需」结论**）

- 事实：
  - `~/.cargo/config.toml`（**全局**，不在仓库内）把 crates-io 指向 `sparse+https://mirrors.tuna.tsinghua.edu.cn/index/`；该索引已过期，`cargo build` 会挂住 —— 这是**本机环境问题**。
  - 仓库内存在 `.cargo/config.toml`，把 crates-io 覆盖为 `sparse+https://rsproxy.cn/index/`，用来绕开上面那个过期索引。
  - 该文件**未被跟踪**，且是被 `.git/info/exclude:7` 排除的 —— 受版本控制的 `.gitignore` 只有 `/target`、`node_modules/`、`dist/`、`src-tauri/gen/schemas/`、`.DS_Store`，**完全没提 `.cargo/`**；`.git/info/exclude` 是本地文件，不随 clone 分发。
- **撤回（r1 的过度外推）**：r1 写的「**它是构建必需**」「未跟踪 → 新克隆的机器 / CI 上 `cargo build` 会卡死或超时」**因果不成立**：
  - rsproxy **不是**构建的必要条件 —— 干净环境下默认 crates.io 直连可用，新克隆的仓库和 CI 都不需要这个覆盖；
  - 「全局 TUNA 索引过期」是**本机环境**的问题，推不出「所有新克隆 / CI 都会卡」；
  - 因此 r1 把一个环境问题写成了项目级构建依赖，这一结论撤回。
- **仍然成立的部分（降级为 [低]）**：本机构建依赖一个「只有这台机器有、clone 不到、也没有任何文档说明」的覆盖。同事 clone 仓库拿不到它；若他也配了同样过期的镜像源，`cargo build` 会挂住且无从排查（没有 README、没有 CI 来暴露这一点）。
- 仓库层真正的缺陷在 **C2（无 CI）** 与 **C4（无 README）**：换源 / 配置源这类环境前置条件本就该写进 README，而不是留在某台机器的 `.git/info/exclude` 里。
- 建议（低优先级，不阻塞其它项）：把 crates-io 源的选择写进 README 的环境准备一节；CI 里固定用官方 crates.io；若确实要团队共享镜像配置，应显式写进 `.gitignore` 并文档化，而不是依赖本地 exclude。

### C2 [中] 完全没有 CI

- 没有 `.github/`，`cargo test` / `npm run typecheck` 没有任何自动化触发。上面 A/B 段所有缺陷能长期存活，直接原因就是这一条。

### C3 [中] `csp: null` → 不注入 CSP

- `src-tauri/tauri.conf.json:20`。当前没有已知注入点（见 B7，Shiki 输出已转义），capabilities 也较窄（`core:default` + `dialog:allow-open`），爆炸半径有限；但这是唯一的 XSS 纵深防御层且已关闭。

### C4 [低] 没有 README，且 `bundle.active: false`

- 无构建/运行说明；`src-tauri/tauri.conf.json` 的 `"bundle": {"active": false}` → `tauri build` 不产出可分发包。

---

## D. 为什么这些都能逃过测试

`crates/core/tests/compile.rs:42` 的四格矩阵（MySQL/PG × 缓存开/关）全部使用 `tinyint_as_bool: false`，且主键只有单列 `id BIGINT`；`crates/core/src/generate/tests.rs:79-89` 的 tinyint bool 用例只断言 `files[0]`（模型文件），而那张表 `CREATE TABLE t (f TINYINT(1) NOT NULL)` **没有主键** → 根本不生成 repository/service/cache → bool 路径从未经过 `go build`。fixture `crates/core/src/fixtures/sample_mysql_ddl.sql` 有 `age TINYINT(1) NULL`、`status TINYINT NOT NULL`，但没有 tinyint(1) 主键。

即：**A1/A2/A3/A4/A9/A11 全部落在现有覆盖之外**，A4 恰好落在两处覆盖的交集里。

导出层同样没有针对性测试：`crates/core/src/export.rs:87-196` 的单元测试只覆盖「正常写入 / 冲突 / 覆盖 / 绝对路径 / `..` 遍历」五种，**没有任何软链接用例**（A16）、没有批内重复路径用例（A14）、没有写入失败后的原子性用例（A15）、没有「错误信息含路径」断言（A18）。TTL 也没有下界用例（A17）。

---

## E. 建议的修复顺序（按 §0.2 的 13 个修复主题组织）

**第 0 批 —— 一行改动、直接解锁用户可见性**：B1（前端改读 `statementIndex`）。这同时让 story 26 的「解析错误定位到第几条语句」第一次能被用户看到。

**第 1 批 —— 生成前把「编译失败类」堵死（R1 / R4）**：
1. **R1 标识符合法化与消歧（A1 / A8 / A9 / A11 / A14）**：加一个生成前校验层（`crates/core/src/naming.rs` + `crates/core/src/generate.rs`）—— 表名/列名字符白名单（`[A-Za-z0-9_]`）、字段名与模型固定方法名（如 `TableName`）冲突检测、字段名重复消歧、复数化后表名撞名检测。**A9 与 A14 是同一现象在生成层与导出层的两个防御点**（同一修复主题 R1）：生成层消歧（表名撞名时报错/加后缀）+ 导出层检测（批内重复路径直接失败，不要静默覆盖）。
2. **R4 生成物 import 与类型映射联动（A3）**：生成器在产出 `time.Time` 这类类型时必须同步收集对应 import（模型侧已有 `crates/core/src/generate.rs:146-148` 的 `needs_time_import` 雏形，仓储/服务的 import 写死在 `crates/core/src/layers.rs:49-55`、`:143-148`）—— 这是「类型映射 → import」的联动问题，**不属于命名校验**。

**第 2 批 —— 主键与类型贯通（R2 / R3）**：
3. **R2 主键标识符按方言 × 引号规则比较（A2）**：不要统一成大小写不敏感。归一化比较键按 §A2 的三步规则算（PG 未加引号 → 小写折叠；PG 加引号 → 原样精确；MySQL 列名 → 忽略大小写），并在 `crates/core/src/layers.rs:19` 与 `crates/core/src/generate.rs:118` 两处用**同一个**归一化函数；同时让 `crates/core/src/generate.rs:69` 依据「解析出的主键列数」而非「原始约束非空」决定是否生成 repo/service/cache，匹配失败必须出 warning 而不是静默丢弃。
4. **R3 类型信息层间不丢失（A4 / A5）**：`pk_columns` 接收 `request` 并复用 `request.tinyint_as_bool` 与 `deleted_at → gorm.DeletedAt` 的特殊映射；缓存 key 的读写两侧统一取值表达式；补 `int8/int4/int2/float8/float4` 映射。

**第 3 批 —— 导出与前端输入（R7 / R8 / R9 / R10 / R12）**：
5. **R10 换掉 `window.confirm`（B8）**：改用 tauri dialog 插件的 `ask()`/`confirm()`（需在 `src-tauri/capabilities/default.json` 补 `dialog:allow-confirm`）或自绘确认弹窗。这是 macOS 上导出覆盖功能直接不可用的一条。
6. **R8 导出层加固（A16 / A15 / A14 / A18）**：
   - **A16 软链接**（最高优先）：**先定策略**（拒绝 / 跟随 / 就地写真实文件，三选一，属产品决策），再按策略实现（`canonicalize` 或逐级 `symlink_metadata` 判定悬空软链接，确认最终路径仍在目标目录内）。验收只能写「消除已知的软链接逃逸路径」—— 检查与写入之间仍有 TOCTOU 窗口，且需分别在 Unix / Windows 上验证（junction 是否纳入策略要一并定）。这是唯一会写到用户没选的目录、甚至覆盖目录外文件的一条。
   - **A15 原子写**：写同目录 `.tmp` + `fs::rename`，保证**单个文件**不出现半截（原内容不会被先截断）；整批不是事务，失败清理**只删本次新建的文件与临时文件、绝不删导出前已存在的同名文件**，也不依赖内存备份回写（回写在磁盘满时同样会失败）。若要整批级保证，需改成「写临时目录 + 整体替换」，属产品决策。
   - **A14 批内重复路径**：与 A9 的类型名消歧配套（生成层 + 导出层两个防御点）。
   - **A18 错误信息**：IO 错误带上路径，前端补 `title` 全量提示。
7. **R7 输入校验（B2 / B3 / A12 / A17）**：**A17 先定 `0` 的语义再校验**（关闭缓存 / 显式不过期 / 非法报错，三选一，写进 spec 与界面文案），别让「关闭」与「永久」共用一个数值；再在入口归一化或拒绝，生成侧二次断言。TTL 空值/非正数在输入侧规整；module path 非空、路径合法性与字符集校验（还要防引号/换行注入进 import 块）；分页参数非法要报错而不是静默返回第一页。
8. **R9 IPC 契约（B1 / B2）**：统一 DTO 命名（`statementIndex` vs `statement_index`）、`toParseError` 的兜底分支不要一律标成 "Parse error"、清空 TTL 触发的反序列化失败要与解析错误区分开。
9. **R12 前端健壮性（B4 / B5 / B6）**：复制失败要有 toast；localStorage 写侧 try/catch；列表 `key` 去重。

**第 4 批 —— 方言与解析语义（R5 / R6）**：
10. **R5（A6 / A10）**：`COMMENT ON COLUMN` 取 object name 的正确 schema 分量；生成 SQL 时列名/表名按方言加引号（保留字列名不再产生运行时语法错误）。
11. **R6（A7）**：`is_identity` 区分 `GENERATED ALWAYS AS` 与自增，生成列不生成 INSERT。

**第 5 批 —— 工程化（R13）与加固（R11）**：
12. **R13**：先补 **C2（CI）** —— `cargo test` + `npm run typecheck` + 一条 `go build` 冒烟；再补 **C4（README）**（含环境准备：crates-io 源、Go、Node、tauri CLI），顺带把 **C1** 的镜像说明写进 README；**C3（`csp: null`）**与 **A13（注释漂移）**可随手带上。
13. **R11（B9）**：给 `generate_project` / `export_project` 加 `#[tauri::command(async)]`。实测命令体耗时 3–66 ms，不阻塞其它批次，属于便宜的加固。

**每批的验收要求**：对应缺陷各补一个 fixture / 回归用例。现有矩阵（`crates/core/tests/compile.rs`）全用 `tinyint_as_bool: false` + 单列 `id BIGINT`，必须补上 tinyint(1) 主键、大小写主键、保留字列名、非法字符表名、撞名表等输入；导出层还要补软链接目标目录 / 悬空软链接文件 / 批内重复路径 / 写入失败后原文件仍在 / TTL=0 五类用例，否则修完仍无保护（见 D 段）。

---

## F. 未验证 / 不主张的部分

- **A2** 的 PG 侧行为只在 DDL 文本路径验证过（PG 的标识符折叠发生在数据库侧），未在真实 PG 实例上跑过；「方言 × 引号规则」的归一化方案是**设计建议**，未实现验证。
- **A4** 的「更新后读脏数据」是通过生成物比对 + 独立 Go 程序验证 key 不等式得出的，未在真实 Redis 上跑端到端。
- **A5 / A6 / A7** 只验证了生成结果与警告文本，没有验证「生成物在 PG 上实际可编译且语义正确」。
- **B8**：**WebKit 层行为已实证**（独立 WKWebView A/B 对照实验，wry 同构 delegate → confirm 静默 false；补上 confirm 实现 → 正常 true）。但「**本应用**在 macOS 上必然失效」仍是**源码推定**（wry 0.57.3 的 `WryWebViewUIDelegate` 未实现 confirm 面板方法，tauri 未补位），**没有启动本项目的 Tauri 应用跑完整导出流程**。要彻底坐实，需在 macOS 上跑一次「导出到已有同名文件的目录」，看是否出现确认弹窗。工具：`/tmp/wk_confirm_probe.swift`（仓库外）。
- **B9**：主线程路径已逐跳取证，命令体耗时已实测（3–66 ms，debug profile）。**未构造**异常场景（超大 DDL / 慢盘或网络盘 / 海量文件），因此「已发生用户可见冻结」这一说法不成立，本报告不主张之。工具：`/tmp/genbench/`（仓库外）。
- **C1**：本报告**不主张**「新克隆或 CI 构建会失败/卡死」——该结论已在 r2 撤回。C1 只主张「本机构建依赖一个 clone 不到、未文档化的本地覆盖」。
- **A16**：三种软链接形态（目录组件软链接 / 文件软链接 / 悬空软链接）均已**实测复现**，写入确实落到目标目录之外；**未**在真实用户目录里跑完整 GUI 导出流程（探针是直接调 `export_project`）。**未**验证任何修法能消除检查与写入之间的 TOCTOU 窗口（当前实现是「拼路径 + `fs::write`」，要真正消除需改成基于 fd 的相对写入，属结构性改动）；**未**在 Windows 上验证 junction（目录联接）与符号链接的行为差异，也未验证 Windows 创建符号链接的权限前提 —— 平台边界是明确的未验证项。
- **A17**：已实测生成物为 `const OrderCacheTTL = 0 * time.Second` 并核对 go-redis v9.7.0 的 `Set` 源码（expiration == 0 不追加过期参数）；**未**连真实 Redis 观察 key 是否真的不过期（该行为由 Redis 的 `SET` 语义决定，属高置信推断）。**「0 的语义是什么」在 spec / 界面 / 代码注释里均无约定，本报告不对用户意图作任何假设**（先前版本写的「用户以为 0 = 立即过期」是推测，已删除）；**与 A4 叠加导致脏条目长期驻留的后果未做端到端验证**（需真实 Redis + 一次 Update 后回读）。
- **A15**：原文件被 O_TRUNC 抹掉的后果由 rev-io 实测（`RLIMIT_FSIZE` 截断写入）验证；本报告**未**在真实磁盘满 / 配额超限的场景复现（用 rlimit 模拟，等价）。本报告**不主张**整批级原子性 —— 现有实现逐个写文件，整批中途失败必然留下混合状态；「失败清理不得删导出前已存在的同名文件」「内存备份回写不保证恢复」是给修复者的约束，不是本条缺陷的既成事实。
- 本次 review **没有改动任何生产代码**，以上均为待修项；期间用于实证的临时探针文件均已删除，工作树只剩文档修改（见 §0.1）。
