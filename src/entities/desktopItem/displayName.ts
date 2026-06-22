import type { DesktopItem, DesktopNameDisplayMode } from "@/entities/desktopItem/types";

/**
 * 按显示策略生成文件标签；该函数只影响 UI 文本，不参与重命名和真实路径计算。
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
