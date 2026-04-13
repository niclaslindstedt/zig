import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// The zig web UI is served from the root of the zig-serve HTTP server when
// the --web flag is set. During local development, `npm run dev` proxies API
// calls to a separately-running `zig serve` instance.
export default defineConfig({
  plugins: [tailwindcss(), react()],
  base: "/",
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  server: {
    proxy: {
      "/api": {
        target: "http://127.0.0.1:3000",
        changeOrigin: true,
        ws: true,
      },
    },
  },
});
