import { Monitor, Moon, Sun } from "@lucide/vue";
import type { Component } from "vue";
import {
  APP_SETTING_NUMBER_LIMITS,
  type AppSettingNumberKey,
} from "../../../shared/config/appSettings";
import type { DesktopNameDisplayMode, ThemeMode } from "../../../shared/types/desktop";

/**
 * Box 外观数值项限定在当前设置页展示的字段内，吸附距离继续归属窗口行为配置。
 */
export type BoxVisualSettingKey = Exclude<AppSettingNumberKey, "snapThreshold">;

/**
 * 设置页内容宽度统一配置，保证各面板视觉密度一致。
 */
export const SETTINGS_PANEL_WIDTH = {
  default: "max-w-[780px]",
  wide: "max-w-[820px]",
} as const;

/**
 * 主题分段选项与持久化枚举一一对应，避免面板内重复定义。
 */
export const THEME_SEGMENT_OPTIONS: Array<{
  icon: Component;
  label: string;
  value: ThemeMode;
}> = [
  { icon: Sun, label: "浅色", value: "light" },
  { icon: Monitor, label: "跟随系统", value: "system" },
  { icon: Moon, label: "深色", value: "dark" },
];

/**
 * 文件名样式选项按信息量递减排列，让用户从完整到简洁逐步选择。
 */
export const NAME_DISPLAY_SEGMENT_OPTIONS: Array<{
  label: string;
  value: DesktopNameDisplayMode;
}> = [
  { label: "完整名称", value: "full" },
  { label: "隐藏.lnk", value: "hideShortcutExtension" },
  { label: "简洁名称", value: "hideAllExtensions" },
];

/**
 * 吸附距离滑块范围集中维护，避免模板中的输入限制和设置默认值分散。
 */
export const SNAP_THRESHOLD_INPUT = {
  ...APP_SETTING_NUMBER_LIMITS.snapThreshold,
} as const;

/**
 * Box 外观调节项集中维护文案、单位和范围，模板只负责渲染控件。
 */
export const BOX_VISUAL_SETTING_CONTROLS: Array<{
  description: string;
  key: BoxVisualSettingKey;
  label: string;
  max: number;
  min: number;
  step: number;
  unit: string;
}> = [
  {
    key: "boxBackgroundOpacity",
    label: "背景透明度",
    description: "控制 Box 背景与桌面壁纸的融合程度。",
    ...APP_SETTING_NUMBER_LIMITS.boxBackgroundOpacity,
  },
  {
    key: "boxIconSize",
    label: "图标大小",
    description: "调整 Box 内项目图标的显示尺寸。",
    ...APP_SETTING_NUMBER_LIMITS.boxIconSize,
  },
  {
    key: "boxLabelTextSize",
    label: "文字大小",
    description: "调整文件名文字大小，适配不同分辨率。",
    ...APP_SETTING_NUMBER_LIMITS.boxLabelTextSize,
  },
  {
    key: "boxIconGapX",
    label: "横向间距",
    description: "调整图标列之间的水平距离。",
    ...APP_SETTING_NUMBER_LIMITS.boxIconGapX,
  },
  {
    key: "boxIconGapY",
    label: "纵向间距",
    description: "调整图标行之间的垂直距离。",
    ...APP_SETTING_NUMBER_LIMITS.boxIconGapY,
  },
  {
    key: "boxFilenameWidth",
    label: "文件名宽度",
    description: "控制文件名换行宽度，长名称会在此范围内显示。",
    ...APP_SETTING_NUMBER_LIMITS.boxFilenameWidth,
  },
  {
    key: "boxCornerRadius",
    label: "圆角大小",
    description: "调整 Box 面板和图标悬停区域的圆角。",
    ...APP_SETTING_NUMBER_LIMITS.boxCornerRadius,
  },
];
