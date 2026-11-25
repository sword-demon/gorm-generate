// The generated project and the preview state built on it: table selection,
// the selected file, and the generate call. Export and copy state belong to
// their own composables; the caller resets those around a generate.

import { computed, ref } from "vue"

import { invoke } from "@tauri-apps/api/core"

import {
  generateErrorDetail,
  generateErrorTitle,
  toGenerateError,
  type GenerateError,
  type GeneratedProject,
} from "@/lib/ipc"

import type { SettingsRefs } from "./useSettings"

const DEFAULT_TTL_SECONDS = 300
// The core reads the TTL as a u32; anything larger would be rejected by serde.
const MAX_TTL_SECONDS = 4_294_967_295

// The TTL field is free text: it starts as a number, but clearing it leaves
// "", and a negative or fractional entry passes straight through. Coerce here,
// at the one place the request is assembled, so the core never sees a value it
// cannot deserialize.
function cacheTtl(seconds: number | string): number {
  const n = Math.trunc(Number(seconds))
  if (!Number.isFinite(n) || n < 1) return DEFAULT_TTL_SECONDS
  return Math.min(n, MAX_TTL_SECONDS)
}

export function useProject(settings: SettingsRefs, isBlocked: () => boolean) {
  const project = ref<GeneratedProject | null>(null)
  const selectedPath = ref("")
  const error = ref<GenerateError | null>(null)
  const pending = ref(false)
  // Toast dismissal: hide the parse-error toast without losing the inline error.
  const toastDismissed = ref(false)
  // Selected tables; set to all tables on each generate.
  const selectedTables = ref<Set<string>>(new Set())

  const visibleFiles = computed(() =>
    project.value?.files.filter((f) => selectedTables.value.has(f.table)) ?? [],
  )

  const selectedFile = computed(
    () => visibleFiles.value.find((f) => f.path === selectedPath.value) ?? null,
  )

  const allContent = computed(() =>
    visibleFiles.value.map((f) => `// ==== ${f.path} ====\n${f.content}`).join("\n"),
  )

  // Dependency hints come from the core, which derives them from the files it
  // actually emitted; the UI no longer re-derives them from the switches.
  const dependencies = computed(() => project.value?.dependencies ?? [])

  // Parse error toast: shown right after a failed generate, dismissible.
  const showErrorToast = computed(() => error.value !== null && !toastDismissed.value)

  const errorTitle = computed(() => generateErrorTitle(error.value))

  const errorDetail = computed(() => generateErrorDetail(error.value))

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
    // An export writes files for the current project; regenerating underneath
    // it would make its late status message describe a project that no longer
    // shows. The caller passes the export's busy flag in through `isBlocked`.
    if (pending.value || isBlocked()) return
    pending.value = true
    project.value = null
    selectedPath.value = ""
    error.value = null
    toastDismissed.value = false
    try {
      project.value = await invoke<GeneratedProject>("generate_project", {
        request: {
          ddl: settings.ddl.value,
          dialect: settings.dialect.value,
          modulePath: settings.modulePath.value,
          cacheEnabled: settings.cacheEnabled.value,
          cacheTtlSeconds: settings.cacheEnabled.value
            ? cacheTtl(settings.cacheTtlSeconds.value)
            : DEFAULT_TTL_SECONDS,
          tinyintAsBool: settings.tinyintAsBool.value,
        },
      })
      // Fresh generation: every table selected by default.
      selectedTables.value = new Set(project.value.tables)
      selectedPath.value = project.value.files[0]?.path ?? ""
    } catch (cause) {
      error.value = toGenerateError(cause)
    } finally {
      pending.value = false
    }
  }

  function dismissToast() {
    toastDismissed.value = true
  }

  return {
    project,
    pending,
    error,
    toastDismissed,
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
    generate,
    dismissToast,
  }
}

export type ProjectState = ReturnType<typeof useProject>
