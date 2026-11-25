// Writing the visible files to a picker-confirmed directory, including the
// one retry the user can authorise after seeing the conflict list.

import { ref } from "vue"

import { invoke } from "@tauri-apps/api/core"
import { confirm } from "@tauri-apps/plugin-dialog"

import { exportFailureText, toExportFailure } from "@/lib/ipc"

import type { ProjectState } from "./useProject"

export function useExport(project: Pick<ProjectState, "project" | "visibleFiles">) {
  const exporting = ref(false)
  const exportMsg = ref("")

  function reset() {
    exportMsg.value = ""
  }

  async function exportToDir() {
    if (!project.project.value || exporting.value || !project.visibleFiles.value.length) {
      return
    }
    exporting.value = true
    exportMsg.value = ""
    const files = project.visibleFiles.value
    try {
      // The directory comes from Rust's native picker; the renderer never picks
      // the write target itself.
      const dir = await invoke<string | null>("pick_export_dir")
      if (!dir) return
      try {
        await invoke<void>("export_project", { files, overwrite: false })
      } catch (cause) {
        const failure = toExportFailure(cause)
        // Conflicts are the one failure the user can decide to override.
        if (failure.kind !== "conflicts") throw failure
        const proceed = await confirm(
          `${failure.paths.length} file(s) already exist and will be overwritten:\n\n` +
            failure.paths.join("\n"),
        )
        if (!proceed) {
          exportMsg.value = "Export cancelled"
          return
        }
        await invoke<void>("export_project", { files, overwrite: true })
      }
      exportMsg.value = `Exported ${files.length} file(s) to ${dir}`
    } catch (cause) {
      exportMsg.value = `Export failed: ${exportFailureText(toExportFailure(cause))}`
    } finally {
      exporting.value = false
    }
  }

  return { exporting, exportMsg, exportToDir, reset }
}
