---
title: Tauri 前端的浏览器 E2E 只打桩 IPC 边界
date: 2026-11-27
category: testing
module: frontend
problem_type: testing_strategy
component: e2e
severity: low
applies_when:
  - 桌面壳（Tauri / Electron）的界面逻辑缺少端到端覆盖，只有组件挂载测试
  - 想在现有 CI 的浏览器环境里跑真实页面，而不是启动整台桌面应用
  - 新增第二套测试文件，既有运行器可能把它们一起收集
tags:
  - playwright
  - tauri
  - ipc-stub
  - vitest-boundary
  - browser-e2e
---

# Tauri 前端的浏览器 E2E 只打桩 IPC 边界

## Context

项目是 Tauri 2 桌面应用：Rust 侧解析与生成，Vue 3 界面通过 `@tauri-apps/api/core` 的 `invoke` 调用 `generate_project` 等命令。挂载测试（`src/App.test.ts`）覆盖了模板接线，但真实页面里的启动、生成、表过滤、设置持久化没有自动覆盖；导出写盘也只能人工冒烟。PR #20 加了 4 个 Playwright 用例，跑在 CI 的 `frontend` job 里。

## Guidance

不要在浏览器里模拟整个 Tauri，只替换它唯一的出口：`@tauri-apps/api/core` 最终调用的是 `window.__TAURI_INTERNALS__.invoke`。用 `page.addInitScript` 在页面脚本执行前放入一个 `invoke`，命令名以外的调用直接抛错（顺便证明界面没有调用未预期的命令）：

```ts
await page.addInitScript(
  ({ result, failure }) => {
    const w = window as unknown as { __TAURI_INTERNALS__: unknown }
    w.__TAURI_INTERNALS__ = {
      invoke: async (cmd: string) => {
        if (cmd !== "generate_project") throw new Error(`unexpected Tauri command: ${cmd}`)
        if (failure) throw failure
        return result
      },
    }
  },
  { result: project, failure: null },
)
```

被替换的只有这一个边界：真实 composable、`src/lib/ipc.ts` 的错误收窄、模板与样式照常运行。断言用可访问性查询（`getByRole` / `getByLabel` / `getByText`），只有在没有语义节点时才退回到 scoped 的 CSS 选择器（例如 `.code-preview pre`，避开 SQL 编辑器的高亮背面）。

服务端由 Playwright 自己起：`webServer.command: "npm run dev"`，端口与 Tauri 的 `devUrl` 一致（1420），本地复用已开着的 dev server，CI 里每次新起。

**给两套运行器划界。** 引入 `e2e/*.spec.ts` 的同时必须在 Vitest 配置里排除它们，否则 `npm test` 会收集 Playwright 的 `test()` 并在 CI 单测步骤失败（见 `tasks/lessons.md` 第 11 条）：

```ts
test: { exclude: [...configDefaults.exclude, "e2e/**"] }
```

CI 侧只需两步：`npx playwright install --with-deps chromium` 与 `npm run test:e2e`，不额外占一个 job。

## Why This Matters

挂载测试证明组件能装配，但不能证明应用能启动、生成按钮真的连到 composable、刷新后 localStorage 读回正确。浏览器 E2E 用真实页面补上这一段，同时避开 Tauri 桌面壳在 runner 上的不稳定因素（GUI 依赖、权限、选择器）。因为桩只在一处，Rust 侧的线格式不会被复制到前端测试里 —— 它的契约由 `wire_format` 测试与 `src/lib/ipc.ts` 的收窄共同约束；代价是桩对参数不敏感（调用 `invoke` 时传了什么不校验），这一层仍靠单元测试。

## When to Apply

- 桌面应用的前端逻辑值得端到端覆盖，但整机 GUI 冒烟在 CI 上不可行。
- 已有的单元测试已经把纯逻辑覆盖得很好，缺的是「页面真的能启动并跑通主路径」。
- 准备新增第二种测试文件类型时，顺手确认既有运行器的收集范围。

## Examples

四个用例覆盖五条用户可见的行为：启动即带示例、生成后文件计数与预览、取消勾选表后计数变化、设置跨刷新持久化、方言不匹配的提示措辞。失败路径同样打桩：`invoke` 抛出 `{ kind: "dialectMismatch", statementIndex, message, snippet }`，断言界面显示 `Dialect mismatch in statement #1`，而不是解析错误。

## Related

- [`e2e/app.spec.ts`](../../../e2e/app.spec.ts)
- [`playwright.config.ts`](../../../playwright.config.ts)
- [`vite.config.ts`](../../../vite.config.ts)（Vitest 的 `e2e/**` 排除）
- [`tasks/lessons.md`](../../../tasks/lessons.md) 第 11 条
- PR [sword-demon/gorm-generate#20](https://github.com/sword-demon/gorm-generate/pull/20)
