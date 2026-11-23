# 10: 生成物编译验证与全量回归

**What to build:** 端到端测试：对 MySQL 与 PostgreSQL 样例分别以缓存开/关生成产物，写入临时 Go 项目（含 go.mod），执行 `go build ./...` 必须通过；同一输入两次生成结果一致的全矩阵幂等回归同步覆盖。本机 Go 1.27.3 可用。

**Blocked by:** 05, 06

**Status:** resolved

- [x] MySQL 样例 × 缓存开/关 生成物可编译
- [x] PostgreSQL 样例 × 缓存开/关 生成物可编译
- [x] 全矩阵幂等断言通过
- [x] 测试可在本机用一条命令运行

## Comments

2026-11-23: Implemented.

- New integration test `crates/core/tests/compile.rs`: 4-cell matrix (mysql/pg × cache on/off). Each cell generates, asserts expected layers exist (guards against a vacuous empty-module pass), exports via `export_project`, writes a `go.mod`, then runs `go mod tidy` + `go build ./...`. Second test asserts byte-identical output across the matrix (idempotency).
- One command: `cargo test --test compile` (or plain `cargo test`).
- **Bug found & fixed:** repository/service/cache templates referenced bare `User` instead of `model.User` while importing the `model` package — generated code never compiled. `layers.rs` now renders `model.{Struct}` for every model-type reference; goldens + inline assertions regenerated. This is exactly what ticket 10 exists to catch.
- Requires Go on PATH (1.27.3); first run needs network for `go mod tidy` to fetch gorm/go-redis.
- `cargo test` 39 passed (4 suites).

