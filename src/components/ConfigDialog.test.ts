// @vitest-environment happy-dom
import { mount } from "@vue/test-utils"
import { describe, expect, it } from "vitest"
import { defineComponent, h, ref } from "vue"

import ConfigDialog from "./ConfigDialog.vue"

/** Mounts the dialog with a parent that owns the settings. */
function mountDialog() {
  const state = {
    open: ref(true),
    dialect: ref<"mysql" | "postgres">("mysql"),
    modulePath: ref("github.com/example/app"),
    tinyintAsBool: ref(false),
    cacheEnabled: ref(false),
    cacheTtlSeconds: ref<number | string>(300),
  }
  const Host = defineComponent({
    setup() {
      return () =>
        h(ConfigDialog, {
          open: state.open.value,
          "onUpdate:open": (v: boolean) => (state.open.value = v),
          dialect: state.dialect.value,
          "onUpdate:dialect": (v: "mysql" | "postgres") => (state.dialect.value = v),
          modulePath: state.modulePath.value,
          "onUpdate:modulePath": (v: string) => (state.modulePath.value = v),
          tinyintAsBool: state.tinyintAsBool.value,
          "onUpdate:tinyintAsBool": (v: boolean) => (state.tinyintAsBool.value = v),
          cacheEnabled: state.cacheEnabled.value,
          "onUpdate:cacheEnabled": (v: boolean) => (state.cacheEnabled.value = v),
          cacheTtlSeconds: state.cacheTtlSeconds.value,
          "onUpdate:cacheTtlSeconds": (v: number | string) => (state.cacheTtlSeconds.value = v),
        })
    },
  })
  const wrapper = mount(Host)
  return { wrapper, state }
}

describe("ConfigDialog", () => {
  it("shows the current settings", () => {
    const { wrapper } = mountDialog()

    expect(wrapper.find('[role="dialog"]').exists()).toBe(true)
    expect(wrapper.find('select').element).toHaveProperty("value", "mysql")
    expect(wrapper.find('input[type="text"], input:not([type])').element).toHaveProperty(
      "value",
      "github.com/example/app",
    )
  })

  it("edits the settings through the parent", async () => {
    const { wrapper, state } = mountDialog()

    await wrapper.find("select").setValue("postgres")
    await wrapper.findAll("input[type=checkbox]")[1].setValue(true)

    expect(state.dialect.value).toBe("postgres")
    expect(state.cacheEnabled.value).toBe(true)
  })

  it("shows the TTL field only when the cache is enabled", async () => {
    const { wrapper } = mountDialog()

    expect(wrapper.find('input[type="number"]').exists()).toBe(false)

    await wrapper.findAll("input[type=checkbox]")[1].setValue(true)

    expect(wrapper.find('input[type="number"]').exists()).toBe(true)
  })

  it("closes on Escape and on a backdrop click", async () => {
    const { wrapper, state } = mountDialog()

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }))
    await wrapper.vm.$nextTick()
    expect(state.open.value).toBe(false)

    state.open.value = true
    await wrapper.vm.$nextTick()
    await wrapper.find(".fixed.inset-0").trigger("click.self")
    expect(state.open.value).toBe(false)
  })
})
