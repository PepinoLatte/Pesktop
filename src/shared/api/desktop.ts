import { invoke } from "@tauri-apps/api/core";
import type { DesktopSnapshot } from "../types/desktop";

/**
 * 读取真实桌面目录快照；真实文件永远不在前端侧移动或改名。
 */
export function getDesktopSnapshot(): Promise<DesktopSnapshot> {
  return invoke<DesktopSnapshot>("get_desktop_snapshot");
}
