# 缓存只装饰主键读取

## Status

accepted

## Context

缓存开关打开时需要生成缓存代码。可选项包括缓存全部查询、写入时回填、或把缓存塞进 GORM 回调。GORM 官方没有缓存插件。

## Decision

生成 Redis cache-aside 装饰器：仅缓存 `GetByID` 的结果，`Update`/`Delete` 删除对应 key，`List` 不缓存，缓存读写失败一律降级查库。TTL 在生成时配置为常量。装饰器实现与真实仓储相同的接口，key 为 `<表名>:<主键值...>`。

## Considered Options

- **缓存 List**：失效条件依赖过滤参数，无法低成本正确失效，放弃。
- **进程内缓存**：多实例部署下各实例数据不一致，作为默认实现是错的，放弃。
- **GORM 回调/插件**：把缓存耦合进 ORM 层，仓储实现不再纯粹，放弃。

## Consequences

- 缓存故障不会变成业务故障（降级查库）。
- 复合主键按顺序拼进缓存 key。
- 需要 List 缓存的用户自行在 service 层实现。
