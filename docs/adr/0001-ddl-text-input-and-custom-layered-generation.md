# DDL 文本输入与自研分层生成

## Status

accepted

## Context

参考实现 gormt 只能连接数据库读取 `information_schema`，产物只有 model 与查询函数；GORM 官方 CLI 的输入是 Go struct，`gorm.io/gen` 的输出以 DAO 为中心。两者都不提供 repo/service 分层与缓存产物，也无法只凭一段建表语句离线生成。

## Decision

以粘贴的 DDL 脚本为唯一输入，在 Rust 侧用 sqlparser 解析，生成 Go 的 model/repository/service/cache 四层代码；仓储层内部使用 GORM 原生 API，不引入 `gorm.io/gen` 的 DAO。缓存以装饰器形式实现同一仓储接口。

## Considered Options

- **复用 gormt**：只能连库、无 repo/service 分层、模板写死不可改，放弃。
- **gorm.io/gen**：DAO 与自定义 repo/service 分层职责重叠，且依赖连库生成，放弃。
- **进程内模板引擎（askama/tera）**：Go 代码对空白敏感，模板的缩进控制收益低，改为 Rust 字符串构建 + golden 测试。

## Consequences

- 用户必须自备 DDL（可从任意工具导出），不做数据库内省。
- 生成的仓储接口天然可 mock、可被缓存装饰器包裹。
- 生成逻辑集中在 Rust，未来加 CLI 模式成本低。
