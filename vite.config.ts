import { fileURLToPath, URL } from "node:url"

import tailwindcss from "@tailwindcss/vite"
import vue from "@vitejs/plugin-vue"
import { configDefaults, defineConfig } from "vitest/config"

export default defineConfig({
  plugins: [vue(), tailwindcss()],
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
