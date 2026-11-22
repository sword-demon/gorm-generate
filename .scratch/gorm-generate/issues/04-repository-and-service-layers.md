# 04: 仓储与服务层 + module path

**What to build:** 增加 module path 输入；生成仓储与服务的接口和实现（接口与实现同文件），接口包含 Create、GetByID、List、Update、Delete 五个方法；主键参数类型跟随 DDL；复合主键生成多参数；无主键表只生成模型并给出警告；服务实现与仓储同签名薄透传；构造函数注入仓储接口；生成代码的 import 路径使用输入的 module path。

**Blocked by:** 03

**Status:** ready-for-agent

- [ ] 输入 module path 后生成物 import 路径正确
- [ ] 每张有主键的表生成仓储接口、实现与构造函数
- [ ] 每张有主键的表生成服务接口、实现与构造函数
- [ ] List 返回分页结果与总数，按主键升序
- [ ] 复合主键表生成多参数方法
- [ ] 无主键表只生成模型并出现在警告列表
- [ ] golden 测试覆盖单主键、复合主键、无主键三种情况
