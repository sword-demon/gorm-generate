# 20: 不提供 Apple 签名,维持未签名分发

**What to build:** 不配置 Apple Developer ID 签名与公证，Release 继续发布未签名、未公证的 macOS 产物；把「用户如何首次打开未签名 App」写清楚，作为正式的分发方式而不是遗留缺口。

**Status:** resolved
**Category:** enhancement

**Blocked by:** None

**Why:** 项目没有 Apple Developer Program 账号，拿不到 Developer ID 证书，签名与公证这条路走不通。与其把「未签名」挂在待办里，不如把它定成正式的分发策略：用户在 Release 下载对应架构的 `.dmg`，拖进 Applications 后用 Control-点击「打开」（或「隐私与安全性 → 仍要打开」）放行。

**推翻了什么:** 此前 HANDOFF 把「产物未签名、未公证」列在「未验证」里，README「发布」也留着 Developer ID secrets 表格，读起来像是迟早要补的事。现在它是一个已定的决策。

## Steps

1. README「发布」新增「首次打开未签名 App」：选架构、拖到 Applications、Control-点击「打开」、「隐私与安全性 → 仍要打开」的兜底，以及只从本项目 Release 下载的提醒。
2. README 移除 Developer ID secrets 表格，改为说明「本项目不配置 Apple Developer ID 证书」。
3. `.github/workflows/release.yml` 移除签名/公证的环境变量与 `APPLE_*` secrets 读取，末尾只打印 `signed=no notarized=no` 与一条说明性 warning。
4. HANDOFF 把签名条目从「未验证」移出，改记为已知的产品分发约束。
5. `spec.md` 的 Out of Scope 本就列了「打包签名与自动更新」，保持不变。

## 验收

- [x] README 有可照做的首次打开步骤
- [x] workflow 不再引用任何 `APPLE_*` secret，仍照常发布两个架构的 `.dmg`
- [x] `release.yml` YAML 解析通过
- [x] HANDOFF 不再把签名列为待办
- [x] spec 的 Out of Scope 覆盖打包签名

## Comments

2026-11-26: 落地，提交 `90a9c90`。用户确认拿不到 Apple 签名，因此按未签名分发推进。
