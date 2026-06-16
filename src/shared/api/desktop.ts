import { invoke } from "@tauri-apps/api/core";
import type { DesktopSnapshot } from "../types/desktop";

/**
 * 读取真实桌面目录快照；真实文件永远不在前端侧移动或改名。
 */
export function getDesktopSnapshot(): Promise<DesktopSnapshot> {
  return invoke<DesktopSnapshot>("get_desktop_snapshot");
}

/**
 * 使用系统默认程序打开桌面项目；前端不判断文件类型，统一交给后端 Shell 入口处理。
 */
export function openDesktopItem(path: string): Promise<void> {
  return invoke("open_desktop_item", { path });
}
