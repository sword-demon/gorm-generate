// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { useCopy } from "./useCopy"

function setClipboard(value: unknown) {
  Object.defineProperty(navigator, "clipboard", { value, configurable: true })
}

beforeEach(() => {
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
  vi.restoreAllMocks()
})

describe("useCopy", () => {
  it("copies and clears the label after the flash", async () => {
    const writeText = vi.fn(async () => {})
    setClipboard({ writeText })
    const copier = useCopy()

    await copier.copy("package models\n", "models/user.go")

    expect(writeText).toHaveBeenCalledWith("package models\n")
    expect(copier.copied.value).toBe("models/user.go")

    vi.advanceTimersByTime(1499)
    expect(copier.copied.value).toBe("models/user.go")

    vi.advanceTimersByTime(1)
    expect(copier.copied.value).toBe("")
  })

  it("keeps a newer label after an older copy's timer fires", async () => {
    setClipboard({ writeText: vi.fn(async () => {}) })
    const copier = useCopy()

    await copier.copy("a", "a.go")
    vi.advanceTimersByTime(1000)
    await copier.copy("b", "b.go")

    vi.advanceTimersByTime(1000)
    expect(copier.copied.value).toBe("b.go")
  })

  it("reports a missing clipboard", async () => {
    setClipboard(undefined)
    const copier = useCopy()

    await copier.copy("text", "label")

    expect(copier.copied.value).toBe("")
    expect(copier.copyError.value).toBe("Copy failed: Error: Clipboard is unavailable")
  })

  it("reports a rejected write", async () => {
    setClipboard({ writeText: vi.fn(async () => Promise.reject(new Error("denied"))) })
    const copier = useCopy()

    await copier.copy("text", "label")

    expect(copier.copied.value).toBe("")
    expect(copier.copyError.value).toBe("Copy failed: Error: denied")
  })

  it("resets both states", async () => {
    setClipboard({ writeText: vi.fn(async () => {}) })
    const copier = useCopy()
    await copier.copy("text", "label")

    copier.reset()

    expect(copier.copied.value).toBe("")
    expect(copier.copyError.value).toBe("")
  })
})
