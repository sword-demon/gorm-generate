// @vitest-environment happy-dom
// The editor's highlight is debounced; these tests pin when a pass runs and
// that a slow older pass cannot overwrite a newer result.

import { mount } from "@vue/test-utils"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { defineComponent, h, nextTick, ref } from "vue"

import { highlight } from "@/lib/highlight"

import SqlEditor from "./SqlEditor.vue"

vi.mock("@/lib/highlight", () => ({ highlight: vi.fn() }))

const mockedHighlight = vi.mocked(highlight)

// A host keeps the model in one place so the editor behaves as it does in App.
const Host = defineComponent({
  setup() {
    const ddl = ref("CREATE TABLE t (id INT);")
    return () =>
      h(SqlEditor, {
        modelValue: ddl.value,
        "onUpdate:modelValue": (value: string) => (ddl.value = value),
      })
  },
})

function deferred() {
  let resolve: (html: string) => void = () => {}
  const promise = new Promise<string>((r) => (resolve = r))
  return { promise, resolve }
}

beforeEach(() => {
  vi.useFakeTimers()
  mockedHighlight.mockReset()
  mockedHighlight.mockImplementation(async (code) => `<pre>${code}</pre>`)
})

afterEach(() => {
  vi.useRealTimers()
})

describe("SqlEditor", () => {
  it("highlights the initial value without waiting for the debounce", async () => {
    mount(Host)

    expect(mockedHighlight).toHaveBeenCalledTimes(1)
    expect(mockedHighlight).toHaveBeenCalledWith("CREATE TABLE t (id INT);\n", "sql")
  })

  it("debounces typing into one pass", async () => {
    const wrapper = mount(Host)
    mockedHighlight.mockClear()
    const textarea = wrapper.find("textarea")

    await textarea.setValue("CREATE TABLE t (id INT);\n")
    await textarea.setValue("CREATE TABLE t (id INT);\n\n")
    await textarea.setValue("CREATE TABLE t (id BIGINT);")

    expect(mockedHighlight).not.toHaveBeenCalled()

    vi.advanceTimersByTime(120)
    await nextTick()

    expect(mockedHighlight).toHaveBeenCalledTimes(1)
    expect(mockedHighlight).toHaveBeenCalledWith("CREATE TABLE t (id BIGINT);\n", "sql")
  })

  it("keeps the previous colors up until the new pass lands", async () => {
    const wrapper = mount(Host)
    await nextTick()
    const before = wrapper.find(".sql-backdrop").html()

    await wrapper.find("textarea").setValue("CREATE TABLE changed (id INT);")

    expect(wrapper.find(".sql-backdrop").html()).toBe(before)
  })

  it("drops a slow older result instead of letting it overwrite a newer one", async () => {
    const wrapper = mount(Host)
    const slow = deferred()
    mockedHighlight.mockReset()
    mockedHighlight.mockReturnValueOnce(slow.promise)
    mockedHighlight.mockResolvedValueOnce("<pre>fresh</pre>")
    const textarea = wrapper.find("textarea")

    await textarea.setValue("old")
    vi.advanceTimersByTime(120)
    await nextTick()

    await textarea.setValue("new")
    vi.advanceTimersByTime(120)
    await nextTick()
    await nextTick()

    slow.resolve("<pre>stale</pre>")
    await nextTick()
    await nextTick()

    expect(wrapper.find(".sql-backdrop").text()).toBe("fresh")
  })

  it("cancels a pending pass when the editor goes away", async () => {
    const wrapper = mount(Host)
    mockedHighlight.mockClear()

    await wrapper.find("textarea").setValue("CREATE TABLE gone (id INT);")
    wrapper.unmount()
    vi.advanceTimersByTime(500)

    expect(mockedHighlight).not.toHaveBeenCalled()
  })
})
