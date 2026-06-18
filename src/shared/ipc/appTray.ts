import { listen, type Event, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * 托盘菜单请求隐藏 main WebView 创建 Box，前端复用现有 Store 和窗口打开逻辑
 */
export const TRAY_CREATE_BOX_EVENT = "dasktop://tray-create-box";

/**
 * 系统自启状态可能由托盘或设置页修改，通过事件让所有设置入口显示同一状态
 */
export const AUTOSTART_CHANGED_EVENT = "dasktop://autostart-changed";

/**
 * 托盘菜单请求把 Explorer 原生桌面图标层切换到指定状态，最终写入仍由设置 Store 完成
 */
export const TRAY_TOGGLE_NATIVE_DESKTOP_ICONS_HIDDEN_EVENT =
  "dasktop://tray-toggle-native-desktop-icons-hidden";

/**
 * 监听托盘创建 Box 请求；处理方应创建 Box 并打开独立 Box 窗口
 */
export async function listenTrayCreateBox(
  handler: (event: Event<Record<string, never>>) => void | Promise<void>,
): Promise<UnlistenFn> {
  return listen<Record<string, never>>(TRAY_CREATE_BOX_EVENT, handler);
}

/**
 * 监听系统自启状态变化，避免托盘菜单和设置页开关互相滞后
 */
export async function listenAutostartChanged(
  handler: (event: Event<boolean>) => void | Promise<void>,
): Promise<UnlistenFn> {
  return listen<boolean>(AUTOSTART_CHANGED_EVENT, handler);
}

/**
 * 监听托盘隐藏系统桌面图标请求；payload 是托盘当前勾选后的目标状态
 */
export async function listenTrayToggleNativeDesktopIconsHidden(
  handler: (event: Event<boolean>) => void | Promise<void>,
): Promise<UnlistenFn> {
  return listen<boolean>(TRAY_TOGGLE_NATIVE_DESKTOP_ICONS_HIDDEN_EVENT, handler);
}
