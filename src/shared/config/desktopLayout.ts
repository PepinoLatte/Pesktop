/**
 * Box 窗口默认尺寸和最小尺寸统一在这里维护，Store 与窗口创建配置保持一致。
 */
export const BOX_WINDOW_SIZE = {
  default: {
    width: 340,
    height: 280,
  },
  min: {
    width: 240,
    height: 184,
  },
} as const;

/**
 * 新建 Box 使用固定起点和错位步长，避免连续创建时窗口完全重叠。
 */
export const BOX_WINDOW_PLACEMENT = {
  initialX: 96,
  initialY: 96,
  cascadeStep: 28,
} as const;

/**
 * Box 菜单尺寸用于约束菜单位置，防止透明窗口外区域不可点击。
 */
export const BOX_CONTEXT_MENU_LAYOUT = {
  width: 190,
  height: 128,
  viewportPadding: 8,
} as const;

/**
 * Box 拖动和缩放保存节流参数统一维护，避免多个窗口交互参数分散。
 */
export const BOX_WINDOW_INTERACTION_TIMING = {
  positionApplyLockMs: 80,
  resizePersistSettleMs: 180,
} as const;

/**
 * 设置窗聚焦同步需要避开标题栏拖动首帧，延迟值集中维护便于后续调优。
 */
export const SETTINGS_WINDOW_SYNC_TIMING = {
  focusSyncDelayMs: 260,
  dragReleaseFallbackMs: 1200,
} as const;

/**
 * 主题过渡只在主动切换时短暂启用，避免启动和拖动时出现多余动画。
 */
export const THEME_TRANSITION_CONFIG = {
  className: "theme-transition",
  timeoutMs: 260,
} as const;
