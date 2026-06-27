/**
 * Box 自绘文件图标的稳定尺寸配置，收口图标背景、标题间距和拖拽阈值，避免模板中散落不可解释的魔法数。
 */
export const DESKTOP_ICON_VIEW = {
  dragStartThreshold: 5,
  fallbackIconScale: 0.72,
  fallbackIconSize: 20,
  itemBlockPadding: 5,
  itemInlinePadding: 0,
  labelGap: 6,
  openClickDetail: 1,
  renameClickDelayMs: 520,
} as const;
