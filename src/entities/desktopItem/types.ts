/**
 * Box 文件项类型由 Rust 扫描真实文件夹得到，前端只用它决定后备图标和交互语义。
 */
export type DesktopItemKind = "file" | "folder" | "shortcut" | "unknown";

/**
 * 文件名显示模式只影响 Box 内标签文本，不改变真实文件名。
 */
export type DesktopNameDisplayMode = "full" | "hideShortcutExtension" | "hideAllExtensions";

/**
 * 自绘 Box 文件项模型，path 是真实文件操作的稳定主键。
 */
export interface DesktopItem {
  extension: string | null;
  iconDataUrl: string | null;
  id: string;
  kind: DesktopItemKind;
  name: string;
  path: string;
}
