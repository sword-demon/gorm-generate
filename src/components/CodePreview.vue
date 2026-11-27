<script setup lang="ts">
import { useHighlight } from "@/composables/useHighlight"
import type { HighlightLang } from "@/lib/highlight"

const props = defineProps<{
  code: string
  lang: HighlightLang
}>()

const html = useHighlight(() => ({ code: props.code, lang: props.lang }))
</script>

<template>
  <!-- Read-only: Shiki output is inert HTML, no inputs or handlers. -->
  <div class="code-preview min-h-0 flex-1 overflow-auto" v-html="html" />
</template>

<style scoped>
.code-preview :deep(pre) {
  margin: 0;
  padding: 0.75rem;
  font-size: 0.75rem;
  line-height: 1.6;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}
</style>
