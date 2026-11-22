# 06: 缓存装饰器

**What to build:** 增加缓存开关与 TTL 配置；开启时额外生成缓存包：装饰器实现与仓储相同的接口，GetByID 走 Redis 缓存（key 为表名加主键值），Update/Delete 删除对应 key，List/Create 不处理缓存，缓存读写失败降级查库且不向上抛错，并提供注入真实仓储与 Redis 客户端的工厂函数；关闭时不生成缓存包。

**Blocked by:** 04

**Status:** ready-for-agent

- [ ] 缓存开关关闭时不生成任何缓存文件
- [ ] 开启后生成装饰器、工厂函数与 TTL 常量
- [ ] GetByID 的缓存 key 符合 `表名:主键值` 约定，复合主键按顺序拼接
- [ ] Update/Delete 包含缓存失效逻辑
- [ ] 缓存错误路径降级查库
- [ ] golden 测试覆盖缓存开/关两种产物
