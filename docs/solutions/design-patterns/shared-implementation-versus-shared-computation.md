---
title: 共享实现不等于共享计算
date: 2026-11-26
category: design-patterns
module: core rendering
problem_type: architecture_pattern
component: service_layer
severity: low
applies_when:
  - 多个渲染器、适配器或服务消费同一组派生事实
  - 已有共享 helper 或抽象，但调用方仍分别构造或格式化相同数据
  - 输出测试通过，但需要确认重复计算或实现漂移是否仍然存在
tags:
  - shared-computation
  - renderer-contract
  - single-owner
  - golden-tests
---

# 共享实现不等于共享计算

## Context

在 PR #1 之前的重构中，仓储、服务和缓存共用的五方法契约已收敛到 `Contract`，但三个渲染器仍各自调用 `Contract::new`；PR #1 已将构造上移到 `render::table`，使每张表只计算一次。共享类型解决了重复实现，却没有自动保证共享计算。相同的重复工作也曾出现在主键 import 片段、解析错误片段、标识符提取、主键 membership 和前端事件监听清理中。

## Guidance

先区分两件事：**共享实现**意味着多个调用方使用同一个 helper 或类型；**共享计算**意味着共同的派生事实在共同的上游边界计算一次，再以只读结果传给下游。

在本项目中，`render::table` 是一张表的渲染编排边界。它在需要分层渲染时构造一次 `Contract`，再把 `&Contract` 传给 repository、service 和 cache renderer；各 renderer 只保留自己的输出格式和函数体。共享的 PK import 格式化也由一个 helper 负责，而不是在两个模板中手写相同分支。

同样的判断适用于其他重复工作：只在错误路径使用的数据应延迟到错误分支构造；跨模块重复的纯 helper 应有一个 owner；循环中的 membership 判断应使用与数据规模匹配的结构；事件监听器的注册和清理应复用同一组函数引用。

## Why This Matters

重复计算会让性能收益和一致性风险隐藏在“输出完全相同”的表象下。它还会让未来的改动只更新其中一个调用方，造成层之间漂移。共享计算边界能同时减少工作量和可变的实现数量，但必须保留每个 renderer 对自身输出的所有权。

黄金输出测试只能证明生成的字节没有变化，不能证明内部只计算了一次，也不能发现某个重复 helper 已经成为漂移源。因此，重构时应同时检查调用图和派生事实的 owner，并为共享契约保留直接的单元测试；黄金测试负责证明外部输出保持兼容。

## When to Apply

- 看到多个调用方分别构造同一派生对象，尤其是在同一张表、请求或任务的处理链中。
- 看到多个模板分别格式化同一组选项、参数或 import。
- 看到测试只比较最终字符串，却声称验证内部去重、单次计算或共享 owner。
- 看到生命周期 API 用不同包装函数注册和移除同一类事件监听器。

## Examples

Before (abridged): each renderer constructed the same contract independently:

```rust
fn repository(table: &Table, request: &GenerateRequest) -> GeneratedFile {
    let contract = Contract::new(table);
    // render repository using contract and request
}

fn service(table: &Table, request: &GenerateRequest) -> GeneratedFile {
    let contract = Contract::new(table);
    // render service using contract and request
}
```

After: the common boundary constructs it once and collects each renderer's output:

```rust
fn table(table: &Table, request: &GenerateRequest) -> TableOutput {
    let contract = Contract::new(table);
    output.files.push(repository(table, request, &contract));
    output.files.push(service(table, request, &contract));
}
```

The generated output should still be checked with golden or snapshot tests, while the contract's method order, composite-key arguments, and reserved-word handling should be checked directly at the contract boundary.

## Related

- [ADR-0003: 渲染前先解析成已解析表](../../adr/0003-resolved-table-before-rendering.md)
- [ADR-0008: 渲染器拥有每张表的产出](../../adr/0008-renderers-own-each-tables-output.md)
- [GLOSSARY.md](../../../GLOSSARY.md)
- PR [sword-demon/gorm-generate#1](https://github.com/sword-demon/gorm-generate/pull/1)
