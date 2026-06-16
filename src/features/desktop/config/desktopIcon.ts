/**
 * Box 图标的通用尺寸配置，避免模板里散落不可解释的图标数值。
 */
export const DESKTOP_ICON_VIEW = {
  fallbackIconSize: 20,
  openClickDetail: 1,
} as const;

/**
 * 快捷方式角标按 Windows 桌面小白底蓝箭头绘制，和系统快捷方式语义保持一致。
 */
export const WINDOWS_SHORTCUT_BADGE = {
  viewBox: "0 0 16 16",
  backgroundPath:
    "M1.2 14.8h8.7v-2.6H6.5l6.6-6.6 1.7 1.7V1.2H8.7l1.7 1.7-6.6 6.6V6.1H1.2v8.7z",
  arrowPath:
    "M2.4 13.6h6.3v-1.4H5.2l8.4-8.4 1 1V2.1h-2.7l1 1-8.4 8.4V8H2.4v5.6z",
  arrowColor: "#0964d8",
  backgroundColor: "#ffffff",
} as const;
