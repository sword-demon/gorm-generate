import { beforeEach, describe, expect, it, vi } from "vitest"
import { computed, ref } from "vue"

import { invoke } from "@tauri-apps/api/core"
import { confirm } from "@tauri-apps/plugin-dialog"

import type { GeneratedProject } from "@/lib/ipc"

import { useExport } from "./useExport"

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))
vi.mock("@tauri-apps/plugin-dialog", () => ({ confirm: vi.fn() }))

const mockedInvoke = vi.mocked(invoke)
const mockedConfirm = vi.mocked(confirm)

const project: GeneratedProject = {
  tables: ["users"],
  warnings: [],
  dependencies: [],
  files: [
    { path: "models/users.go", content: "package models", table: "users", kind: "model" },
    { path: "repository/users.go", content: "package repository", table: "users", kind: "repository" },
  ],
}

function start(withProject = true) {
  const state = ref<GeneratedProject | null>(withProject ? project : null)
  const visibleFiles = computed(() => state.value?.files ?? [])
  return useExport({ project: state, visibleFiles })
}

beforeEach(() => {
  mockedInvoke.mockReset()
  mockedConfirm.mockReset()
})

describe("useExport", () => {
  it("does nothing without a project", async () => {
    const exporter = start(false)

    await exporter.exportToDir()

    expect(mockedInvoke).not.toHaveBeenCalled()
  })

  it("writes the visible files into the picked directory", async () => {
    mockedInvoke.mockImplementation(async (command) => {
      if (command === "pick_export_dir") return "/tmp/out"
      return undefined
    })
    const exporter = start()

    await exporter.exportToDir()

    expect(mockedInvoke).toHaveBeenNthCalledWith(1, "pick_export_dir")
    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "export_project", {
      files: project.files,
      overwrite: false,
    })
    expect(exporter.exportMsg.value).toBe("Exported 2 file(s) to /tmp/out")
    expect(exporter.exporting.value).toBe(false)
  })

  it("stops quietly when the picker is cancelled", async () => {
    mockedInvoke.mockResolvedValue(null)
    const exporter = start()

    await exporter.exportToDir()

    expect(mockedInvoke).toHaveBeenCalledTimes(1)
    expect(exporter.exportMsg.value).toBe("")
    expect(exporter.exporting.value).toBe(false)
  })

  it("retries with overwrite once the user accepts the conflicts", async () => {
    mockedInvoke.mockImplementation(async (command, args) => {
      if (command === "pick_export_dir") return "/tmp/out"
      if (command === "export_project" && args && !(args as { overwrite: boolean }).overwrite) {
        throw { kind: "conflicts", paths: ["models/users.go"] }
      }
      return undefined
    })
    mockedConfirm.mockResolvedValue(true)
    const exporter = start()

    await exporter.exportToDir()

    expect(mockedConfirm).toHaveBeenCalledWith(
      "1 file(s) already exist and will be overwritten:\n\nmodels/users.go",
    )
    expect(mockedInvoke).toHaveBeenLastCalledWith("export_project", {
      files: project.files,
      overwrite: true,
    })
    expect(exporter.exportMsg.value).toBe("Exported 2 file(s) to /tmp/out")
  })

  it("reports a cancelled overwrite", async () => {
    mockedInvoke.mockImplementation(async (command) => {
      if (command === "pick_export_dir") return "/tmp/out"
      throw { kind: "conflicts", paths: ["models/users.go"] }
    })
    mockedConfirm.mockResolvedValue(false)
    const exporter = start()

    await exporter.exportToDir()

    expect(mockedInvoke).not.toHaveBeenCalledWith("export_project", {
      files: project.files,
      overwrite: true,
    })
    expect(exporter.exportMsg.value).toBe("Export cancelled")
    expect(exporter.exporting.value).toBe(false)
  })

  it("reports other failures without offering to overwrite", async () => {
    mockedInvoke.mockImplementation(async (command) => {
      if (command === "pick_export_dir") return "/tmp/out"
      throw { kind: "io", message: "disk full" }
    })
    const exporter = start()

    await exporter.exportToDir()

    expect(mockedConfirm).not.toHaveBeenCalled()
    expect(exporter.exportMsg.value).toBe("Export failed: write failed: disk full")
    expect(exporter.exporting.value).toBe(false)
  })

  it("reports a picker failure like any other", async () => {
    mockedInvoke.mockRejectedValue("no window")
    const exporter = start()

    await exporter.exportToDir()

    expect(exporter.exportMsg.value).toBe("Export failed: write failed: no window")
    expect(exporter.exporting.value).toBe(false)
  })

  it("clears the status on reset", async () => {
    mockedInvoke.mockResolvedValue(null)
    const exporter = start()
    await exporter.exportToDir()
    exporter.exportMsg.value = "Export cancelled"

    exporter.reset()

    expect(exporter.exportMsg.value).toBe("")
  })
})
