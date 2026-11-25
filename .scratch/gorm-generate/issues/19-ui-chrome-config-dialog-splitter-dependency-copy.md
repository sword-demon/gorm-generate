# 19: 标题栏收进配置表单、加可拖动分隔条、依赖一键复制

**What to build:** 标题栏的方言、module path、`tinyint(1)` 开关、Redis 缓存开关与 TTL 占掉两行，改为最右端一个配置按钮打开表单；SQL DDL 与右侧结果之间加一条可拖动的分隔条；底部依赖支持一键复制对应的 `go get` 命令。

**Status:** resolved
**Category:** enhancement

**Blocked by:** None

**Why:** 标题栏在窄窗口里折成两行，把「生成」挤到第二行；两栏各占 50% 是写死的，长 SQL 与宽预览没法同时看；依赖只以静态文字展示，读者得自己把 `gorm.io/gorm` 拼成 `go get gorm.io/gorm` 再敲。

## Steps

1. 新增 `src/components/ConfigDialog.vue`：五个选项收进一个模态表单（`role="dialog"`、Esc 关闭、点遮罩关闭、打开时聚焦 module path、关闭时归还焦点），字段用 `defineModel` 直接绑 `useSettings` 的 ref，持久化行为不变。
2. 标题栏只留标题、当前配置摘要（方言 · module path · 开关摘要）、生成按钮与最右端的齿轮按钮。
3. 新增 `src/composables/usePaneSplit.ts`：分隔条按容器宽度百分比计算，拖动时在 window 上监听 pointermove（不依赖 pointer capture），方向键 ±2%（Shift ±10%），夹在 15%–85%。宽度存进 localStorage 的 `panePercent`。
4. 分隔条用 `role="separator"` + `aria-orientation="vertical"` + `aria-valuenow` 暴露给辅助技术。
5. 依赖行：每条 `go get …` 变成按钮，点击复制整条命令并打勾；再加一个「Copy all commands」整组复制。

## 验收

- [x] 标题栏不再换行，窄窗口下「生成」仍在第一行
- [x] 配置表单里的改动立刻生效（生成请求带上新值）并照旧持久化
- [x] 分隔条可拖动、可键盘调整，宽度刷新后还在
- [x] 点依赖即复制 `go get <dep>`，整组可一次复制
- [x] `npm run typecheck` / `npm test`（83 passed）/ `npm run build` 通过
- [x] 浏览器 dev server 里实测：头部单行、勾 Redis 后出现 TTL、拖到 35% 后宽度与 localStorage 一致

## Comments

2026-11-26: 落地。浏览器 dev server 与挂载测试通过。

2026-11-26: 用户在真实 macOS `.app` 中完成验收：配置表单、分隔条拖动、单条依赖复制、Copy all、Export to directory 均成功。此前 agent 的合成点击受 WKWebView 和辅助功能权限限制,不代表应用限制。
