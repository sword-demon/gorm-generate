// Syntax-highlighted HTML for whatever `source()` currently returns.
//
// Highlighting is asynchronous, so a slow pass can finish after a newer one
// has already started; only the newest pass may write its HTML. Both editors
// (the SQL backdrop and the read-only preview) need exactly this, so it lives
// here once. `debounceMs` postpones passes while the input keeps changing —
// the SQL editor uses it so typing does not queue one pass per keystroke —
// but the first pass always runs at once, so the initial paint is colored.

import { getCurrentScope, onScopeDispose, ref, watch, type Ref } from "vue"

import { highlight, type HighlightLang } from "@/lib/highlight"

export interface HighlightInput {
  code: string
  lang: HighlightLang
}

export function useHighlight(
  source: () => HighlightInput,
  debounceMs = 0,
): Ref<string> {
  const html = ref("")
  // Guards against a stale async result overwriting a newer one.
  let token = 0
  let timer: ReturnType<typeof setTimeout> | undefined
  let first = true

  async function render({ code, lang }: HighlightInput) {
    const mine = ++token
    const rendered = await highlight(code, lang)
    if (mine === token) html.value = rendered
  }

  function schedule(input: HighlightInput) {
    // First paint is not a keystroke: color it at once rather than after a
    // delay. Without a debounce every change is its own pass.
    if (first || debounceMs === 0) {
      first = false
      void render(input)
      return
    }
    clearTimeout(timer)
    timer = setTimeout(() => void render(input), debounceMs)
  }

  watch(source, schedule, { immediate: true })

  if (getCurrentScope()) {
    onScopeDispose(() => {
      clearTimeout(timer)
      // An in-flight pass that lands after disposal is stale too.
      token++
    })
  }

  return html
}
