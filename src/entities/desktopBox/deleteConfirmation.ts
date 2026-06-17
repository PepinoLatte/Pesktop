import type { DesktopBox } from "./types";

/**
 * 删除 Box 只移除 Dasktop 分组和映射，不会删除真实桌面文件；两个入口共用同一确认文案。
 */
export function confirmDesktopBoxDeletion(box: DesktopBox, itemCount = 0): boolean {
  const title = box.title || "未命名 Box";
  const itemText = itemCount > 0 ? `\n当前 Box 中有 ${itemCount} 个项目会回到未分组状态。` : "";

  return window.confirm(
    `确定删除「${title}」吗？${itemText}\n\n真实文件不会被删除，只会移除这个 Box 和它的收纳映射。`,
  );
}
