import { onScopeDispose, ref, watch } from "vue"

import { loadState, sampleForDialect, STORAGE_KEY } from "@/lib/settings"

export function useSettings() {
  const initial = loadState(localStorage.getItem(STORAGE_KEY))
  const ddl = ref(initial.ddl)
  const modulePath = ref(initial.modulePath)
  const dialect = ref<"mysql" | "postgres">(initial.dialect)
  const tinyintAsBool = ref(initial.tinyintAsBool)
  const cacheEnabled = ref(initial.cacheEnabled)
  const cacheTtlSeconds = ref<number | string>(initial.cacheTtlSeconds)
  const panePercent = ref(initial.panePercent)
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
          panePercent: panePercent.value,
        }),
      )
      persistenceError.value = ""
    } catch {
      persistenceError.value =
        "Settings could not be saved. Keep a copy of your DDL before closing."
    }
  }

  window.addEventListener("pagehide", saveState)
  // `onScopeDispose` also fires when the owning component unmounts, and works
  // under a bare effect scope in tests.
  onScopeDispose(() => {
    saveState()
    window.removeEventListener("pagehide", saveState)
  })

  // Debounce synchronous storage writes during typing.
  watch(
    [ddl, modulePath, dialect, tinyintAsBool, cacheEnabled, cacheTtlSeconds, panePercent],
    () => {
      clearTimeout(persistTimer)
      persistTimer = setTimeout(saveState, 300)
    },
    { deep: true },
  )

  // Switching dialect must not leave a script the new dialect cannot parse:
  // the untouched sample becomes the new dialect's sample. A script the user
  // wrote is theirs and stays.
  watch(dialect, (next) => {
    const swapped = sampleForDialect(ddl.value, next)
    if (swapped) ddl.value = swapped
  })

  return {
    ddl,
    modulePath,
    dialect,
    tinyintAsBool,
    cacheEnabled,
    cacheTtlSeconds,
    panePercent,
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
