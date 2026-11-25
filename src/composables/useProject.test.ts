import { beforeEach, describe, expect, it, vi } from "vitest"
import { ref } from "vue"

import { invoke } from "@tauri-apps/api/core"

import type { GeneratedProject } from "@/lib/ipc"
import type { SettingsRefs } from "./useSettings"

import { useProject } from "./useProject"

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))

const mockedInvoke = vi.mocked(invoke)

function settings(overrides: Partial<Record<keyof SettingsRefs, unknown>> = {}): SettingsRefs {
  return {
    ddl: ref("CREATE TABLE users (id INT);"),
    modulePath: ref("example.com/app"),
    dialect: ref<"mysql" | "postgres">("mysql"),
    tinyintAsBool: ref(false),
    cacheEnabled: ref(false),
    cacheTtlSeconds: ref<number | string>(60),
    ...overrides,
  } as SettingsRefs
}

const project: GeneratedProject = {
  tables: ["users", "orders"],
  warnings: ["users.age is nullable"],
  dependencies: ["gorm.io/gorm"],
  files: [
    { path: "models/users.go", content: "package models", table: "users", kind: "model" },
    { path: "models/orders.go", content: "package models", table: "orders", kind: "model" },
    { path: "repository/users.go", content: "package repository", table: "users", kind: "repository" },
  ],
}

beforeEach(() => {
  mockedInvoke.mockReset()
})

describe("useProject", () => {
  it("sends the settings, with the default TTL when the cache is off", async () => {
    mockedInvoke.mockResolvedValue(project)
    const state = useProject(settings({ cacheEnabled: ref(false), cacheTtlSeconds: ref(60) }), () => false)

    await state.generate()

    expect(mockedInvoke).toHaveBeenCalledWith("generate_project", {
      request: {
        ddl: "CREATE TABLE users (id INT);",
        dialect: "mysql",
        modulePath: "example.com/app",
        cacheEnabled: false,
        cacheTtlSeconds: 300,
        tinyintAsBool: false,
      },
    })
  })

  it("sends the chosen TTL when the cache is on", async () => {
    mockedInvoke.mockResolvedValue(project)
    const state = useProject(settings({ cacheEnabled: ref(true), cacheTtlSeconds: ref(90) }), () => false)

    await state.generate()

    expect(mockedInvoke.mock.calls[0][1]).toMatchObject({
      request: { cacheEnabled: true, cacheTtlSeconds: 90 },
    })
  })

  it("selects every table and opens the first file after generating", async () => {
    mockedInvoke.mockResolvedValue(project)
    const state = useProject(settings(), () => false)

    await state.generate()

    expect(state.project.value).toEqual(project)
    expect(state.selectedTables.value).toEqual(new Set(["users", "orders"]))
    expect(state.selectedPath.value).toBe("models/users.go")
    expect(state.visibleFiles.value).toHaveLength(3)
    expect(state.selectedFile.value?.path).toBe("models/users.go")
    expect(state.dependencies.value).toEqual(["gorm.io/gorm"])
    expect(state.error.value).toBeNull()
    expect(state.pending.value).toBe(false)
  })

  it("records a failed generate as a renderable error", async () => {
    mockedInvoke.mockRejectedValue({ kind: "parse", statementIndex: 2, snippet: "near `users`" })
    const state = useProject(settings(), () => false)

    await state.generate()

    expect(state.project.value).toBeNull()
    expect(state.error.value).toEqual({ kind: "parse", statementIndex: 2, snippet: "near `users`" })
    expect(state.errorTitle.value).toBe("Parse error in statement #2")
    expect(state.errorDetail.value).toBe("near `users`")
    expect(state.showErrorToast.value).toBe(true)
    expect(state.pending.value).toBe(false)
  })

  it("hides the toast once dismissed but keeps the inline error", async () => {
    mockedInvoke.mockRejectedValue({ kind: "validation", message: "no tables" })
    const state = useProject(settings(), () => false)
    await state.generate()

    state.dismissToast()

    expect(state.showErrorToast.value).toBe(false)
    expect(state.error.value).toEqual({ kind: "validation", message: "no tables" })
  })

  it("moves the preview off a file whose table is deselected", async () => {
    mockedInvoke.mockResolvedValue(project)
    const state = useProject(settings(), () => false)
    await state.generate()

    state.toggleTable("users")

    expect(state.selectedTables.value).toEqual(new Set(["orders"]))
    expect(state.visibleFiles.value.map((f) => f.path)).toEqual(["models/orders.go"])
    expect(state.selectedPath.value).toBe("models/orders.go")
  })

  it("restores a table and keeps the current file while it stays visible", async () => {
    mockedInvoke.mockResolvedValue(project)
    const state = useProject(settings(), () => false)
    await state.generate()

    state.toggleTable("orders")
    state.toggleTable("orders")

    expect(state.selectedTables.value).toEqual(new Set(["users", "orders"]))
    expect(state.selectedPath.value).toBe("models/users.go")
  })

  it("concatenates the visible files for copy-all", async () => {
    mockedInvoke.mockResolvedValue(project)
    const state = useProject(settings(), () => false)
    await state.generate()
    state.toggleTable("orders")

    expect(state.allContent.value).toBe(
      [
        "// ==== models/users.go ====\npackage models",
        "// ==== repository/users.go ====\npackage repository",
      ].join("\n"),
    )
  })

  it("does nothing while an export is writing", async () => {
    const state = useProject(settings(), () => true)

    await state.generate()

    expect(mockedInvoke).not.toHaveBeenCalled()
  })

  it("does nothing while a generate is already pending", async () => {
    let finish: (value: GeneratedProject) => void = () => {}
    mockedInvoke.mockImplementation(() => new Promise((resolve) => (finish = resolve)))
    const state = useProject(settings(), () => false)

    const first = state.generate()
    await state.generate()

    expect(mockedInvoke).toHaveBeenCalledTimes(1)
    finish(project)
    await first
  })
})
