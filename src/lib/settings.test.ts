import { describe, expect, it } from "vitest"

import { defaults, defaultDdl, loadState, MAX_PANE_PERCENT, MIN_PANE_PERCENT } from "./settings"

describe("loadState", () => {
  it("returns the defaults without stored state", () => {
    expect(loadState(null)).toEqual(defaults())
    expect(loadState("")).toEqual(defaults())
  })

  it("returns the defaults for corrupt json", () => {
    expect(loadState("{not json")).toEqual(defaults())
    expect(loadState("null")).toEqual(defaults())
  })

  it("keeps every valid field", () => {
    const stored = {
      ddl: "CREATE TABLE t (id INT);",
      dialect: "postgres",
      modulePath: "example.com/other",
      cacheEnabled: true,
      cacheTtlSeconds: 60,
      tinyintAsBool: true,
      panePercent: 40,
    }
    expect(loadState(JSON.stringify(stored))).toEqual(stored)
  })

  it("falls back per field, not wholesale", () => {
    const base = defaults()
    const state = loadState(
      JSON.stringify({
        ddl: "CREATE TABLE kept (id INT);",
        dialect: "sqlite",
        modulePath: 42,
        cacheEnabled: "yes",
        cacheTtlSeconds: 0,
        tinyintAsBool: 1,
        panePercent: "wide",
      }),
    )
    expect(state.ddl).toBe("CREATE TABLE kept (id INT);")
    expect(state.dialect).toBe("mysql")
    expect(state.modulePath).toBe(base.modulePath)
    expect(state.cacheEnabled).toBe(false)
    expect(state.cacheTtlSeconds).toBe(300)
    expect(state.tinyintAsBool).toBe(false)
    expect(state.panePercent).toBe(base.panePercent)
  })

  it("clamps the stored pane width", () => {
    expect(loadState(JSON.stringify({ panePercent: 2 })).panePercent).toBe(MIN_PANE_PERCENT)
    expect(loadState(JSON.stringify({ panePercent: 99 })).panePercent).toBe(MAX_PANE_PERCENT)
  })

  it("rejects non-positive and fractional TTLs", () => {
    for (const ttl of [0, -5, 1.5, "300", null]) {
      const state = loadState(JSON.stringify({ cacheTtlSeconds: ttl }))
      expect(state.cacheTtlSeconds, String(ttl)).toBe(defaults().cacheTtlSeconds)
    }
  })

  it("treats an unknown dialect as mysql", () => {
    expect(loadState(JSON.stringify({ dialect: "sqlite" })).dialect).toBe("mysql")
    expect(loadState(JSON.stringify({ dialect: "postgres" })).dialect).toBe("postgres")
  })

  it("starts from the default DDL when none was stored", () => {
    expect(loadState(JSON.stringify({})).ddl).toBe(defaultDdl)
  })
})
