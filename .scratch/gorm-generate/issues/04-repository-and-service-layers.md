# 04: 仓储与服务层 + module path

**What to build:** 增加 module path 输入；生成仓储与服务的接口和实现（接口与实现同文件），接口包含 Create、GetByID、List、Update、Delete 五个方法；主键参数类型跟随 DDL；复合主键生成多参数；无主键表只生成模型并给出警告；服务实现与仓储同签名薄透传；构造函数注入仓储接口；生成代码的 import 路径使用输入的 module path。

**Blocked by:** 03

**Status:** resolved

- [x] 输入 module path 后生成物 import 路径正确
- [x] 每张有主键的表生成仓储接口、实现与构造函数
- [x] 每张有主键的表生成服务接口、实现与构造函数
- [x] List 返回分页结果与总数，按主键升序
- [x] 复合主键表生成多参数方法
- [x] 无主键表只生成模型并出现在警告列表
- [x] golden 测试覆盖单主键、复合主键、无主键三种情况

## Comments

2026-11-23: Implemented.

- Core `layers` module: repository (interface + GORM impl + constructor, `gormUserRepository`-style unexported impl) and service (interface + thin passthrough + constructor injecting `repository.XxxRepository`). Files at `internal/repository/<table>_repo.go` / `internal/service/<table>_service.go`, singular names.
- Contract per spec 仓储契约: Create/GetByID/List/Update/Delete; List returns (items, total, error) ordered by PK ascending with page/pageSize; composite PK -> multi-param methods (`orderId int64, itemId int64`); no-PK tables get model only + warning.
- Service passthrough forwards `ctx` and PK args verbatim (bug caught by seam test: initial version dropped `ctx`).
- module_path drives `<module>/internal/model` and `<module>/internal/repository` imports; Tauri payload already carries it.
- Golden coverage: three locked DDLs — sample (single PK, comments, soft delete), composite PK (`fixtures/composite_pk_ddl.sql`), no PK (`fixtures/no_pk_ddl.sql`), each byte-compared including the no-PK warning assertion.
- Verified: `cargo test` 19 passed; `npm run build` passed.
