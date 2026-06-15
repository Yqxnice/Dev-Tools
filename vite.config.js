import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { resolve } from "path";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [vue()],
  resolve: {
    alias: {
      "@": resolve(__dirname, "src"),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] }
  },
  build: {
    minify: "terser",
    terserOptions: {
      compress: { drop_console: true, drop_debugger: true }
    },
    rollupOptions: {
      output: {
        manualChunks: {
          'vue-vendor': ['vue', 'pinia', 'pinia-plugin-persistedstate'],
          'naive-ui': ['naive-ui'],
          'tauri': ['@tauri-apps/api', '@tauri-apps/plugin-shell']
        }
      }
    },
    chunkSizeWarningLimit: 1500
  }
}));
