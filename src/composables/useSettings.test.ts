// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { effectScope, nextTick } from "vue"

import { defaults, STORAGE_KEY } from "@/lib/settings"

import { useSettings } from "./useSettings"

function start() {
  const scope = effectScope()
  const settings = scope.run(useSettings)!
  return { settings, stop: () => scope.stop() }
}

function stored(): unknown {
  return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null")
}

beforeEach(() => {
  localStorage.clear()
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe("useSettings", () => {
  it("starts from the defaults without stored state", () => {
    const { settings, stop } = start()
    expect(settings.ddl.value).toBe(defaults().ddl)
    expect(settings.modulePath.value).toBe(defaults().modulePath)
    expect(settings.dialect.value).toBe("mysql")
    expect(settings.cacheEnabled.value).toBe(false)
    expect(settings.cacheTtlSeconds.value).toBe(300)
    expect(settings.tinyintAsBool.value).toBe(false)
    expect(settings.panePercent.value).toBe(defaults().panePercent)
    stop()
  })

  it("starts from stored state", () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        ddl: "CREATE TABLE t (id INT);",
        dialect: "postgres",
        modulePath: "example.com/other",
        cacheEnabled: true,
        cacheTtlSeconds: 60,
        tinyintAsBool: true,
        panePercent: 35,
      }),
    )
    const { settings, stop } = start()
    expect(settings.ddl.value).toBe("CREATE TABLE t (id INT);")
    expect(settings.dialect.value).toBe("postgres")
    expect(settings.modulePath.value).toBe("example.com/other")
    expect(settings.cacheEnabled.value).toBe(true)
    expect(settings.cacheTtlSeconds.value).toBe(60)
    expect(settings.tinyintAsBool.value).toBe(true)
    expect(settings.panePercent.value).toBe(35)
    stop()
  })

  it("debounces saves while typing", async () => {
    const { settings, stop } = start()
    settings.ddl.value = "CREATE TABLE a (id INT);"
    settings.ddl.value = "CREATE TABLE b (id INT);"
    await nextTick()

    vi.advanceTimersByTime(299)
    expect(stored()).toBeNull()

    vi.advanceTimersByTime(1)
    expect(stored()).toMatchObject({ ddl: "CREATE TABLE b (id INT);" })
    stop()
  })

  it("saves every field", async () => {
    const { settings, stop } = start()
    settings.ddl.value = "CREATE TABLE t (id INT);"
    settings.modulePath.value = "example.com/app"
    settings.dialect.value = "postgres"
    settings.tinyintAsBool.value = true
    settings.cacheEnabled.value = true
    settings.cacheTtlSeconds.value = 60
    settings.panePercent.value = 35
    await nextTick()
    vi.advanceTimersByTime(300)

    expect(stored()).toEqual({
      ddl: "CREATE TABLE t (id INT);",
      modulePath: "example.com/app",
      dialect: "postgres",
      tinyintAsBool: true,
      cacheEnabled: true,
      cacheTtlSeconds: 60,
      panePercent: 35,
    })
    stop()
  })

  it("flushes on page hide", async () => {
    const { settings, stop } = start()
    settings.ddl.value = "CREATE TABLE t (id INT);"
    await nextTick()

    window.dispatchEvent(new Event("pagehide"))

    expect(stored()).toMatchObject({ ddl: "CREATE TABLE t (id INT);" })
    stop()
  })

  it("flushes and unhooks when its scope stops", async () => {
    const { settings, stop } = start()
    settings.ddl.value = "CREATE TABLE t (id INT);"
    await nextTick()

    stop()

    expect(stored()).toMatchObject({ ddl: "CREATE TABLE t (id INT);" })

    localStorage.clear()
    window.dispatchEvent(new Event("pagehide"))
    expect(stored()).toBeNull()
  })

  it("reports a storage failure without throwing", async () => {
    const { settings, stop } = start()
    vi.spyOn(localStorage, "setItem").mockImplementation(() => {
      throw new Error("quota exceeded")
    })

    settings.ddl.value = "CREATE TABLE t (id INT);"
    await nextTick()
    vi.advanceTimersByTime(300)

    expect(settings.persistenceError.value).toContain("could not be saved")
    stop()
  })
})
