import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// The simulation worker imports the wasm-bindgen `web` target bundle from
// src/wasm/generated, which `npm run build:wasm` produces. Keep `target` at a
// modern baseline: the game targets mid-range Android browsers (§10.5).
export default defineConfig({
  plugins: [react()],
  build: {
    target: "es2022",
    sourcemap: true,
  },
  worker: {
    format: "es",
  },
  test: {
    environment: "node",
  },
});
