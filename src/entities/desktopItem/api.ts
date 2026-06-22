import { invoke } from "@tauri-apps/api/core";
import type { DesktopItem } from "@/entities/desktopItem/types";

/**
 * 桌面快照只在文件夹型 Box 删除默认策略里提供桌面路径，不再承载桌面文件列表。
 */
export interface DesktopSnapshot {
  desktopPath: string;
}

/**
 * 读取真实桌面目录快照；真实文件永远不在前端侧移动或改名
 */
export function getDesktopSnapshot(): Promise<DesktopSnapshot> {
  return invoke<DesktopSnapshot>("get_desktop_snapshot");
}

/**
 * 扫描 Box 真实文件夹的直接子项，前端据此自绘文件网格。
 */
export function listBoxFolderItems(folderPath: string): Promise<DesktopItem[]> {
  return invoke<DesktopItem[]>("list_box_folder_items", {
    folderPath,
  });
}

/**
 * 使用系统默认程序打开文件项，保持文件、文件夹和快捷方式与 Explorer 一致。
 */
export function openDesktopItem(path: string): Promise<void> {
  return invoke("open_desktop_item", {
    path,
  });
}

/**
 * 在指定屏幕坐标弹出 Windows Shell 原生右键菜单，菜单内容和命令执行均由系统接管。
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
 * 将 Box 文件项重命名为同目录下的新名称；前端传入的是用户确认后的完整文件名。
 */
export function renameDesktopItem(path: string, newName: string): Promise<void> {
  return invoke("rename_desktop_item", {
    path,
    newName,
  });
}

/**
 * 删除 Box 内选中文件项时走 Windows 回收站，避免误删后无法恢复。
 */
export function deleteDesktopItems(paths: string[]): Promise<void> {
  return invoke("delete_desktop_items", {
    paths,
  });
}

/**
 * Box 窗口注册 Windows 原生 DropTarget，补齐 WebView2 在透明窗口下文件拖入不稳定的问题。
 */
export function registerBoxNativeDropTarget(windowLabel: string): Promise<void> {
  return invoke("register_box_native_drop_target", {
    windowLabel,
  });
}

/**
 * Box 窗口销毁前释放自定义 DropTarget，避免旧窗口句柄继续接收拖放。
 */
export function unregisterBoxNativeDropTarget(windowLabel: string): Promise<void> {
  return invoke("unregister_box_native_drop_target", {
    windowLabel,
  });
}

/**
 * 读取系统当前左键状态，用于跨 Box 拖拽时判断释放点，不依赖单个 WebView 的 pointerup
 */
export function isPrimaryMouseButtonPressed(): Promise<boolean> {
  return invoke<boolean>("is_primary_mouse_button_pressed");
}
