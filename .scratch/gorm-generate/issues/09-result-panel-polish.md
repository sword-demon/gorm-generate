# 09: 结果区完善（高亮、错误、警告、依赖提示）

**What to build:** 代码预览使用 Shiki 做 Go 与 SQL 语法高亮且保持只读；解析错误以 toast 展示并包含出错语句的序号与片段；警告以 Badge/列表呈现；结果区展示需要执行的 go get 依赖（gorm，缓存开启时加 go-redis）。

**Blocked by:** 06

**Status:** ready-for-agent

- [ ] Go 与 SQL 预览有语法高亮且只读
- [ ] 解析错误展示语句序号与片段
- [ ] 警告列表可读，数量可见
- [ ] 结果区显示与当前开关匹配的 go get 依赖提示
