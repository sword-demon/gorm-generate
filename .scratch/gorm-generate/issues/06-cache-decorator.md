# 06: 缓存装饰器

**What to build:** 增加缓存开关与 TTL 配置；开启时额外生成缓存包：装饰器实现与仓储相同的接口，GetByID 走 Redis 缓存（key 为表名加主键值），Update/Delete 删除对应 key，List/Create 不处理缓存，缓存读写失败降级查库且不向上抛错，并提供注入真实仓储与 Redis 客户端的工厂函数；关闭时不生成缓存包。

**Blocked by:** 04

**Status:** resolved

- [x] 缓存开关关闭时不生成任何缓存文件
- [x] 开启后生成装饰器、工厂函数与 TTL 常量
- [x] GetByID 的缓存 key 符合 `表名:主键值` 约定，复合主键按顺序拼接
- [x] Update/Delete 包含缓存失效逻辑
- [x] 缓存错误路径降级查库
- [x] golden 测试覆盖缓存开/关两种产物

## Comments

2026-11-23: Implemented.

- `layers::render_cache` emits `internal/cache/<table>_cache.go`: decorator over the repository interface, `XxxCacheTTL` const, `NewXxxRepoWithCache(repo, rdb)` factory.
- GetByID: Redis GET -> unmarshal hit returns early; miss/error falls through to repo, then best-effort SET with TTL.
- Update/Delete: run repo first, then `_ = rdb.Del(key)` on success (errors swallowed).
- Key: `fmt.Sprintf("tablename:%v[:%v...]", pk...)`; Update extracts PK from `m.FieldName`.
- List/Create pure passthrough. No-PK tables still get no cache file (already skip repo/service).
- 7 new tests + `golden_user_cache.txt`; `cargo test` 31 passed, `npm run build` passed.

