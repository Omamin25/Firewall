import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },
  clearScreen: false,
  server: { strictPort: true, port: 1420 },
  // Pure-logic tests run in Node; component tests opt into a DOM with a
  // `@vitest-environment jsdom` docblock, so the fast suite stays fast.
  test: { environment: "node", restoreMocks: true },
});
