import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

// @ts-expect-error Tauri dev server 会通过 Node 环境变量注入局域网调试地址。
const host = process.env.TAURI_DEV_HOST;
const srcAliasPath = new URL("./src", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");

export default defineConfig(async () => ({
  plugins: [vue(), tailwindcss()],

  resolve: {
    // 跨窗口与领域模块统一使用 @/ 前缀，避免目录重构后深层相对路径继续扩散。
    alias: {
      "@": srcAliasPath,
    },
  },

  // 保留 Rust 错误输出，避免前端 dev server 清屏后吞掉 Tauri 侧诊断信息。
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Rust 目标目录变动很频繁，忽略它可以减少无意义的前端热更新。
      ignored: ["**/src-tauri/**"],
    },
  },
}));
