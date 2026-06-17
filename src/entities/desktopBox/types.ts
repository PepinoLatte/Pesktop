import type { DesktopItem } from "@/entities/desktopItem/types";

/**
 * Box 标题位置由窗口菜单控制，每个 Box 可独立保存自己的布局偏好。
 */
export type DesktopBoxTitlePosition = "top" | "bottom";

/**
 * Box 内拖拽排序的插入方向，用于在目标图标左右两侧显示位置提示。
 */
export type DesktopBoxItemDropPlacement = "before" | "after";

/**
 * Box 是桌面扩展层中的可视分组，不移动真实文件。
 */
export interface DesktopBox {
  /**
   * 收缩模式开启后，鼠标离开 Box 会折叠到只剩标题栏，hover 标题时临时展开。
   */
  collapsed: boolean;
  id: string;
  /**
   * 锁定后禁止移动和改变窗口大小，但 Box 内文件操作仍然可用。
   */
  locked: boolean;
  title: string;
  /**
   * Box 闲置可见度，0 表示鼠标未进入 Box 区域时整体透明；hover 后恢复完全可见。
   */
  titleOpacity: number;
  /**
   * 标题位置属于单个 Box 的布局偏好，不参与全局设置同步。
   */
  titlePosition: DesktopBoxTitlePosition;
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * Box 与桌面项目的关联独立建模，便于按 Box 或路径快速查询、删除和去重。
 */
export interface DesktopBoxItem {
  boxId: string;
  itemPath: string;
  /**
   * Box 内的手动排序序号，值越小越靠前。
   */
  orderIndex: number;
}

/**
 * 桌面快照是后端扫描结果，前端基于它刷新 Box 可用项目。
 */
export interface DesktopSnapshot {
  desktopPath: string;
  items: DesktopItem[];
}
