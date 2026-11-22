# 10: 生成物编译验证与全量回归

**What to build:** 端到端测试：对 MySQL 与 PostgreSQL 样例分别以缓存开/关生成产物，写入临时 Go 项目（含 go.mod），执行 `go build ./...` 必须通过；同一输入两次生成结果一致的全矩阵幂等回归同步覆盖。本机 Go 1.27.3 可用。

**Blocked by:** 05, 06

**Status:** ready-for-agent

- [ ] MySQL 样例 × 缓存开/关 生成物可编译
- [ ] PostgreSQL 样例 × 缓存开/关 生成物可编译
- [ ] 全矩阵幂等断言通过
- [ ] 测试可在本机用一条命令运行
