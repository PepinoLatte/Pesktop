import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import "./styles.css";

/**
 * 每个 Tauri WebView 都独立挂载应用实例，Box 窗和设置窗通过 URL 参数选择视图
 */
createApp(App).use(createPinia()).mount("#app");
