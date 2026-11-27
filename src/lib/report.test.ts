import { describe, expect, it } from "vitest"

import { buildBugReportUrl } from "./report"

function params(url: string) {
  return new URL(url).searchParams
}

describe("buildBugReportUrl", () => {
  it("targets the bug form with a fixed title prefix", () => {
    const url = buildBugReportUrl({
      version: "0.1.3",
      dialect: "mysql",
      cacheEnabled: false,
    })

    expect(url.startsWith("https://github.com/sword-demon/gorm-generate/issues/new?")).toBe(
      true,
    )
    expect(params(url).get("template")).toBe("bug_report.yml")
    expect(params(url).get("title")).toBe("[bug] ")
  })

  it("prefills the facts triage needs", () => {
    const mysql = params(
      buildBugReportUrl({ version: "0.1.3", dialect: "mysql", cacheEnabled: false }),
    )
    expect(mysql.get("version")).toBe("0.1.3")
    expect(mysql.get("dialect")).toBe("MySQL")
    expect(mysql.get("cache")).toBe("关闭")

    const postgres = params(
      buildBugReportUrl({ version: "0.2.0", dialect: "postgres", cacheEnabled: true }),
    )
    expect(postgres.get("dialect")).toBe("PostgreSQL")
    expect(postgres.get("cache")).toBe("开启（Redis）")
  })

  it("never prefills the DDL", () => {
    const url = buildBugReportUrl({
      version: "0.1.3",
      dialect: "mysql",
      cacheEnabled: false,
    })

    expect(params(url).has("ddl")).toBe(false)
  })
})
