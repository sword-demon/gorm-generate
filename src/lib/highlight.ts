import { createHighlighterCore, type HighlighterCore } from "shiki/core"
import { createJavaScriptRegexEngine } from "shiki/engine/javascript"

// Lazy singleton: only the two grammars we need, loaded once.
let highlighterPromise: Promise<HighlighterCore> | null = null

function getHighlighter(): Promise<HighlighterCore> {
  if (!highlighterPromise) {
    highlighterPromise = createHighlighterCore({
      themes: [import("@shikijs/themes/github-light")],
      langs: [import("@shikijs/langs/go"), import("@shikijs/langs/sql")],
      engine: createJavaScriptRegexEngine(),
    })
  }
  return highlighterPromise
}

export type HighlightLang = "go" | "sql"

/// Returns syntax-highlighted HTML (light theme). Falls back to escaped text
/// if the highlighter fails to load.
export async function highlight(code: string, lang: HighlightLang): Promise<string> {
  try {
    const hl = await getHighlighter()
    return hl.codeToHtml(code, { lang, theme: "github-light" })
  } catch {
    const escaped = code
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
    return `<pre class="shiki"><code>${escaped}</code></pre>`
  }
}
