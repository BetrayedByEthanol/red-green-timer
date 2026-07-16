import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [svelte()],

  // Tauri expects a fixed port and will fail if it's already in use.
  server: {
    port: 1420,
    strictPort: true,
  },

  // Don't obscure Rust errors with a minified/obfuscated frontend stack trace.
  clearScreen: false,

  // Tauri supports `TAURI_ENV_PLATFORM` and friends for platform-specific builds.
  envPrefix: ["VITE_", "TAURI_ENV_*"],

  build: {
    target:
      process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});
