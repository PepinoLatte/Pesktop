import type { DesktopItem, DesktopNameDisplayMode } from "../../../shared/types/desktop";

/**
 * 按当前 Box 文件名显示策略生成展示名称；该逻辑只影响 UI 文本，不改变真实文件名和路径映射。
 */
export function formatDesktopItemDisplayName(
  item: DesktopItem,
  mode: DesktopNameDisplayMode,
): string {
  if (mode === "full" || !item.extension) {
    return item.name;
  }

  if (mode === "hideShortcutExtension" && item.kind !== "shortcut") {
    return item.name;
  }

  const extensionSuffix = `.${item.extension}`;

  return item.name.toLowerCase().endsWith(extensionSuffix.toLowerCase())
    ? item.name.slice(0, -extensionSuffix.length)
    : item.name;
}
