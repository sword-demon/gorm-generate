<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core"
import { ref } from "vue"

import { Button } from "@/components/ui/button"

const result = ref("")
const error = ref("")
const pending = ref(false)

async function ping() {
  pending.value = true
  result.value = ""
  error.value = ""
  try {
    result.value = await invoke<string>("ping")
  } catch (cause) {
    error.value = String(cause)
  } finally {
    pending.value = false
  }
}
</script>

<template>
  <main
    class="flex min-h-screen flex-col items-center justify-center gap-6 bg-background p-8 text-foreground"
  >
    <div class="space-y-2 text-center">
      <h1 class="text-3xl font-semibold tracking-tight">GORM Code Generator</h1>
      <p class="text-sm text-muted-foreground">
        Paste SQL DDL, get layered Go code.
      </p>
    </div>

    <Button :disabled="pending" @click="ping">
      {{ pending ? "Pinging..." : "Ping Rust" }}
    </Button>

    <pre
      v-if="result"
      class="rounded-md border border-border bg-muted px-4 py-2 font-mono text-sm"
    >{{ result }}</pre>
    <p v-else-if="error" class="text-sm text-destructive">{{ error }}</p>
  </main>
</template>
