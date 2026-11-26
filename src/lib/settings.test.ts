import { describe, expect, it } from "vitest"

import {
  defaults,
  loadState,
  MAX_PANE_PERCENT,
  MIN_PANE_PERCENT,
  mysqlSampleDdl,
  postgresSampleDdl,
  sampleDdl,
  sampleForDialect,
} from "./settings"

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
    expect(loadState(JSON.stringify({})).ddl).toBe(mysqlSampleDdl)
  })

  it("swaps a stored sample of the other dialect for this dialect's one", () => {
    const state = loadState(
      JSON.stringify({ ddl: mysqlSampleDdl, dialect: "postgres" }),
    )
    expect(state.ddl).toBe(postgresSampleDdl)
  })

  it("keeps a stored script the user wrote", () => {
    const ddl = "CREATE TABLE users (id BIGINT PRIMARY KEY);"
    expect(loadState(JSON.stringify({ ddl, dialect: "postgres" })).ddl).toBe(ddl)
  })
})

describe("sample DDL", () => {
  it("is written in the dialect it belongs to", () => {
    expect(sampleDdl("mysql")).toBe(mysqlSampleDdl)
    expect(sampleDdl("postgres")).toBe(postgresSampleDdl)
    expect(mysqlSampleDdl).toContain("AUTO_INCREMENT")
    expect(postgresSampleDdl).toContain("BIGSERIAL")
    expect(postgresSampleDdl).toContain("COMMENT ON TABLE users")
  })

  it("swaps the untouched sample of the other dialect", () => {
    expect(sampleForDialect(mysqlSampleDdl, "postgres")).toBe(postgresSampleDdl)
    expect(sampleForDialect(postgresSampleDdl, "mysql")).toBe(mysqlSampleDdl)
  })

  it("leaves a script the user wrote alone", () => {
    const ddl = "CREATE TABLE users (id BIGINT PRIMARY KEY);"
    expect(sampleForDialect(ddl, "postgres")).toBeNull()
    expect(sampleForDialect(ddl, "mysql")).toBeNull()
  })

  it("leaves the dialect's own sample alone", () => {
    expect(sampleForDialect(mysqlSampleDdl, "mysql")).toBeNull()
    expect(sampleForDialect(postgresSampleDdl, "postgres")).toBeNull()
  })
})
