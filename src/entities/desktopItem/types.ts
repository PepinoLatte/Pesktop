/**
 * Box 文件项类型由 Rust 扫描真实文件夹得到，前端只用它决定后备图标和交互语义。
 */
export type DesktopItemKind = "file" | "folder" | "shell" | "shortcut" | "unknown";

/**
 * Box 文件项来源决定前端动作是调用真实文件命令，还是只维护 Shell 虚拟项引用。
 */
export type DesktopItemSource = "fileSystem" | "shell";

/**
 * 文件名显示模式只影响 Box 内标签文本，不改变真实文件名。
 */
export type DesktopNameDisplayMode = "full" | "hideShortcutExtension" | "hideAllExtensions";

/**
 * 自绘 Box 文件项模型，path 是排序和选择的稳定主键，Shell 项使用 `shell::` 虚拟路径。
 */
export interface DesktopItem {
  extension: string | null;
  iconDataUrl: string | null;
  id: string;
  kind: DesktopItemKind;
  name: string;
  path: string;
  shellId: string | null;
  source: DesktopItemSource;
}
