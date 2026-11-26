import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";

// ویندوز: وقتی cargo فایل‌های target را قفل می‌کند، watcher خطای EBUSY می‌دهد و
// Vite بدون این handler از کار می‌افتد. خطا فقط گزارش می‌شود.
const tolerateWatchErrors: Plugin = {
  name: "tolerate-watch-errors",
  configureServer(server) {
    server.watcher.on("error", (e) => server.config.logger.warn(`[watch] ${(e as Error).message}`));
  },
};

export default defineConfig({
  plugins: [react(), tolerateWatchErrors],
  clearScreen: false,
  server: {
    // IPv4 صریح: روی ویندوز «localhost» به ::1 می‌رسد و Tauri پورت را روی 127.0.0.1 می‌سنجد
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
    watch: { ignored: (p: string) => /[\\/](src-tauri|crates|target|preview)([\\/]|$)/.test(p) },
  },
  build: { target: "es2021", sourcemap: false },
});
