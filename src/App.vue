<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core"
import { open, confirm } from "@tauri-apps/plugin-dialog"
import { computed, ref, watch, onBeforeUnmount } from "vue"

import CodePreview from "@/components/CodePreview.vue"
import SqlEditor from "@/components/SqlEditor.vue"
import { Button } from "@/components/ui/button"

interface GeneratedFile {
  path: string
  content: string
  table: string
}

interface GeneratedProject {
  files: GeneratedFile[]
  tables: string[]
  warnings: string[]
}

interface ParseError {
  statementIndex: number
  snippet: string
}

interface ExportResult {
  conflicts: string[]
  error: string | null
}

const STORAGE_KEY = "gorm-generate:state"

const defaultDdl =
  "CREATE TABLE `users` (\n" +
  "  `id` BIGINT NOT NULL AUTO_INCREMENT,\n" +
  "  `username` VARCHAR(64) NOT NULL,\n" +
  "  `email` VARCHAR(128) NOT NULL,\n" +
  "  `age` TINYINT(1) NULL,\n" +
  "  `balance` DECIMAL(12,2) NOT NULL,\n" +
  "  `created_at` DATETIME NOT NULL,\n" +
  "  `deleted_at` DATETIME NULL,\n" +
  "  PRIMARY KEY (`id`)\n" +
  ") ENGINE=InnoDB COMMENT='用户表';"

interface PersistedState {
  ddl: string
  dialect: "mysql" | "postgres"
  modulePath: string
  cacheEnabled: boolean
  cacheTtlSeconds: number
  tinyintAsBool: boolean
}

function loadState(): PersistedState {
  const base = defaults()
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return base
    const s = JSON.parse(raw)
    // Validate each field; a corrupt/stale value falls back to its default.
    return {
      ddl: typeof s.ddl === "string" ? s.ddl : base.ddl,
      dialect: s.dialect === "postgres" ? "postgres" : "mysql",
      modulePath: typeof s.modulePath === "string" ? s.modulePath : base.modulePath,
      cacheEnabled: typeof s.cacheEnabled === "boolean" ? s.cacheEnabled : base.cacheEnabled,
      cacheTtlSeconds:
        Number.isInteger(s.cacheTtlSeconds) && s.cacheTtlSeconds > 0
          ? s.cacheTtlSeconds
          : base.cacheTtlSeconds,
      tinyintAsBool: typeof s.tinyintAsBool === "boolean" ? s.tinyintAsBool : base.tinyintAsBool,
    }
  } catch {
    // ignore corrupt state, fall back to defaults
  }
  return base
}

function defaults(): PersistedState {
  return {
    ddl: defaultDdl,
    dialect: "mysql",
    modulePath: "github.com/example/app",
    cacheEnabled: false,
    cacheTtlSeconds: 300,
    tinyintAsBool: false,
  }
}

const initial = loadState()
const ddl = ref(initial.ddl)
const modulePath = ref(initial.modulePath)
const dialect = ref<"mysql" | "postgres">(initial.dialect)
const tinyintAsBool = ref(initial.tinyintAsBool)
const cacheEnabled = ref(initial.cacheEnabled)
const cacheTtlSeconds = ref<number | string>(initial.cacheTtlSeconds)

const project = ref<GeneratedProject | null>(null)
const selectedPath = ref("")
const error = ref<ParseError | null>(null)
const pending = ref(false)
const copied = ref("")
const exporting = ref(false)
const exportMsg = ref("")
// Toast dismissal: hide the parse-error toast without losing the inline error.
const toastDismissed = ref(false)
// Selected tables; set to all tables on each generate.
const selectedTables = ref<Set<string>>(new Set())

const persistenceError = ref("")
const copyError = ref("")
let persistTimer: ReturnType<typeof setTimeout> | undefined
function saveState() {
  clearTimeout(persistTimer)
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({
      ddl: ddl.value, modulePath: modulePath.value, dialect: dialect.value,
      tinyintAsBool: tinyintAsBool.value, cacheEnabled: cacheEnabled.value,
      cacheTtlSeconds: cacheTtlSeconds.value,
    }))
    persistenceError.value = ""
  } catch {
    persistenceError.value = "Settings could not be saved. Keep a copy of your DDL before closing."
  }
}
function flushState() { saveState() }
window.addEventListener("pagehide", flushState)
onBeforeUnmount(() => {
  saveState()
  window.removeEventListener("pagehide", flushState)
})
// Debounce synchronous storage writes during typing.
watch(
  [ddl, modulePath, dialect, tinyintAsBool, cacheEnabled, cacheTtlSeconds],
  () => {
    clearTimeout(persistTimer)
    persistTimer = setTimeout(saveState, 300)
  },
  { deep: true },
)

const visibleFiles = computed(() =>
  project.value?.files.filter((f) => selectedTables.value.has(f.table)) ?? [],
)

const selectedFile = computed(
  () => visibleFiles.value.find((f) => f.path === selectedPath.value) ?? null,
)

function toggleTable(table: string) {
  const next = new Set(selectedTables.value)
  if (next.has(table)) next.delete(table)
  else next.add(table)
  selectedTables.value = next
  // Keep preview pointed at a visible file.
  if (!visibleFiles.value.some((f) => f.path === selectedPath.value)) {
    selectedPath.value = visibleFiles.value[0]?.path ?? ""
  }
}

async function generate() {
  // An export writes files for the current project; regenerating underneath it
  // would make its late status message describe a project that no longer shows.
  if (pending.value || exporting.value) return
  pending.value = true
  project.value = null
  selectedPath.value = ""
  error.value = null
  copied.value = ""
  exportMsg.value = ""
  copyError.value = ""
  toastDismissed.value = false
  try {
    if (!modulePath.value || modulePath.value.split("/").some((part) => !part || part === "." || part === ".." || part.startsWith(".") || part.endsWith(".")) || !/^[A-Za-z0-9._~/-]+$/.test(modulePath.value)) {
      throw new Error("Enter a valid Go module path without spaces, quotes or traversal.")
    }
    if (cacheEnabled.value && (typeof cacheTtlSeconds.value !== "number" || !Number.isInteger(cacheTtlSeconds.value) || cacheTtlSeconds.value < 1 || cacheTtlSeconds.value > 4294967295)) {
      throw new Error("Cache TTL must be an integer from 1 to 4294967295 seconds.")
    }
    project.value = await invoke<GeneratedProject>("generate_project", {
      request: {
        ddl: ddl.value,
        dialect: dialect.value,
        modulePath: modulePath.value,
        cacheEnabled: cacheEnabled.value,
        cacheTtlSeconds: cacheEnabled.value ? cacheTtlSeconds.value : 300,
        tinyintAsBool: tinyintAsBool.value,
      },
    })
    // Fresh generation: every table selected by default.
    selectedTables.value = new Set(project.value.tables)
    selectedPath.value = project.value.files[0]?.path ?? ""
  } catch (cause) {
    error.value = toParseError(cause)
  } finally {
    pending.value = false
  }
}

// Tauri rejects with whatever the command returned; narrow to the shape we
// render instead of trusting an arbitrary value.
function toParseError(cause: unknown): ParseError {
  if (cause && typeof cause === "object" && "snippet" in cause) {
    const c = cause as Partial<ParseError>
    return {
      statementIndex: typeof c.statementIndex === "number" ? c.statementIndex : 0,
      snippet: typeof c.snippet === "string" ? c.snippet : String(cause),
    }
  }
  return { statementIndex: 0, snippet: String(cause) }
}

async function copy(text: string, label: string) {
  copyError.value = ""
  try {
    if (!navigator.clipboard?.writeText) throw new Error("Clipboard is unavailable")
    await navigator.clipboard.writeText(text)
    copied.value = label
    setTimeout(() => {
      if (copied.value === label) copied.value = ""
    }, 1500)
  } catch (cause) {
    copyError.value = `Copy failed: ${String(cause)}`
  }
}

async function exportToDir() {
  if (!project.value || exporting.value || !visibleFiles.value.length) return
  exporting.value = true
  exportMsg.value = ""
  const files = visibleFiles.value
  try {
    const dir = await open({ directory: true, multiple: false, title: "Export to directory" })
    if (!dir) return
    let result = await invoke<ExportResult>("export_project", { dir, files, overwrite: false })
    if (result.conflicts.length) {
      const proceed = await confirm(
        `${result.conflicts.length} file(s) already exist and will be overwritten:\n\n` +
          result.conflicts.join("\n"),
      )
      if (!proceed) {
        exportMsg.value = "Export cancelled"
        return
      }
      result = await invoke<ExportResult>("export_project", { dir, files, overwrite: true })
    }
    exportMsg.value = result.error
      ? `Export failed: ${result.error}`
      : `Exported ${files.length} file(s) to ${dir}`
  } catch (cause) {
    exportMsg.value = `Export failed: ${String(cause)}`
  } finally {
    exporting.value = false
  }
}

const allContent = computed(() =>
  visibleFiles.value.map((f) => `// ==== ${f.path} ====\n${f.content}`).join("\n"),
)

// Dependency hints follow the current switches. Mirrors the imports emitted
// by crates/core/src/layers.rs — keep the two in sync.
const goGetDeps = computed(() => {
  const deps = ["gorm.io/gorm"]
  if (cacheEnabled.value) deps.push("github.com/redis/go-redis/v9")
  return deps
})

// Parse error toast: shown right after a failed generate, dismissible.
const showErrorToast = computed(() => error.value !== null && !toastDismissed.value)

function dismissToast() {
  toastDismissed.value = true
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
        <div class="font-medium">
          <template v-if="error.statementIndex">
            Parse error in statement #{{ error.statementIndex }}
          </template>
          <template v-else>Generation failed</template>
        </div>
        <pre class="mt-1 whitespace-pre-wrap break-words font-mono text-xs">{{ error.snippet }}</pre>
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
          <template v-if="error.statementIndex">
            Parse error in statement #{{ error.statementIndex }}:
          </template>
          {{ error.snippet }}
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
              v-for="dep in goGetDeps"
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
