// @vitest-environment happy-dom
// The composable is the seam both editors share: these tests pin the first
// immediate pass, debouncing, stale-result dropping and cleanup.

import { effectScope, nextTick, ref, type EffectScope } from "vue"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import type { HighlightLang } from "@/lib/highlight"
import { highlight } from "@/lib/highlight"

import { useHighlight } from "./useHighlight"

vi.mock("@/lib/highlight", () => ({ highlight: vi.fn() }))

const mockedHighlight = vi.mocked(highlight)

function deferred() {
  let resolve: (html: string) => void = () => {}
  const promise = new Promise<string>((r) => (resolve = r))
  return { promise, resolve }
}

const scopes: EffectScope[] = []

function setup(debounceMs?: number) {
  const scope = effectScope()
  scopes.push(scope)
  const code = ref("initial")
  const lang = ref<HighlightLang>("sql")
  const html = scope.run(
    () => useHighlight(() => ({ code: code.value, lang: lang.value }), debounceMs),
  )
  return { scope, code, lang, html: html! }
}

beforeEach(() => {
  vi.useFakeTimers()
  mockedHighlight.mockReset()
  mockedHighlight.mockImplementation(async (code) => `<pre>${code}</pre>`)
})

afterEach(() => {
  scopes.splice(0).forEach((scope) => scope.stop())
  vi.useRealTimers()
})

describe("useHighlight", () => {
  it("highlights the first value at once and renders the result", async () => {
    const { html } = setup(120)

    expect(mockedHighlight).toHaveBeenCalledTimes(1)
    expect(mockedHighlight).toHaveBeenCalledWith("initial", "sql")

    await nextTick()
    expect(html.value).toBe("<pre>initial</pre>")
  })

  it("re-highlights when the language changes", async () => {
    const { lang } = setup()

    lang.value = "go"
    await nextTick()
    await nextTick()

    expect(mockedHighlight).toHaveBeenLastCalledWith("initial", "go")
  })

  it("runs a pass per change when not debounced", async () => {
    const { code, html } = setup()

    code.value = "changed"
    await nextTick()
    await nextTick()

    expect(mockedHighlight).toHaveBeenCalledTimes(2)
    expect(html.value).toBe("<pre>changed</pre>")
  })

  it("coalesces changes into one pass after the delay", async () => {
    const { code } = setup(120)
    mockedHighlight.mockClear()

    code.value = "a"
    await nextTick()
    code.value = "ab"
    await nextTick()

    expect(mockedHighlight).not.toHaveBeenCalled()

    vi.advanceTimersByTime(120)
    await nextTick()
    await nextTick()

    expect(mockedHighlight).toHaveBeenCalledTimes(1)
    expect(mockedHighlight).toHaveBeenCalledWith("ab", "sql")
  })

  it("drops a slow older result instead of letting it overwrite a newer one", async () => {
    const { code, html } = setup()
    const slow = deferred()
    mockedHighlight.mockReset()
    mockedHighlight.mockReturnValueOnce(slow.promise)
    mockedHighlight.mockResolvedValueOnce("<pre>fresh</pre>")

    code.value = "old"
    await nextTick()
    code.value = "new"
    await nextTick()
    await nextTick()

    slow.resolve("<pre>stale</pre>")
    await nextTick()
    await nextTick()

    expect(html.value).toBe("<pre>fresh</pre>")
  })

  it("cancels a pending pass when its scope goes away", async () => {
    const { scope, code } = setup(120)
    mockedHighlight.mockClear()

    code.value = "gone"
    await nextTick()
    scope.stop()
    vi.advanceTimersByTime(500)

    expect(mockedHighlight).not.toHaveBeenCalled()
  })

  it("ignores a result that lands after its scope goes away", async () => {
    const { scope, code, html } = setup()
    await nextTick()
    const slow = deferred()
    mockedHighlight.mockReset()
    mockedHighlight.mockReturnValueOnce(slow.promise)

    code.value = "late"
    await nextTick()
    scope.stop()
    slow.resolve("<pre>late</pre>")
    await nextTick()
    await nextTick()

    expect(html.value).toBe("<pre>initial</pre>")
  })
})
