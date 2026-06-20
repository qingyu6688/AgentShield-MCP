import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// base 用相对路径，便于由 agentshield dashboard 在任意路径下托管
export default defineConfig({
  plugins: [vue()],
  base: "./",
  server: {
    port: 5173,
    // 开发时把 /api 代理到本地 dashboard 后端
    proxy: {
      "/api": "http://127.0.0.1:8787",
    },
  },
  build: {
    outDir: "dist",
    chunkSizeWarningLimit: 900,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes("node_modules")) {
            return undefined;
          }
          if (id.includes("node_modules/echarts")) {
            return "echarts";
          }
          if (id.includes("node_modules/ant-design-vue") || id.includes("node_modules/@ant-design")) {
            return "antd";
          }
          if (id.includes("node_modules/vue") || id.includes("node_modules/@vue")) {
            return "vue";
          }
          return "vendor";
        },
      },
    },
  },
});
