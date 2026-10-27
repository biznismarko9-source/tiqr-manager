import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

// Tauri expects a fixed port and strict port usage during dev.
const host = process.env.TAURI_DEV_HOST;

// 2.117.0: `vite build --mode web` (npm run build:web) makes the web screens
// that Web access serves from TIQR on the server (src-tauri/src/web_access.rs):
// the same app, with Tauri's modules swapped for the ones in src/web, which
// talk to TIQR on the server instead of to a window's own backend.
const web = (name: string) => fileURLToPath(new URL(`./src/web/${name}.ts`, import.meta.url));
const WEB_MODULES = [
  { find: /^@tauri-apps\/api\/core$/, replacement: web("core") },
  { find: /^@tauri-apps\/api\/event$/, replacement: web("event") },
  { find: /^@tauri-apps\/api\/window$/, replacement: web("window") },
  { find: /^@tauri-apps\/plugin-dialog$/, replacement: web("dialog") },
  { find: /^@tauri-apps\/plugin-opener$/, replacement: web("opener") },
  { find: /^@tauri-apps\/plugin-process$/, replacement: web("process") },
  { find: /^@tauri-apps\/plugin-updater$/, replacement: web("updater") },
  { find: /^@tauri-apps\/plugin-deep-link$/, replacement: web("deepLink") },
];

// 2.117.0: "Add to Home Screen" on an iPhone - the TIQR icon and a window
// without Safari's bars - and the same for "Install app" on Android.
function homeScreen(): Plugin {
  const icon = (file: string) => readFileSync(fileURLToPath(new URL(`./src-tauri/icons/${file}`, import.meta.url)));
  const manifest = {
    name: "TIQR Manager",
    short_name: "TIQR",
    start_url: "/",
    display: "standalone",
    background_color: "#f8fafc",
    theme_color: "#f8fafc",
    icons: [
      { src: "/icon-192.png", sizes: "192x192", type: "image/png" },
      { src: "/icon-512.png", sizes: "512x512", type: "image/png" },
    ],
  };
  return {
    name: "tiqr-home-screen",
    transformIndexHtml(html) {
      // viewport-fit=cover: the iPhone's notch and home bar report their
      // size (env(safe-area-inset-*)), which the tab bar keeps clear of.
      return html.replace('content="width=device-width, initial-scale=1.0"', 'content="width=device-width, initial-scale=1.0, viewport-fit=cover"').replace(
        "</head>",
        `  <meta name="theme-color" content="#f8fafc" />
    <meta name="apple-mobile-web-app-capable" content="yes" />
    <meta name="apple-mobile-web-app-title" content="TIQR" />
    <link rel="apple-touch-icon" href="/apple-touch-icon.png" />
    <link rel="manifest" href="/manifest.webmanifest" />
  </head>`,
      );
    },
    generateBundle() {
      this.emitFile({ type: "asset", fileName: "apple-touch-icon.png", source: icon("ios/AppIcon-60x60@3x.png") });
      this.emitFile({ type: "asset", fileName: "icon-192.png", source: icon("android/mipmap-xxxhdpi/ic_launcher.png") });
      this.emitFile({ type: "asset", fileName: "icon-512.png", source: icon("icon.png") });
      this.emitFile({ type: "asset", fileName: "manifest.webmanifest", source: JSON.stringify(manifest, null, 2) });
    },
  };
}

export default defineConfig(({ mode }) => {
  const isWeb = mode === "web";
  return {
    plugins: isWeb ? [react(), homeScreen()] : [react()],
    clearScreen: false,
    define: { __TIQR_WEB__: JSON.stringify(isWeb) },
    resolve: isWeb ? { alias: WEB_MODULES } : undefined,
    server: {
      port: 1420,
      strictPort: true,
      host: host || false,
      watch: {
        ignored: ["**/src-tauri/**"],
      },
    },
    envPrefix: ["VITE_", "TAURI_"],
    build: isWeb
      ? {
          // Phones: Safari on iOS 14 and newer.
          outDir: "dist-web",
          emptyOutDir: true,
          target: ["es2020", "safari14", "chrome105"],
          minify: "esbuild",
          sourcemap: false,
        }
      : {
          target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
          minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
          sourcemap: !!process.env.TAURI_ENV_DEBUG,
        },
  };
});
