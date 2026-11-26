// ساخت پیش‌نمایش تک‌فایلی از همان کد React برنامه (بدون Tauri).
// اجرا: node preview/build.mjs   (به esbuild و react نیاز دارد)
import { build } from "esbuild";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
// fileURLToPath: روی ویندوز هم مسیر درست می‌دهد (pathname آن‌جا «/C:/…» است)
const here = fileURLToPath(new URL(".", import.meta.url)).replaceAll("\\", "/");
const shim = here + "shims/tauri.js";
const r = await build({
  entryPoints: [here + "../src/main.tsx"],
  bundle: true, minify: true, write: false, format: "iife", jsx: "automatic",
  loader: { ".css": "css" }, outdir: here + "out",
  alias: {
    "@tauri-apps/api/core": shim, "@tauri-apps/plugin-dialog": shim,
    "@tauri-apps/plugin-fs": shim, "@tauri-apps/plugin-notification": shim,
  },
  plugins: [
    { name: "fonts", setup(b) { b.onResolve({ filter: /^@fontsource\// }, () => ({ path: here + "shims/empty.css" })); } },
    // مثل Vite: ?raw متن فایل را برمی‌گرداند
    { name: "raw", setup(b) {
      b.onResolve({ filter: /\?raw$/ }, (a) => ({ path: join(a.resolveDir, a.path.slice(0, -4)), namespace: "raw" }));
      b.onLoad({ filter: /.*/, namespace: "raw" }, (a) => ({ contents: readFileSync(a.path, "utf8"), loader: "text" }));
    } },
  ],
  define: { "process.env.NODE_ENV": '"production"' },
  nodePaths: (process.env.NODE_PATH || "").split(":").filter(Boolean),
});
const js = r.outputFiles.find((f) => f.path.endsWith(".js")).text.replace(/<\/script/gi, "<\\/script");
const css = r.outputFiles.find((f) => f.path.endsWith(".css"))?.text ?? "";
const html = readFileSync(here + "../index.html", "utf8")
  .replace('<script type="module" src="/src/main.tsx"></script>', () => `<script>${js}</script>`)
  .replace("</head>", () => `<style>${css}</style></head>`);
writeFileSync(here + "netdesk-preview.html", html);
console.log("ok", (html.length / 1024).toFixed(0), "KB");
