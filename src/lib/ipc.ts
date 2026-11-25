// The IPC boundary: the shapes the Tauri commands return, and the narrowing
// that turns arbitrary rejection values into them. Nothing here touches Vue.

export type FileKind = "model" | "repository" | "service" | "cache"

export interface GeneratedFile {
  path: string
  content: string
  table: string
  kind: FileKind
}

export interface GeneratedProject {
  files: GeneratedFile[]
  tables: string[]
  warnings: string[]
  dependencies: string[]
}

export type GenerateError =
  | { kind: "parse"; statementIndex: number; snippet: string }
  | { kind: "validation"; message: string }

// Export failures come from two places with one shape: the core's tagged
// ExportError (ADR-0009), and Tauri's own "no picker-confirmed root"
// rejection.
export type ExportFailure =
  | { kind: "unsafePath"; path: string }
  | { kind: "duplicatePath"; path: string }
  | { kind: "conflicts"; paths: string[] }
  | { kind: "io"; message: string }
  | { kind: "noRoot"; message: string }

// Tauri rejects with whatever the command returned; narrow to the tagged
// union we render instead of trusting an arbitrary value.
export function toGenerateError(cause: unknown): GenerateError {
  if (cause && typeof cause === "object") {
    const c = cause as {
      kind?: unknown
      statementIndex?: unknown
      snippet?: unknown
      message?: unknown
    }
    if (c.kind === "parse" && typeof c.snippet === "string") {
      return {
        kind: "parse",
        statementIndex:
          typeof c.statementIndex === "number" ? c.statementIndex : 1,
        snippet: c.snippet,
      }
    }
    if (c.kind === "validation" && typeof c.message === "string") {
      return { kind: "validation", message: c.message }
    }
  }
  // IPC argument failures and anything else are request problems, not parse
  // errors; label them as such.
  return { kind: "validation", message: String(cause) }
}

export function generateErrorTitle(error: GenerateError | null): string {
  if (error === null) return ""
  return error.kind === "parse"
    ? `Parse error in statement #${error.statementIndex}`
    : "Invalid input"
}

export function generateErrorDetail(error: GenerateError | null): string {
  if (error === null) return ""
  return error.kind === "parse" ? error.snippet : error.message
}

// Narrow an IPC rejection to the export failure union instead of trusting an
// arbitrary value; anything unrecognised is reported verbatim.
export function toExportFailure(cause: unknown): ExportFailure {
  if (cause && typeof cause === "object") {
    const c = cause as {
      kind?: unknown
      path?: unknown
      paths?: unknown
      message?: unknown
    }
    if (
      (c.kind === "unsafePath" || c.kind === "duplicatePath") &&
      typeof c.path === "string"
    ) {
      return { kind: c.kind, path: c.path }
    }
    if (
      c.kind === "conflicts" &&
      Array.isArray(c.paths) &&
      c.paths.every((p: unknown) => typeof p === "string")
    ) {
      return { kind: "conflicts", paths: c.paths as string[] }
    }
    if ((c.kind === "io" || c.kind === "noRoot") && typeof c.message === "string") {
      return { kind: c.kind, message: c.message }
    }
  }
  return { kind: "io", message: String(cause) }
}

export function exportFailureText(failure: ExportFailure): string {
  switch (failure.kind) {
    case "unsafePath":
      return `unsafe export path: ${failure.path}`
    case "duplicatePath":
      return `duplicate export path: ${failure.path}`
    case "conflicts":
      return `${failure.paths.length} file(s) already exist`
    case "io":
      return `write failed: ${failure.message}`
    case "noRoot":
      return failure.message
  }
}
