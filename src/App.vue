<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core"
import { open } from "@tauri-apps/plugin-dialog"
import { computed, ref } from "vue"

import { Button } from "@/components/ui/button"

interface GeneratedFile {
  path: string
  content: string
}

interface GeneratedProject {
  files: GeneratedFile[]
  tables: string[]
  warnings: string[]
}

interface ParseError {
  statement_index: number
  snippet: string
}

const ddl = ref(
  "CREATE TABLE `users` (\n" +
    "  `id` BIGINT NOT NULL AUTO_INCREMENT,\n" +
    "  `username` VARCHAR(64) NOT NULL,\n" +
    "  `email` VARCHAR(128) NOT NULL,\n" +
    "  `age` TINYINT(1) NULL,\n" +
    "  `balance` DECIMAL(12,2) NOT NULL,\n" +
    "  `created_at` DATETIME NOT NULL,\n" +
    "  `deleted_at` DATETIME NULL,\n" +
    "  PRIMARY KEY (`id`)\n" +
    ") ENGINE=InnoDB COMMENT='用户表';",
)
const modulePath = ref("github.com/example/app")
const dialect = ref<"mysql" | "postgres">("mysql")
const tinyintAsBool = ref(false)
const cacheEnabled = ref(false)
const cacheTtlSeconds = ref(300)
const project = ref<GeneratedProject | null>(null)
const selectedPath = ref("")
const error = ref<ParseError | null>(null)
const pending = ref(false)
const copied = ref("")
const exporting = ref(false)
const exportMsg = ref("")

interface ExportResult {
  conflicts: string[]
  error: string | null
}

const selectedFile = computed(() =>
  project.value?.files.find((f) => f.path === selectedPath.value) ?? null,
)

async function generate() {
  pending.value = true
  project.value = null
  selectedPath.value = ""
  error.value = null
  copied.value = ""
  try {
    project.value = await invoke<GeneratedProject>("generate_project", {
      request: {
        ddl: ddl.value,
        dialect: dialect.value,
        modulePath: modulePath.value,
        cacheEnabled: cacheEnabled.value,
        cacheTtlSeconds: cacheTtlSeconds.value,
        tinyintAsBool: tinyintAsBool.value,
      },
    })
    selectedPath.value = project.value.files[0]?.path ?? ""
  } catch (cause) {
    error.value = cause as ParseError
  } finally {
    pending.value = false
  }
}

async function copy(text: string, label: string) {
  await navigator.clipboard.writeText(text)
  copied.value = label
  setTimeout(() => {
    if (copied.value === label) copied.value = ""
  }, 1500)
}

async function exportToDir() {
  if (!project.value) return
  const dir = await open({ directory: true, multiple: false, title: "Export to directory" })
  if (!dir) return

  exporting.value = true
  exportMsg.value = ""
  const files = project.value.files
  try {
    let result = await invoke<ExportResult>("export_project", { dir, files, overwrite: false })
    if (result.conflicts.length) {
      const proceed = window.confirm(
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
  project.value?.files.map((f) => `// ==== ${f.path} ====\n${f.content}`).join("\n") ?? "",
)
</script>

<template>
  <div class="flex h-screen flex-col bg-background text-foreground">
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
      <Button :disabled="pending" @click="generate">
        {{ pending ? "Generating..." : "Generate" }}
      </Button>
    </header>

    <div class="flex min-h-0 flex-1">
      <section class="flex w-1/2 min-w-0 flex-col border-r border-border">
        <div class="px-3 pt-2 text-xs font-medium text-muted-foreground">
          SQL DDL
        </div>
        <textarea
          v-model="ddl"
          spellcheck="false"
          class="min-h-0 flex-1 resize-none bg-transparent p-3 font-mono text-sm outline-none"
        />
      </section>

      <section class="flex w-1/2 min-w-0 flex-col">
        <div
          v-if="error"
          class="m-3 rounded-md border border-destructive bg-destructive/10 p-3 text-sm text-destructive"
        >
          <template v-if="error.statement_index">
            Parse error in statement #{{ error.statement_index }}:
          </template>
          {{ error.snippet }}
        </div>

        <template v-else-if="project">
          <div
            v-if="project.warnings.length"
            class="m-3 rounded-md border border-yellow-600/50 bg-yellow-500/10 p-2 text-xs text-yellow-700"
          >
            <div v-for="w in project.warnings" :key="w">{{ w }}</div>
          </div>
          <div class="flex items-center gap-2 border-b border-border px-3 py-2">
            <span class="text-xs text-muted-foreground">
              {{ project.files.length }} files
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
              :disabled="exporting"
              @click="exportToDir"
            >
              {{ exporting ? "Exporting..." : "Export to directory" }}
            </Button>
            <span v-if="exportMsg" class="truncate text-xs text-muted-foreground">
              {{ exportMsg }}
            </span>
          </div>
          <div class="flex min-h-0 flex-1">
            <ul class="w-56 shrink-0 overflow-y-auto border-r border-border py-1 text-sm">
              <li v-for="f in project.files" :key="f.path">
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
              <pre
                class="min-h-0 flex-1 overflow-auto bg-muted/40 p-3 font-mono text-xs leading-relaxed"
              >{{ selectedFile?.content }}</pre>
            </div>
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
