# GORM 代码生成器

将 SQL DDL 脚本转换为 Go GORM 分层代码的桌面工具。本词汇表固定设计讨论中的用词。

## Language

**DDL 脚本**:
用户提供的一段或多段建表语句及其补充语句（如 `COMMENT ON`），是生成的唯一事实源。
_Avoid_: SQL 语句（过宽，含 DML）、建表脚本

**方言**:
DDL 脚本所属的数据库类型，本项目仅指 MySQL 或 PostgreSQL。
_Avoid_: 数据库类型、db type

**生成产物**:
一次生成输出的全部文件及其目录结构。
_Avoid_: 输出、结果

**模型 (Model)**:
一张表在 Go 中的结构体表示，含 GORM 标签与表名方法。
_Avoid_: 实体 (Entity)、PO

**仓储 (Repository)**:
一张表的持久化访问接口及其 GORM 实现，不含业务规则。
_Avoid_: DAO、Mapper、Store

**服务 (Service)**:
业务层接口及其实现，默认与仓储同签名的薄透传。
_Avoid_: Logic、UseCase、Biz

**缓存装饰器 (Cache Decorator)**:
实现同一仓储接口、在真实仓储外增加缓存读写与失效逻辑的实现。
_Avoid_: 缓存层、Cache Repo

**软删除**:
由 `deleted_at` 列表达的删除语义：删除写时间戳，查询自动过滤。
_Avoid_: 逻辑删除

**警告**:
生成时不影响文件产出、但需要用户注意的问题（如不支持的列类型）。
_Avoid_: 错误、Error
