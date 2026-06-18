import { invoke } from "@tauri-apps/api/core";
import type { DesktopItem } from "@/entities/desktopItem/types";
import type { DesktopSnapshot } from "@/entities/desktopBox/types";

/**
 * 读取真实桌面目录快照；真实文件永远不在前端侧移动或改名
 */
export function getDesktopSnapshot(): Promise<DesktopSnapshot> {
  return invoke<DesktopSnapshot>("get_desktop_snapshot");
}

/**
 * 按任意真实路径解析项目元信息，支持非桌面目录文件拖入 Box 后继续显示原生图标
 */
export function getDesktopItemsByPaths(paths: string[]): Promise<DesktopItem[]> {
  return invoke<DesktopItem[]>("get_desktop_items_by_paths", { paths });
}

/**
 * 使用系统默认程序打开桌面项目；前端不判断文件类型，统一交给后端 Shell 入口处理
 */
export function openDesktopItem(path: string): Promise<void> {
  return invoke("open_desktop_item", { path });
}

/**
 * 在指定屏幕坐标弹出 Windows Shell 原生右键菜单，菜单内容和命令执行均由系统接管
 */
export function showNativeItemContextMenu(
  path: string,
  screenX: number,
  screenY: number,
): Promise<void> {
  return invoke("show_native_item_context_menu", {
    path,
    screenX: Math.round(screenX),
    screenY: Math.round(screenY),
  });
}

/**
 * 通过 Explorer 桌面图标层窗口切换全部 Windows 原生桌面图标显示状态
 */
export function setNativeDesktopIconsHidden(hidden: boolean): Promise<void> {
  return invoke("set_native_desktop_icons_hidden", {
    hidden,
  });
}

/**
 * 读取系统当前左键状态，用于跨 Box 拖拽时判断释放点，不依赖单个 WebView 的 pointerup
 */
export function isPrimaryMouseButtonPressed(): Promise<boolean> {
  return invoke<boolean>("is_primary_mouse_button_pressed");
}
