// The draggable divider between the SQL editor and the result pane.
//
// The split is stored as a percentage of the container width, so it survives a
// window resize: the divider is placed at that fraction of whatever the
// container currently is. While a drag is running the pointer events are
// listened for on the window, so the drag keeps going when the cursor leaves
// the divider.

import { getCurrentScope, onScopeDispose, ref, type Ref } from "vue"

import { clampPanePercent, MAX_PANE_PERCENT, MIN_PANE_PERCENT } from "@/lib/settings"

export { clampPanePercent, MAX_PANE_PERCENT, MIN_PANE_PERCENT }

export function usePaneSplit(
  percent: Ref<number>,
  container: Ref<HTMLElement | null>,
) {
  const dragging = ref(false)

  function setFromClientX(clientX: number) {
    const rect = container.value?.getBoundingClientRect()
    if (!rect || rect.width <= 0) return
    percent.value = clampPanePercent(((clientX - rect.left) / rect.width) * 100)
  }

  function onPointerMove(event: PointerEvent) {
    if (!dragging.value) return
    setFromClientX(event.clientX)
  }

  function onPointerUp() {
    if (!dragging.value) return
    dragging.value = false
    removeWindowListeners()
  }

  function removeWindowListeners() {
    window.removeEventListener("pointermove", onPointerMove)
    window.removeEventListener("pointerup", onPointerUp)
    window.removeEventListener("pointercancel", onPointerUp)
  }

  function onPointerDown(event: PointerEvent) {
    dragging.value = true
    setFromClientX(event.clientX)
    window.addEventListener("pointermove", onPointerMove)
    window.addEventListener("pointerup", onPointerUp)
    window.addEventListener("pointercancel", onPointerUp)
    event.preventDefault()
  }

  /** Arrow keys nudge the divider; Shift makes the step coarse. */
  function onKeyDown(event: KeyboardEvent) {
    const step = event.shiftKey ? 10 : 2
    switch (event.key) {
      case "ArrowLeft":
        percent.value = clampPanePercent(percent.value - step)
        break
      case "ArrowRight":
        percent.value = clampPanePercent(percent.value + step)
        break
      case "Home":
        percent.value = MIN_PANE_PERCENT
        break
      case "End":
        percent.value = MAX_PANE_PERCENT
        break
      default:
        return
    }
    event.preventDefault()
  }

  if (getCurrentScope()) {
    onScopeDispose(() => {
      removeWindowListeners()
    })
  }

  return { dragging, onPointerDown, onPointerMove, onPointerUp, onKeyDown }
}
