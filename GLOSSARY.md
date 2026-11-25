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

**已解析表 (Resolved Table)**:
一张 DDL 表在 Go 侧的事实集合：结构体名、文件基名、实现类型基名、字段（Go 名、Go 类型、gorm 标记、原始注释）、主键索引、条件 import、缓存 key 格式与该表产生的警告。渲染器只读它，不再自行推导类型或命名。
_Avoid_: 表模型、TableShape

**模型 (Model)**:
一张表在 Go 中的结构体表示，含 GORM 标签与表名方法。
_Avoid_: 实体 (Entity)、PO

**仓储 (Repository)**:
一张表的持久化访问接口及其 GORM 实现，不含业务规则。
_Avoid_: DAO、Mapper、Store

**服务 (Service)**:
业务层接口及其实现，默认与仓储同签名的薄透传。
_Avoid_: Logic、UseCase、Biz

**层契约 (Layer Contract)**:
仓储、服务与缓存装饰器共用的五个方法（`Create` / `GetByID` / `List` / `Update` / `Delete`）的签名与主键参数表达式。由一个 crate 私有的契约模块从已解析表一次算出；三层渲染器只负责各自的函数体与 import 块。
_Avoid_: 接口模板、方法表

**缓存装饰器 (Cache Decorator)**:
实现同一仓储接口、在真实仓储外增加缓存读写与失效逻辑的实现。
_Avoid_: 缓存层、Cache Repo

**文件种类 (File Kind)**:
生成产物中一个文件所属的层：模型、仓储、服务、缓存装饰器。由产出它的渲染器声明；调用方按（表名, 文件种类）取文件，不依赖文件顺序或路径字符串。
_Avoid_: 文件类型、category

**写盘计划 (Write Plan)**:
一次导出中每个文件的去向与内容：根目录、项目相对路径、解析后的绝对目标与字节。由纯函数 `plan_export` 产出，不读文件系统；符号链接、冲突与落盘由写盘适配器 `write_plan` 负责。
_Avoid_: 导出列表、文件清单

**软删除**:
由 `deleted_at` 列表达的删除语义：删除写时间戳，查询自动过滤。
_Avoid_: 逻辑删除

**警告**:
生成时不影响文件产出、但需要用户注意的问题（如不支持的列类型）。
_Avoid_: 错误、Error
