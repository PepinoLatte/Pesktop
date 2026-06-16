import { Monitor, Moon, Sun } from "@lucide/vue";
import type { Component } from "vue";
import type { DesktopNameDisplayMode, ThemeMode } from "../../../shared/types/desktop";

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
  min: 8,
  max: 64,
  step: 1,
} as const;
