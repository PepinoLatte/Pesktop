import type { AppSettings, DesktopNameDisplayMode, ThemeMode } from "../types/desktop";

/**
 * 应用设置表和键名统一收口，避免 Store、数据库和设置面板各自硬编码字符串。
 */
export const APP_SETTINGS_STORAGE = {
  databaseUrl: "sqlite:dasktop.db",
  tables: {
    appSettings: "app_settings",
    boxItems: "box_items",
    boxes: "boxes",
  },
} as const;

/**
 * 设置键名使用对象常量承载，调用保存逻辑时始终复用同一份字段定义。
 */
export const APP_SETTING_KEYS = {
  doubleClickOpenItems: "doubleClickOpenItems",
  nameDisplayMode: "nameDisplayMode",
  showItemLabels: "showItemLabels",
  showShortcutArrow: "showShortcutArrow",
  snapThreshold: "snapThreshold",
  snapToEdges: "snapToEdges",
  theme: "theme",
} as const satisfies Record<keyof AppSettings, keyof AppSettings>;

/**
 * 默认设置只包含当前版本真实生效的字段，旧外观参数不再兼容。
 */
export const DEFAULT_APP_SETTINGS: AppSettings = {
  theme: "system",
  snapToEdges: true,
  snapThreshold: 20,
  showItemLabels: true,
  showShortcutArrow: true,
  doubleClickOpenItems: true,
  nameDisplayMode: "full",
};

/**
 * 支持的设置键由默认设置派生，新增设置时只需要补齐默认值和校验逻辑。
 */
export const SUPPORTED_APP_SETTING_KEYS = Object.values(APP_SETTING_KEYS);

/**
 * 明暗主题枚举和 UI 选项共用同一份值，避免设置读取接受 UI 不存在的状态。
 */
export const THEME_MODES = ["light", "system", "dark"] as const satisfies readonly ThemeMode[];

/**
 * Box 文件名显示模式集中定义，设置页和数据库校验使用同一组可选值。
 */
export const DESKTOP_NAME_DISPLAY_MODES = [
  "full",
  "hideShortcutExtension",
  "hideAllExtensions",
] as const satisfies readonly DesktopNameDisplayMode[];
