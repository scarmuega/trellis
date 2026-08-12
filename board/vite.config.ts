import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// The board is a static SPA over the read-only surface of `trellis serve`.
// In dev, proxy the API to the daemon — default port 7357, loopback;
// override with TRELLIS_API to aim at a daemon serving a different domain:
//   TRELLIS_API=http://127.0.0.1:7358 npm run dev
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": "/src",
    },
  },
  server: {
    proxy: {
      "/api": process.env.TRELLIS_API ?? "http://127.0.0.1:7357",
    },
  },
});
