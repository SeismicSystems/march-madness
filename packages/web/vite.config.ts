import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  envDir: "../../",
  define: {
    // Privy's embedded wallet signer uses Buffer.from() internally.
    // This makes the global available in the browser.
    global: "globalThis",
  },
});
