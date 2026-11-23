<script setup lang="ts">
import { ref, watch } from "vue"

import { highlight, type HighlightLang } from "@/lib/highlight"

const props = defineProps<{
  code: string
  lang: HighlightLang
}>()

const html = ref("")
// Guards against a stale async result overwriting a newer one.
let token = 0

watch(
  () => [props.code, props.lang] as const,
  async ([code, lang]) => {
    const mine = ++token
    const rendered = await highlight(code, lang)
    if (mine === token) html.value = rendered
  },
  { immediate: true },
)
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
