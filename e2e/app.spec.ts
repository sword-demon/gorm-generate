import { expect, test, type Page } from "@playwright/test"

import type { GeneratedProject } from "../src/lib/ipc"

// `@tauri-apps/api/core` calls `window.__TAURI_INTERNALS__.invoke`, so the
// stub below replaces only the Tauri commands: the real composables, the IPC
// narrowing in src/lib/ipc.ts and every template run unchanged.
const project: GeneratedProject = {
  tables: ["users", "orders"],
  warnings: ["orders.total is nullable"],
  dependencies: ["gorm.io/gorm"],
  files: [
    {
      path: "internal/model/users.go",
      content: "package model\n\n// users",
      table: "users",
      kind: "model",
    },
    {
      path: "internal/repository/users_repo.go",
      content: "package repository\n\n// users",
      table: "users",
      kind: "repository",
    },
    {
      path: "internal/model/orders.go",
      content: "package model\n\n// orders",
      table: "orders",
      kind: "model",
    },
  ],
}

interface StubOptions {
  failure?: unknown
}

async function stubTauri(page: Page, options: StubOptions = {}) {
  await page.addInitScript(
    ({ result, failure }) => {
      const w = window as unknown as { __TAURI_INTERNALS__: unknown }
      w.__TAURI_INTERNALS__ = {
        invoke: async (cmd: string) => {
          if (cmd !== "generate_project") {
            throw new Error(`unexpected Tauri command: ${cmd}`)
          }
          if (failure) throw failure
          return result
        },
      }
    },
    { result: project, failure: options.failure ?? null },
  )
}

test("boots with the MySQL sample and an empty result pane", async ({ page }) => {
  await stubTauri(page)
  await page.goto("/")

  await expect(page.getByRole("heading", { name: "GORM Code Generator" })).toBeVisible()
  await expect(page.getByLabel("SQL DDL input")).toHaveValue(/CREATE TABLE `users`/)
  await expect(page.getByText("Paste MySQL DDL and click Generate.")).toBeVisible()
})

test("generates a project, previews a file and filters by table", async ({ page }) => {
  await stubTauri(page)
  await page.goto("/")
  await page.getByRole("button", { name: "Generate", exact: true }).click()

  await expect(page.getByText("3 / 3 files")).toBeVisible()
  await expect(
    page.getByRole("button", { name: "internal/model/users.go" }),
  ).toBeVisible()
  // The first file is selected after a generate, so its content is rendered.
  await expect(page.locator(".code-preview pre")).toContainText("package model")

  await page.getByLabel("orders").uncheck()
  await expect(page.getByText("2 / 3 files")).toBeVisible()
  await expect(
    page.getByRole("button", { name: "internal/model/orders.go" }),
  ).toHaveCount(0)
})

test("settings survive a reload", async ({ page }) => {
  await stubTauri(page)
  await page.goto("/")

  await page.getByRole("button", { name: "Configuration" }).click()
  await page.getByLabel("Module path").fill("example.com/e2e")
  await page.keyboard.press("Escape")
  await expect(page.getByText("MySQL · example.com/e2e")).toBeVisible()

  await page.reload()
  await expect(page.getByText("MySQL · example.com/e2e")).toBeVisible()
})

test("a dialect mismatch is reported as such, not as a parser error", async ({ page }) => {
  await stubTauri(page, {
    failure: {
      kind: "dialectMismatch",
      statementIndex: 1,
      message: "This script looks like MySQL DDL; switch the dialect.",
      snippet: "CREATE TABLE `users` (",
    },
  })
  await page.goto("/")
  await page.getByRole("button", { name: "Generate", exact: true }).click()

  const alerts = page.getByRole("alert")
  await expect(alerts.first()).toContainText("Dialect mismatch in statement #1")
  await expect(alerts.first()).toContainText("This script looks like MySQL DDL")
})
