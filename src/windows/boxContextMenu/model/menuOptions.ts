import type { BoxCollapseMode, DesktopBoxTitlePosition } from "@/entities/desktopBox/types";

/**
 * 菜单中自动收起分段控件的 UI 值，落库时再转换为 Box 的 collapsed 布尔字段。
 */
export type BoxAutoCollapseMode = "always" | "rollup";

/**
 * 收缩形态分段控件的选项：窗口模式保留标题条，图标模式缩成单图标方块。
 */
export const BOX_COLLAPSE_MODE_OPTIONS: Array<{
  label: string;
  value: BoxCollapseMode;
}> = [
  { label: "窗口", value: "window" },
  { label: "图标", value: "icon" },
];

/**
 * Box 菜单里的标题位置使用紧凑分段控件，选项属于单个 Box 的窗口偏好。
 */
export const BOX_TITLE_POSITION_OPTIONS: Array<{
  label: string;
  value: DesktopBoxTitlePosition;
}> = [
  { label: "上方", value: "top" },
  { label: "下方", value: "bottom" },
];

/**
 * 菜单中的分段控件使用固定列宽，防止不同主题字体下文字把菜单撑宽。
 */
export const BOX_MENU_SEGMENT_WIDTH = {
  collapse: 54,
  titlePosition: 54,
} as const;

/**
 * 更多菜单的操作行统一使用同一信息结构：图标槽、标题说明、右侧状态，降低视觉割裂感。
 */
export const BOX_MENU_ACTION_ROW_CLASS =
  "flex min-h-[44px] w-full items-center gap-2 rounded-[8px] px-2 text-left text-[12px] transition-colors hover:bg-[#eef1f6] focus-visible:bg-[#eef1f6] dark:hover:bg-[#2a2d36] dark:focus-visible:bg-[#2a2d36]";

/**
 * 是否自动收起属于当前 Box 自身状态，用开关语义降低配置理解成本。
 */
export const BOX_AUTO_COLLAPSE_OPTIONS: Array<{
  label: string;
  value: BoxAutoCollapseMode;
}> = [
  { label: "关闭", value: "always" },
  { label: "开启", value: "rollup" },
];
