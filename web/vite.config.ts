import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";
import { VitePWA } from "vite-plugin-pwa";
import pkg from "./package.json" with { type: "json" };

// BASE is the folder the site is served from. GitHub Pages serves a project
// site at /<repo>/, so the deploy workflow sets BASE=/skyglow/.
const base = process.env.BASE ?? "/";

export default defineConfig({
  base,
  define: {
    // Shown in the footer.
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  plugins: [
    vue(),
    VitePWA({
      registerType: "prompt",
      includeAssets: ["icon.svg"],
      manifest: {
        name: "Skyglow",
        short_name: "Skyglow",
        description: "When, and in what colours, the sky and clouds light up at sunset and twilight.",
        theme_color: "#1d2540",
        background_color: "#141a2e",
        display: "standalone",
        start_url: base,
        scope: base,
        icons: [
          { src: "icon-192.png", sizes: "192x192", type: "image/png" },
          { src: "icon-512.png", sizes: "512x512", type: "image/png" },
          { src: "icon.svg", sizes: "any", type: "image/svg+xml", purpose: "any" },
        ],
      },
      workbox: {
        // Cache the whole app, including the engine (.wasm), so it works offline.
        globPatterns: ["**/*.{js,css,html,svg,png,wasm,json}"],
        // Map tiles and online search are never cached: they are online-only extras.
        navigateFallback: "index.html",
      },
    }),
  ],
  server: {
    // The app bundles ../examples/places and ../packs at build time.
    fs: { allow: [".."] },
  },
  test: {
    environment: "node",
    include: ["tests/**/*.test.ts"],
  },
});
