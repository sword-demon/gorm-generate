<script setup lang="ts">
import CodePreview from "@/components/CodePreview.vue"
import SqlEditor from "@/components/SqlEditor.vue"
import { Button } from "@/components/ui/button"
import { useCopy } from "@/composables/useCopy"
import { useExport } from "@/composables/useExport"
import { useProject } from "@/composables/useProject"
import { useSettings } from "@/composables/useSettings"

const settings = useSettings()
// Generate is blocked while an export is writing. The flag lives in the export
// composable and is read lazily, so the two can be assembled in either order.
const projectState = useProject(settings, () => exporter.exporting.value)
const exporter = useExport(projectState)
const copier = useCopy()

const {
  ddl,
  modulePath,
  dialect,
  tinyintAsBool,
  cacheEnabled,
  cacheTtlSeconds,
  persistenceError,
} = settings

const {
  project,
  pending,
  error,
  selectedTables,
  selectedPath,
  visibleFiles,
  selectedFile,
  allContent,
  dependencies,
  showErrorToast,
  errorTitle,
  errorDetail,
  toggleTable,
  dismissToast,
} = projectState

const { exporting, exportMsg, exportToDir } = exporter
const { copied, copyError, copy } = copier

// A new project makes a previous export's or copy's status stale.
function generate() {
  if (projectState.pending.value || exporter.exporting.value) return
  exporter.reset()
  copier.reset()
  void projectState.generate()
}
</script>

<template>
  <div class="flex h-screen flex-col bg-background text-foreground">
    <!-- Parse-error toast (fixed, dismissible). -->
    <div
      v-if="error && showErrorToast"
      class="fixed right-4 top-4 z-50 flex max-w-md items-start gap-3 rounded-md border border-destructive bg-destructive/10 p-3 text-sm text-destructive shadow-lg"
      role="alert"
    >
      <div class="min-w-0">
        <div class="font-medium">{{ errorTitle }}</div>
        <pre class="mt-1 whitespace-pre-wrap break-words font-mono text-xs">{{ errorDetail }}</pre>
      </div>
      <button
        class="shrink-0 rounded p-1 hover:bg-destructive/20"
        aria-label="Dismiss"
        @click="dismissToast"
      >
        ✕
      </button>
    </div>
    <header
      class="flex items-center gap-4 border-b border-border px-4 py-3"
    >
      <h1 class="text-lg font-semibold tracking-tight">GORM Code Generator</h1>
      <select
        v-model="dialect"
        class="rounded-md border border-input bg-background px-2 py-1 text-sm"
      >
        <option value="mysql">MySQL</option>
        <option value="postgres">PostgreSQL</option>
      </select>
      <label class="flex items-center gap-2 text-sm text-muted-foreground">
        module path
        <input
          v-model="modulePath"
          class="w-64 rounded-md border border-input bg-background px-2 py-1 text-sm"
        />
      </label>
      <label class="flex items-center gap-2 text-sm text-muted-foreground">
        <input v-model="tinyintAsBool" type="checkbox" class="accent-foreground" />
        tinyint(1) as bool
      </label>
      <label class="flex items-center gap-2 text-sm text-muted-foreground">
        <input v-model="cacheEnabled" type="checkbox" class="accent-foreground" />
        Redis cache
      </label>
      <label
        v-if="cacheEnabled"
        class="flex items-center gap-2 text-sm text-muted-foreground"
      >
        TTL(s)
        <input
          v-model.number="cacheTtlSeconds"
          type="number"
          min="1"
          class="w-20 rounded-md border border-input bg-background px-2 py-1 text-sm"
        />
      </label>
      <Button :disabled="pending || exporting" @click="generate">
        {{ pending ? "Generating..." : "Generate" }}
      </Button>
    </header>

    <div v-if="persistenceError || copyError" role="alert" class="px-3 py-2 text-sm text-destructive">
      {{ persistenceError || copyError }}
    </div>
    <div class="flex min-h-0 flex-1">
      <section class="flex w-1/2 min-w-0 flex-col border-r border-border">
        <div class="px-3 pt-2 text-xs font-medium text-muted-foreground">
          SQL DDL
        </div>
        <SqlEditor v-model="ddl" />
      </section>

      <section class="flex w-1/2 min-w-0 flex-col">
        <div
          v-if="error"
          class="m-3 rounded-md border border-destructive bg-destructive/10 p-3 text-sm text-destructive"
        >
          <div class="font-medium">{{ errorTitle }}</div>
          <pre class="mt-1 whitespace-pre-wrap break-words font-mono text-xs">{{ errorDetail }}</pre>
        </div>

        <template v-else-if="project">
          <details
            v-if="project.warnings.length"
            class="m-3 rounded-md border border-yellow-600/50 bg-yellow-500/10 text-xs text-yellow-700"
          >
            <summary class="flex cursor-pointer items-center gap-2 p-2">
              <span
                class="inline-flex items-center rounded-full bg-yellow-600/20 px-2 py-0.5 font-medium"
              >
                {{ project.warnings.length }} warning{{ project.warnings.length === 1 ? "" : "s" }}
              </span>
            </summary>
            <ul class="list-disc space-y-1 px-6 pb-2">
              <li v-for="(w, index) in project.warnings" :key="index">{{ w }}</li>
            </ul>
          </details>
          <div class="flex items-center gap-2 border-b border-border px-3 py-2">
            <span class="text-xs text-muted-foreground">
              {{ visibleFiles.length }} / {{ project.files.length }} files
            </span>
            <Button
              size="sm"
              variant="outline"
              @click="copy(allContent, 'all')"
            >
              {{ copied === "all" ? "Copied!" : "Copy all" }}
            </Button>
            <Button
              size="sm"
              variant="outline"
              :disabled="exporting || !visibleFiles.length"
              @click="exportToDir"
            >
              {{ exporting ? "Exporting..." : "Export to directory" }}
            </Button>
            <span v-if="exportMsg" :title="exportMsg" class="truncate text-xs text-muted-foreground">
              {{ exportMsg }}
            </span>
          </div>
          <div class="flex items-center gap-3 border-b border-border px-3 py-1.5 text-sm">
            <span class="text-xs font-medium text-muted-foreground">Tables</span>
            <label
              v-for="t in project.tables"
              :key="t"
              class="flex items-center gap-1 text-xs text-muted-foreground"
            >
              <input
                type="checkbox"
                class="accent-foreground"
                :checked="selectedTables.has(t)"
                @change="toggleTable(t)"
              />
              {{ t }}
            </label>
          </div>
          <div class="flex min-h-0 flex-1">
            <ul class="w-56 shrink-0 overflow-y-auto border-r border-border py-1 text-sm">
              <li v-for="f in visibleFiles" :key="f.path">
                <button
                  class="w-full truncate px-3 py-1.5 text-left hover:bg-accent"
                  :class="{ 'bg-accent font-medium': f.path === selectedPath }"
                  @click="selectedPath = f.path"
                >
                  {{ f.path }}
                </button>
              </li>
            </ul>
            <div class="flex min-w-0 flex-1 flex-col">
              <div class="flex items-center justify-between px-3 py-1.5">
                <span class="truncate text-xs text-muted-foreground">
                  {{ selectedFile?.path }}
                </span>
                <Button
                  v-if="selectedFile"
                  size="sm"
                  variant="ghost"
                  @click="copy(selectedFile.content, selectedFile.path)"
                >
                  {{ copied === selectedFile.path ? "Copied!" : "Copy" }}
                </Button>
              </div>
              <CodePreview
                v-if="selectedFile"
                :code="selectedFile.content"
                lang="go"
              />
              <div
                v-else
                class="flex min-h-0 flex-1 items-center justify-center text-xs text-muted-foreground"
              >
                No file selected.
              </div>
            </div>
          </div>
          <div
            class="flex flex-wrap items-center gap-2 border-t border-border px-3 py-2 text-xs text-muted-foreground"
          >
            <span class="font-medium">Dependencies</span>
            <code
              v-for="dep in dependencies"
              :key="dep"
              class="rounded bg-muted px-1.5 py-0.5 font-mono"
            >go get {{ dep }}</code>
          </div>
        </template>

        <div
          v-else
          class="flex flex-1 items-center justify-center text-sm text-muted-foreground"
        >
          Paste MySQL DDL and click Generate.
        </div>
      </section>
    </div>
  </div>
</template>
