import type { Component } from "vue";

/**
 * 设置页的导航键值统一收口，避免侧栏、标题和内容区域各自维护字符串。
 */
export type SettingsSection =
  | "boxes"
  | "appearance"
  | "behavior"
  | "about";

/**
 * 设置页左侧导航项，icon 使用组件类型以便保持 Lucide 图标风格一致。
 */
export interface SettingsNavItem {
  key: SettingsSection;
  label: string;
  icon: Component;
}
