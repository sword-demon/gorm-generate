import { readFileSync } from "node:fs"
import { fileURLToPath, URL } from "node:url"

import tailwindcss from "@tailwindcss/vite"
import vue from "@vitejs/plugin-vue"
import { configDefaults, defineConfig } from "vitest/config"

// The release gate ties tags to the workspace version (Cargo.toml), so the
// report form prefill reads from there instead of keeping a second copy.
const workspaceCargo = readFileSync(
  new URL("./Cargo.toml", import.meta.url),
  "utf8",
)
const version = /^version = "(.+)"$/m.exec(workspaceCargo)?.[1]
if (!version) throw new Error("workspace version not found in Cargo.toml")

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  define: {
    // Prefilled in the report form.
    __APP_VERSION__: JSON.stringify(version),
  },
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  // Tauri expects a fixed port and manages its own window.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  test: {
    // e2e/ holds Playwright specs; collecting them would call Playwright's
    // test() inside Vitest and fail the run.
    exclude: [...configDefaults.exclude, "e2e/**"],
  },
})
