/**
 * 桌面文件类型由后端扫描真实桌面目录得到，前端只负责展示与分组映射。
 */
export type DesktopItemKind = "file" | "folder" | "shortcut" | "unknown";

/**
 * 自绘桌面图标模型，path 是所有持久化映射的稳定主键。
 */
export interface DesktopItem {
  id: string;
  name: string;
  path: string;
  extension: string | null;
  kind: DesktopItemKind;
}

/**
 * Box 是桌面扩展层中的可视分组，不移动真实文件。
 */
export interface DesktopBox {
  id: string;
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
  itemPaths: string[];
}

/**
 * 桌面快照是后端扫描结果，前端基于它刷新 Box 可用项目。
 */
export interface DesktopSnapshot {
  desktopPath: string;
  items: DesktopItem[];
}

/**
 * 设置页只保留浅色、跟随系统和深色三种主题，避免与当前 UI 产生不一致的旧状态。
 */
export type ThemeMode = "light" | "system" | "dark";

/**
 * 应用设置只保存当前版本真实使用的偏好，旧外观参数不再兼容。
 */
export interface AppSettings {
  theme: ThemeMode;
  snapToEdges: boolean;
  snapThreshold: number;
  showItemLabels: boolean;
}
