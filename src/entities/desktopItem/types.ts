/**
 * 桌面文件类型由后端扫描真实桌面目录得到，前端只负责展示与分组映射
 */
export type DesktopItemKind = "file" | "folder" | "shortcut" | "unknown";

/**
 * Box 图标名称显示模式由全局设置控制，避免不同窗口之间出现同一文件命名规则不一致
 */
export type DesktopNameDisplayMode = "full" | "hideShortcutExtension" | "hideAllExtensions";

/**
 * 自绘桌面图标模型，path 是所有持久化映射的稳定主键
 */
export interface DesktopItem {
  id: string;
  name: string;
  path: string;
  extension: string | null;
  kind: DesktopItemKind;
  /**
   * Windows Shell 解析出的原生图标，Box 和桌面列表复用同一份数据以避免拖入后视觉不一致
   */
  iconDataUrl: string | null;
}
