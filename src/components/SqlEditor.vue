<script setup lang="ts">
import { ref } from "vue"

import { useHighlight } from "@/composables/useHighlight"

// Highlighting the whole script on every keystroke would queue one pass per
// character; typing settles long before that. The first render is immediate so
// the editor is colored as soon as it mounts.
const HIGHLIGHT_DEBOUNCE_MS = 120

// Highlighted backdrop behind a transparent textarea: SQL gets syntax color
// while staying editable. Scroll and metrics stay in sync.
const model = defineModel<string>({ required: true })
// Trailing newline keeps the last line's height aligned with the textarea.
const html = useHighlight(
  () => ({ code: model.value + "\n", lang: "sql" }),
  HIGHLIGHT_DEBOUNCE_MS,
)
const backdrop = ref<HTMLElement | null>(null)

function syncScroll(e: Event) {
  const ta = e.target as HTMLTextAreaElement
  if (backdrop.value) {
    backdrop.value.scrollTop = ta.scrollTop
    backdrop.value.scrollLeft = ta.scrollLeft
  }
}
</script>

<template>
  <div class="relative min-h-0 flex-1">
    <div
      ref="backdrop"
      class="sql-backdrop pointer-events-none absolute inset-0 overflow-auto"
      aria-hidden="true"
      v-html="html"
    />
    <textarea
      v-model="model"
      spellcheck="false"
      aria-label="SQL DDL input"
      class="sql-input absolute inset-0 h-full w-full resize-none overflow-auto bg-transparent p-3 font-mono text-sm leading-relaxed text-transparent caret-foreground outline-none focus-visible:ring-1 focus-visible:ring-ring"
      @scroll="syncScroll"
    />
  </div>
</template>

<style scoped>
.sql-backdrop :deep(pre),
.sql-input {
  margin: 0;
  padding: 0.75rem;
  font-size: 0.875rem;
  line-height: 1.625;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  white-space: pre-wrap;
  word-break: break-word;
  tab-size: 2;
}

.sql-backdrop :deep(pre) {
  background: transparent !important;
}

/* Transparent text would hide the selection highlight; tint it instead. */
.sql-input::selection {
  background: oklch(0.708 0 0 / 0.35);
}
</style>
