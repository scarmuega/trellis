import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { domains } from "./vite/domains.ts";

// The board is a static SPA and a singleton: one instance, many domains
// (decision 0062). It holds no API of its own — it reads each domain's
// `trellis serve` directly, cross-origin, at the address `/board.json`
// resolves from ~/.trellis/board.toml. Nothing is proxied.
export default defineConfig({
  plugins: [react(), tailwindcss(), domains()],
  resolve: {
    alias: {
      "@": "/src",
    },
  },
});
