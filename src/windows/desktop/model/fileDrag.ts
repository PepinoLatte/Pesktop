import type { DesktopItem } from "@/entities/desktopItem/types";
import type { BoxFileDragPreviewOptions } from "@/shared/ipc/boxFileDrag";

/**
 * 一次 Box 文件拖拽会话的来源状态，结束时用 sessionId 区分拖到其他 Box 还是拖出到桌面。
 */
export interface ActiveBoxFileDragState {
  item: DesktopItem;
  paths: string[];
  preview: BoxFileDragPreviewOptions;
  sessionId: string;
  sourceBoxId: string;
}

/**
 * 屏幕物理坐标换算到当前 Box 窗口后的命中结果，跨 DPI 拖动时由拖拽逻辑统一使用。
 */
export interface BoxScreenPoint {
  inside: boolean;
  x: number;
  y: number;
}
