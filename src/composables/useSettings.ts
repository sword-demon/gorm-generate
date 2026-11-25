// The settings the app persists: one ref per field, saved to localStorage on a
// short debounce and flushed on page hide or scope disposal.

import { onScopeDispose, ref, watch } from "vue"

import { loadState, STORAGE_KEY } from "@/lib/settings"

export function useSettings() {
  const initial = loadState(localStorage.getItem(STORAGE_KEY))
  const ddl = ref(initial.ddl)
  const modulePath = ref(initial.modulePath)
  const dialect = ref<"mysql" | "postgres">(initial.dialect)
  const tinyintAsBool = ref(initial.tinyintAsBool)
  const cacheEnabled = ref(initial.cacheEnabled)
  const cacheTtlSeconds = ref<number | string>(initial.cacheTtlSeconds)
  const persistenceError = ref("")

  let persistTimer: ReturnType<typeof setTimeout> | undefined

  function saveState() {
    clearTimeout(persistTimer)
    try {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({
          ddl: ddl.value,
          modulePath: modulePath.value,
          dialect: dialect.value,
          tinyintAsBool: tinyintAsBool.value,
          cacheEnabled: cacheEnabled.value,
          cacheTtlSeconds: cacheTtlSeconds.value,
        }),
      )
      persistenceError.value = ""
    } catch {
      persistenceError.value =
        "Settings could not be saved. Keep a copy of your DDL before closing."
    }
  }

  function flushState() {
    saveState()
  }

  window.addEventListener("pagehide", flushState)
  // `onScopeDispose` also fires when the owning component unmounts, and works
  // under a bare effect scope in tests.
  onScopeDispose(() => {
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

  return {
    ddl,
    modulePath,
    dialect,
    tinyintAsBool,
    cacheEnabled,
    cacheTtlSeconds,
    persistenceError,
  }
}

export type SettingsRefs = Pick<
  ReturnType<typeof useSettings>,
  | "ddl"
  | "modulePath"
  | "dialect"
  | "tinyintAsBool"
  | "cacheEnabled"
  | "cacheTtlSeconds"
>
