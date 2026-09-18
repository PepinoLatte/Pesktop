import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import "./styles.css";

/**
 * 每个 Tauri WebView 都独立挂载应用实例，Box 窗和设置窗通过 URL 参数选择视图
 */
suppressNativeContextMenuInProduction();
createApp(App).use(createPinia()).mount("#app");

/**
 * 生产包面向普通用户，WebView 默认菜单会暴露浏览器式操作并打断桌面工具体验。
 * 这里只阻止默认菜单，不阻断事件传播，保证文件项右键仍可触发 Windows Shell 菜单。
 */
function suppressNativeContextMenuInProduction(): void {
  if (!import.meta.env.PROD) {
    return;
  }

  window.addEventListener(
    "contextmenu",
    (event) => {
      event.preventDefault();
    },
    { capture: true },
  );
}
