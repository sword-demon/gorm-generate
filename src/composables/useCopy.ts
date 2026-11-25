// Clipboard feedback: which item was copied and why a copy failed.

import { ref } from "vue"

export function useCopy() {
  const copied = ref("")
  const copyError = ref("")
  let clearTimer: ReturnType<typeof setTimeout> | undefined

  function reset() {
    copied.value = ""
    copyError.value = ""
  }

  async function copy(text: string, label: string) {
    copyError.value = ""
    try {
      if (!navigator.clipboard?.writeText) throw new Error("Clipboard is unavailable")
      await navigator.clipboard.writeText(text)
      copied.value = label
      clearTimeout(clearTimer)
      clearTimer = setTimeout(() => {
        if (copied.value === label) copied.value = ""
      }, 1500)
    } catch (cause) {
      copyError.value = `Copy failed: ${String(cause)}`
    }
  }

  return { copied, copyError, copy, reset }
}
