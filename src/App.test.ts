// @vitest-environment happy-dom
// Mounts the real App.vue to check the wiring between the template and the
// composables. The two Shiki-backed editors are stubbed: highlighting is not
// what these tests are about, and loading it would make them slow and flaky.

import { defineComponent, h } from "vue"
import { flushPromises, mount, type VueWrapper } from "@vue/test-utils"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { invoke } from "@tauri-apps/api/core"

import App from "./App.vue"
import type { GeneratedProject } from "@/lib/ipc"
import { defaults, STORAGE_KEY } from "@/lib/settings"

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))
vi.mock("@tauri-apps/plugin-dialog", () => ({ confirm: vi.fn() }))

const mockedInvoke = vi.mocked(invoke)

const project: GeneratedProject = {
  tables: ["users", "orders"],
  warnings: ["orders.total is nullable"],
  dependencies: ["gorm.io/gorm"],
  files: [
    { path: "models/users.go", content: "package models\n// users", table: "users", kind: "model" },
    {
      path: "repository/users.go",
      content: "package repository\n// users",
      table: "users",
      kind: "repository",
    },
    { path: "models/orders.go", content: "package models\n// orders", table: "orders", kind: "model" },
  ],
}

// The editor stub keeps the v-model contract without Shiki behind it.
const SqlEditorStub = defineComponent({
  props: { modelValue: { type: String, required: true } },
  emits: ["update:modelValue"],
  setup(props, { emit }) {
    return () =>
      h("textarea", {
        class: "stub-sql",
        value: props.modelValue,
        onInput: (event: Event) =>
          emit("update:modelValue", (event.target as HTMLTextAreaElement).value),
      })
  },
})

const CodePreviewStub = defineComponent({
  props: { code: { type: String, required: true }, lang: { type: String, required: true } },
  setup(props) {
    return () => h("pre", { class: "stub-code" }, props.code)
  },
})

let wrapper: VueWrapper
function mountApp() {
  wrapper = mount(App, {
    global: { stubs: { SqlEditor: SqlEditorStub, CodePreview: CodePreviewStub } },
  })
  return wrapper
}

function button(text: string) {
  const found = wrapper.findAll("button").find((b) => b.text() === text)
  if (!found) throw new Error(`no button ${text}`)
  return found
}

function labelInput(text: string) {
  const label = wrapper.findAll("label").find((l) => l.text().trim() === text)
  if (!label) throw new Error(`no label ${text}`)
  return label.find("input")
}

async function generate() {
  await button("Generate").trigger("click")
  await flushPromises()
}

beforeEach(() => {
  localStorage.clear()
  mockedInvoke.mockReset()
})

afterEach(() => {
  wrapper?.unmount()
})

describe("App", () => {
  it("shows the default DDL and the empty state before generating", () => {
    mountApp()

    expect(wrapper.find(".stub-sql").element).toHaveProperty("value", defaults().ddl)
    expect(wrapper.text()).toContain("Paste MySQL DDL and click Generate.")
    expect(button("Generate").attributes("disabled")).toBeUndefined()
  })

  it("renders a generated project: files, tables, dependencies and preview", async () => {
    mockedInvoke.mockResolvedValue(project)
    mountApp()

    await generate()

    expect(mockedInvoke).toHaveBeenCalledWith(
      "generate_project",
      expect.objectContaining({
        request: expect.objectContaining({
          ddl: defaults().ddl,
          dialect: "mysql",
          modulePath: defaults().modulePath,
        }),
      }),
    )
    expect(wrapper.text()).toContain("3 / 3 files")
    expect(wrapper.text()).toContain("1 warning")
    expect(wrapper.find(".stub-code").text()).toBe(project.files[0].content)
    expect(wrapper.text()).toContain("go get gorm.io/gorm")

    const tables = wrapper.findAll("label").filter((l) => ["users", "orders"].includes(l.text().trim()))
    expect(tables).toHaveLength(2)
    for (const label of tables) {
      expect(label.find("input").element).toHaveProperty("checked", true)
    }
  })

  it("shows a failed generate as a toast and inline error, dismissing only the toast", async () => {
    mockedInvoke.mockRejectedValue({ kind: "validation", message: "no tables found" })
    mountApp()

    await generate()

    expect(wrapper.text()).toContain("Invalid input")
    expect(wrapper.text()).toContain("no tables found")

    await wrapper.find('button[aria-label="Dismiss"]').trigger("click")

    expect(wrapper.find('button[aria-label="Dismiss"]').exists()).toBe(false)
    expect(wrapper.text()).toContain("no tables found")
  })

  it("narrows the file list and the preview when a table is unchecked", async () => {
    mockedInvoke.mockResolvedValue(project)
    mountApp()
    await generate()

    await labelInput("users").setValue(false)

    expect(wrapper.text()).toContain("1 / 3 files")
    expect(wrapper.find(".stub-code").text()).toBe(project.files[2].content)
  })

  it("exports the visible files and blocks generate while writing", async () => {
    let releasePicker: (dir: string | null) => void = () => {}
    mockedInvoke.mockImplementation(async (command) => {
      if (command === "generate_project") return project
      if (command === "pick_export_dir") {
        return new Promise<string | null>((resolve) => {
          releasePicker = resolve
        })
      }
      return undefined
    })
    mountApp()
    await generate()

    await button("Export to directory").trigger("click")
    await flushPromises()

    expect(button("Exporting...").exists()).toBe(true)
    expect(button("Generate").attributes("disabled")).toBeDefined()

    releasePicker("/tmp/out")
    await flushPromises()

    expect(mockedInvoke).toHaveBeenCalledWith("export_project", {
      files: project.files,
      overwrite: false,
    })
    expect(wrapper.text()).toContain("Exported 3 file(s) to /tmp/out")
    expect(button("Generate").attributes("disabled")).toBeUndefined()
  })

  it("copies all visible files and flashes the label", async () => {
    const writeText = vi.fn(async () => {})
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true })
    mockedInvoke.mockResolvedValue(project)
    mountApp()
    await generate()

    await button("Copy all").trigger("click")
    await flushPromises()

    expect(writeText).toHaveBeenCalledWith(
      project.files.map((f) => `// ==== ${f.path} ====\n${f.content}`).join("\n"),
    )
    expect(button("Copied!").exists()).toBe(true)
  })

  it("saves an edited setting once the debounce passes", async () => {
    mountApp()

    await labelInput("module path").setValue("example.com/changed")
    // The save is debounced by 300 ms; wait it out rather than fight the app's
    // own timers with fake ones.
    await new Promise((resolve) => setTimeout(resolve, 350))

    expect(JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null")).toMatchObject({
      modulePath: "example.com/changed",
    })
  })
})
