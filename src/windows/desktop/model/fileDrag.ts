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

/**
 * Box 内排序落点只描述展示顺序，不参与真实文件移动，拖拽预览和最终落库必须共用同一语义。
 */
export type BoxSortInsertionPlacement = "after" | "before" | "end";

/**
 * 排序插入提示使用目标路径和相对位置表达，末尾插入允许没有目标路径。
 */
export interface BoxSortInsertionPreview {
  placement: BoxSortInsertionPlacement;
  targetPath: string | null;
}
