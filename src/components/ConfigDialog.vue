<script setup lang="ts">
// The settings that used to crowd the header, moved into one modal form.
//
// Every field is bound with v-model straight to the persisted settings refs,
// so an edit takes effect immediately — the same behaviour the inline header
// controls had.

import { nextTick, onScopeDispose, ref, watch } from "vue"

import { Button } from "@/components/ui/button"

const open = defineModel<boolean>("open", { required: true })
const dialect = defineModel<"mysql" | "postgres">("dialect", { required: true })
const modulePath = defineModel<string>("modulePath", { required: true })
const tinyintAsBool = defineModel<boolean>("tinyintAsBool", { required: true })
const cacheEnabled = defineModel<boolean>("cacheEnabled", { required: true })
const cacheTtlSeconds = defineModel<number | string>("cacheTtlSeconds", { required: true })

const modulePathInput = ref<HTMLInputElement | null>(null)

function close() {
  open.value = false
}

// Escape closes the dialog. The listener sits on the window so it works no
// matter where the focus is inside the form.
function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.stopPropagation()
    close()
  }
}

// Focus the first field when the dialog opens, and give the focus back to
// whatever had it before.
let previouslyFocused: HTMLElement | null = null

watch(
  open,
  async (isOpen, wasOpen) => {
    if (isOpen) {
      window.addEventListener("keydown", onKeydown)
      if (!wasOpen) {
        previouslyFocused = document.activeElement as HTMLElement | null
        await nextTick()
        modulePathInput.value?.focus()
      }
      return
    }
    window.removeEventListener("keydown", onKeydown)
    previouslyFocused?.focus()
    previouslyFocused = null
  },
  { immediate: true },
)

onScopeDispose(() => window.removeEventListener("keydown", onKeydown))
</script>

<template>
  <div
    v-if="open"
    class="fixed inset-0 z-50 flex items-start justify-center bg-black/40 p-6 pt-24"
    @click.self="close"
  >
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="config-dialog-title"
      class="w-full max-w-md rounded-lg border border-border bg-background p-4 shadow-xl"
    >
      <div class="mb-4 flex items-center justify-between">
        <h2 id="config-dialog-title" class="text-base font-semibold">Configuration</h2>
        <button
          class="rounded p-1 text-muted-foreground hover:bg-accent"
          aria-label="Close"
          @click="close"
        >
          ✕
        </button>
      </div>

      <div class="space-y-4">
        <label class="flex flex-col gap-1 text-sm">
          <span class="text-muted-foreground">Dialect</span>
          <select
            v-model="dialect"
            class="rounded-md border border-input bg-background px-2 py-1.5 text-sm"
          >
            <option value="mysql">MySQL</option>
            <option value="postgres">PostgreSQL</option>
          </select>
        </label>

        <label class="flex flex-col gap-1 text-sm">
          <span class="text-muted-foreground">Module path</span>
          <input
            ref="modulePathInput"
            v-model="modulePath"
            class="rounded-md border border-input bg-background px-2 py-1.5 font-mono text-sm"
          />
        </label>

        <label class="flex items-center gap-2 text-sm">
          <input v-model="tinyintAsBool" type="checkbox" class="accent-foreground" />
          tinyint(1) as bool
        </label>

        <label class="flex items-center gap-2 text-sm">
          <input v-model="cacheEnabled" type="checkbox" class="accent-foreground" />
          Redis cache
        </label>

        <label v-if="cacheEnabled" class="flex flex-col gap-1 text-sm">
          <span class="text-muted-foreground">Cache TTL (seconds)</span>
          <input
            v-model.number="cacheTtlSeconds"
            type="number"
            min="1"
            class="w-28 rounded-md border border-input bg-background px-2 py-1.5 text-sm"
          />
        </label>
      </div>

      <div class="mt-6 flex justify-end">
        <Button variant="outline" @click="close">Close</Button>
      </div>
    </div>
  </div>
</template>
