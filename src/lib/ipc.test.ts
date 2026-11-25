import { describe, expect, it } from "vitest"

import {
  exportFailureText,
  generateErrorDetail,
  generateErrorTitle,
  toExportFailure,
  toGenerateError,
} from "./ipc"

describe("toGenerateError", () => {
  it("keeps a parse error with its statement index and snippet", () => {
    expect(
      toGenerateError({ kind: "parse", statementIndex: 3, snippet: "CREATE TABL" }),
    ).toEqual({ kind: "parse", statementIndex: 3, snippet: "CREATE TABL" })
  })

  it("defaults a missing statement index to the first statement", () => {
    expect(toGenerateError({ kind: "parse", snippet: "x" })).toEqual({
      kind: "parse",
      statementIndex: 1,
      snippet: "x",
    })
  })

  it("keeps a validation error", () => {
    expect(toGenerateError({ kind: "validation", message: "bad module path" })).toEqual({
      kind: "validation",
      message: "bad module path",
    })
  })

  it("labels unknown rejections as validation", () => {
    expect(toGenerateError("boom")).toEqual({ kind: "validation", message: "boom" })
    expect(toGenerateError(null)).toEqual({ kind: "validation", message: "null" })
  })

  it("rejects a parse shape whose fields have the wrong types", () => {
    expect(toGenerateError({ kind: "parse", snippet: 42 })).toEqual({
      kind: "validation",
      message: "[object Object]",
    })
  })
})

describe("generate error presentation", () => {
  it("is empty without an error", () => {
    expect(generateErrorTitle(null)).toBe("")
    expect(generateErrorDetail(null)).toBe("")
  })

  it("names the failing statement for parse errors", () => {
    const error = { kind: "parse" as const, statementIndex: 2, snippet: "CREATE" }
    expect(generateErrorTitle(error)).toBe("Parse error in statement #2")
    expect(generateErrorDetail(error)).toBe("CREATE")
  })

  it("uses the message for validation errors", () => {
    const error = { kind: "validation" as const, message: "bad module path" }
    expect(generateErrorTitle(error)).toBe("Invalid input")
    expect(generateErrorDetail(error)).toBe("bad module path")
  })
})

describe("toExportFailure", () => {
  it("passes every core failure through", () => {
    expect(toExportFailure({ kind: "unsafePath", path: "../x.go" })).toEqual({
      kind: "unsafePath",
      path: "../x.go",
    })
    expect(toExportFailure({ kind: "duplicatePath", path: "a.go" })).toEqual({
      kind: "duplicatePath",
      path: "a.go",
    })
    expect(toExportFailure({ kind: "conflicts", paths: ["a.go", "b.go"] })).toEqual({
      kind: "conflicts",
      paths: ["a.go", "b.go"],
    })
    expect(toExportFailure({ kind: "io", message: "/x: denied" })).toEqual({
      kind: "io",
      message: "/x: denied",
    })
    expect(toExportFailure({ kind: "noRoot", message: "No export directory selected" })).toEqual({
      kind: "noRoot",
      message: "No export directory selected",
    })
  })

  it("falls back to io for malformed shapes", () => {
    expect(toExportFailure({ kind: "conflicts", paths: ["a.go", 2] })).toEqual({
      kind: "io",
      message: "[object Object]",
    })
    expect(toExportFailure({ kind: "io" })).toEqual({
      kind: "io",
      message: "[object Object]",
    })
    expect(toExportFailure(undefined)).toEqual({ kind: "io", message: "undefined" })
  })
})

describe("exportFailureText", () => {
  it("renders each kind", () => {
    expect(exportFailureText({ kind: "unsafePath", path: "../x.go" })).toBe(
      "unsafe export path: ../x.go",
    )
    expect(exportFailureText({ kind: "duplicatePath", path: "a.go" })).toBe(
      "duplicate export path: a.go",
    )
    expect(exportFailureText({ kind: "conflicts", paths: ["a.go", "b.go"] })).toBe(
      "2 file(s) already exist",
    )
    expect(exportFailureText({ kind: "io", message: "/x: denied" })).toBe(
      "write failed: /x: denied",
    )
    expect(exportFailureText({ kind: "noRoot", message: "No export directory selected" })).toBe(
      "No export directory selected",
    )
  })
})
