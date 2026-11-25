// @vitest-environment happy-dom
import { describe, expect, it } from "vitest"
import { ref } from "vue"

import { MAX_PANE_PERCENT, MIN_PANE_PERCENT, usePaneSplit } from "./usePaneSplit"

/** A container stub: the layout engine is not what these tests are about. */
function fakeContainer(left: number, width: number): HTMLElement {
  return {
    getBoundingClientRect: () => ({ left, width, top: 0, height: 0, right: left + width, bottom: 0 }),
  } as unknown as HTMLElement
}

function pointer(clientX: number, extra: Partial<PointerEvent> = {}): PointerEvent {
  return { pointerId: 1, clientX, preventDefault: () => {}, ...extra } as unknown as PointerEvent
}

describe("usePaneSplit", () => {
  it("places the divider at the pointer while dragging", () => {
    const percent = ref(50)
    const container = ref<HTMLElement | null>(fakeContainer(0, 1000))
    const split = usePaneSplit(percent, container)

    split.onPointerDown(pointer(300))
    expect(percent.value).toBe(30)

    split.onPointerMove(pointer(700))
    expect(percent.value).toBe(70)

    split.onPointerUp()
    // The pointer no longer moves the divider once the drag is over.
    split.onPointerMove(pointer(100))
    expect(percent.value).toBe(70)
  })

  it("follows the pointer through the window while dragging", () => {
    const percent = ref(50)
    const container = ref<HTMLElement | null>(fakeContainer(0, 1000))
    const split = usePaneSplit(percent, container)

    split.onPointerDown(pointer(300))
    window.dispatchEvent(new MouseEvent("pointermove", { clientX: 600 }))
    expect(percent.value).toBe(60)
    expect(split.dragging.value).toBe(true)

    window.dispatchEvent(new MouseEvent("pointerup"))
    expect(split.dragging.value).toBe(false)

    window.dispatchEvent(new MouseEvent("pointermove", { clientX: 100 }))
    expect(percent.value).toBe(60)
  })

  it("keeps both panes usable", () => {
    const percent = ref(50)
    const container = ref<HTMLElement | null>(fakeContainer(0, 1000))
    const split = usePaneSplit(percent, container)

    split.onPointerDown(pointer(-500))
    expect(percent.value).toBe(MIN_PANE_PERCENT)

    split.onPointerDown(pointer(5000))
    expect(percent.value).toBe(MAX_PANE_PERCENT)
  })

  it("ignores a drag when the container has no size", () => {
    const percent = ref(50)
    const container = ref<HTMLElement | null>(fakeContainer(0, 0))
    const split = usePaneSplit(percent, container)

    split.onPointerDown(pointer(300))

    expect(percent.value).toBe(50)
  })

  it("moves the divider with the arrow keys", () => {
    const percent = ref(50)
    const container = ref<HTMLElement | null>(fakeContainer(0, 1000))
    const split = usePaneSplit(percent, container)
    const key = (k: string, shiftKey = false) =>
      split.onKeyDown({ key: k, shiftKey, preventDefault: () => {} } as unknown as KeyboardEvent)

    key("ArrowRight")
    expect(percent.value).toBe(52)

    key("ArrowLeft")
    expect(percent.value).toBe(50)

    key("ArrowRight", true)
    expect(percent.value).toBe(60)

    key("Home")
    expect(percent.value).toBe(MIN_PANE_PERCENT)

    key("End")
    expect(percent.value).toBe(MAX_PANE_PERCENT)
  })

  it("leaves other keys alone", () => {
    const percent = ref(50)
    const container = ref<HTMLElement | null>(fakeContainer(0, 1000))
    const split = usePaneSplit(percent, container)

    split.onKeyDown({ key: "a", shiftKey: false, preventDefault: () => {} } as unknown as KeyboardEvent)

    expect(percent.value).toBe(50)
  })
})
