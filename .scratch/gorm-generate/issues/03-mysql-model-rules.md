# 03: MySQL 模型完整规则

**What to build:** 把 MySQL 模型生成补全到 spec 约定的全部规则：可空列指针、decimal/numeric 用 string、json 用 RawMessage、enum/set 用 string、二进制用 []byte、时间用 time.Time、unsigned 按宽度、未知类型用 any；deleted_at 生成软删除类型；表/列注释变成 Go 注释；tinyint(1) 默认整型、界面开关切换为 bool；生成文件带 DO NOT EDIT 头；未知类型与被忽略语句进入警告列表并在界面可见；相同输入连续生成结果一致。

**Blocked by:** 02

**Status:** ready-for-agent

- [ ] 复杂表 DDL（含各类类型、可空、注释、软删除）生成的模型符合 spec 的类型映射表
- [ ] tinyint(1) 默认按 `int8`/`uint8`，打开开关后为 `bool`
- [ ] 表注释与列注释出现在生成代码中
- [ ] 未知类型生成 `any` 并出现在警告列表，界面可看到警告
- [ ] 非建表语句被忽略并计数进警告
- [ ] 生成文件带 DO NOT EDIT 头
- [ ] 幂等测试：同一输入连续两次生成结果完全一致
