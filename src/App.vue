<script setup lang="ts">
import { computed, ref } from "vue"

import { confirm } from "@tauri-apps/plugin-dialog"
import { openUrl } from "@tauri-apps/plugin-opener"

import CodePreview from "@/components/CodePreview.vue"
import ConfigDialog from "@/components/ConfigDialog.vue"
import SqlEditor from "@/components/SqlEditor.vue"
import { Button } from "@/components/ui/button"
import { useCopy } from "@/composables/useCopy"
import { useExport } from "@/composables/useExport"
import { usePaneSplit, MAX_PANE_PERCENT, MIN_PANE_PERCENT } from "@/composables/usePaneSplit"
import { useProject } from "@/composables/useProject"
import { useSettings } from "@/composables/useSettings"
import { buildBugReportUrl } from "@/lib/report"
import { dialectLabel } from "@/lib/settings"

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
  panePercent,
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

// The settings live in a dialog; the header only shows what they currently are.
const configOpen = ref(false)
const reportError = ref("")

// Bug reports are public: ask before sending anyone to the form, so pasting
// DDL is a deliberate choice (design §6 item 3).
const PUBLIC_REPORT_WARNING =
  "Bug reports are public. Remove table names and business details from the DDL before you paste it."

// Opens the prefilled bug form in the system browser. Only what the app knows
// is prefilled; the reporter decides what DDL (if any) to paste.
async function reportIssue() {
  try {
    if (!(await confirm(PUBLIC_REPORT_WARNING))) return
    await openUrl(
      buildBugReportUrl({
        version: __APP_VERSION__,
        dialect: dialect.value,
        cacheEnabled: cacheEnabled.value,
      }),
    )
    reportError.value = ""
  } catch (cause) {
    reportError.value = `Could not open the issue page: ${String(cause)}`
  }
}
const configSummary = computed(() => {
  const parts = [
    dialectLabel(dialect.value),
    modulePath.value || "no module path",
  ]
  if (tinyintAsBool.value) parts.push("tinyint(1) as bool")
  if (cacheEnabled.value) parts.push(`Redis cache ${cacheTtlSeconds.value}s`)
  return parts.join(" · ")
})

// The divider between the SQL pane and the result pane.
const splitContainer = ref<HTMLElement | null>(null)
const { dragging, onPointerDown, onKeyDown } = usePaneSplit(panePercent, splitContainer)

// Each dependency is copied as the command that installs it.
function depLabel(dep: string) {
  return `dep:${dep}`
}

const allDependencyCommands = computed(() =>
  dependencies.value.map((dep) => `go get ${dep}`).join("\n"),
)

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
    <header class="flex items-center gap-3 border-b border-border px-4 py-3">
      <h1 class="shrink-0 text-lg font-semibold tracking-tight">GORM Code Generator</h1>
      <p class="hidden min-w-0 flex-1 truncate text-xs text-muted-foreground md:block">
        {{ configSummary }}
      </p>
      <Button
        class="shrink-0"
        variant="ghost"
        size="icon"
        aria-label="Report an issue"
        title="Report an issue"
        @click="reportIssue"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <circle cx="12" cy="12" r="10" />
          <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" />
          <path d="M12 17h.01" />
        </svg>
      </Button>
      <Button class="shrink-0" :disabled="pending || exporting" @click="generate">
        {{ pending ? "Generating..." : "Generate" }}
      </Button>
      <Button
        class="shrink-0"
        variant="outline"
        size="icon"
        aria-label="Configuration"
        title="Configuration"
        @click="configOpen = true"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="size-4"
          aria-hidden="true"
        >
          <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
          <circle cx="12" cy="12" r="3" />
        </svg>
      </Button>
    </header>

    <ConfigDialog
      v-model:open="configOpen"
      v-model:dialect="dialect"
      v-model:module-path="modulePath"
      v-model:tinyint-as-bool="tinyintAsBool"
      v-model:cache-enabled="cacheEnabled"
      v-model:cache-ttl-seconds="cacheTtlSeconds"
    />

    <div
      v-if="persistenceError || copyError || reportError"
      role="alert"
      class="px-3 py-2 text-sm text-destructive"
    >
      {{ persistenceError || copyError || reportError }}
    </div>
    <div ref="splitContainer" class="flex min-h-0 flex-1">
      <section
        class="flex min-w-0 flex-col"
        :style="{ width: `${panePercent}%` }"
      >
        <div class="px-3 pt-2 text-xs font-medium text-muted-foreground">
          SQL DDL
        </div>
        <SqlEditor v-model="ddl" />
      </section>

      <div
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize panes"
        :aria-valuenow="Math.round(panePercent)"
        :aria-valuemin="MIN_PANE_PERCENT"
        :aria-valuemax="MAX_PANE_PERCENT"
        tabindex="0"
        class="group relative w-2 shrink-0 cursor-col-resize touch-none"
        @pointerdown="onPointerDown"
        @keydown="onKeyDown"
      >
        <span
          class="pointer-events-none absolute inset-y-0 left-1/2 w-px -translate-x-1/2 bg-border group-hover:bg-primary"
          :class="{ 'bg-primary': dragging }"
        />
      </div>

      <section class="flex min-w-0 flex-1 flex-col">
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
            <button
              v-for="dep in dependencies"
              :key="dep"
              class="dep-chip rounded bg-muted px-1.5 py-0.5 font-mono hover:bg-accent"
              :title="`Copy go get ${dep}`"
              @click="copy(`go get ${dep}`, depLabel(dep))"
            >
              <span v-if="copied === depLabel(dep)" class="text-primary">✓</span>
              go get {{ dep }}
            </button>
            <button
              v-if="dependencies.length"
              class="dep-copy-all underline underline-offset-2 hover:text-foreground"
              @click="copy(allDependencyCommands, 'deps')"
            >
              {{ copied === "deps" ? "Copied!" : "Copy all commands" }}
            </button>
          </div>
        </template>

        <div
          v-else
          class="flex flex-1 items-center justify-center text-sm text-muted-foreground"
        >
          Paste {{ dialect === "postgres" ? "PostgreSQL" : "MySQL" }} DDL and click Generate.
        </div>
      </section>
    </div>
  </div>
</template>
