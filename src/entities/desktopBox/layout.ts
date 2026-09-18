/**
 * Box 窗口默认尺寸和最小尺寸统一在这里维护，Store 与窗口创建配置保持一致
 */
export const BOX_WINDOW_SIZE = {
  default: {
    width: 320,
    height: 320,
  },
  min: {
    width: 240,
    height: 240,
  },
} as const;

/**
 * Box 自身的默认交互偏好只随单个窗口保存，不进入全局 app_settings
 */
export const BOX_DEFAULT_STATE = {
  collapsed: false,
  locked: false,
  titleOpacity: 100,
  titlePosition: "top",
} as const;

/**
 * Box 闲置可见度使用百分比表达；0 表示鼠标未进入 Box 区域时整体不可见
 */
export const BOX_TITLE_OPACITY = {
  max: 100,
  min: 0,
  step: 1,
} as const;

/**
 * 收缩相关的窗口写入需要短暂屏蔽，避免动画过程把中间高度当成用户手动 resize 落库
 */
export const BOX_COLLAPSE_INTERACTION = {
  sizeApplyLockMs: 240,
} as const;

/**
 * 标题区域使用独立高度定义，收缩态只保留这一段作为 hover 展开入口
 */
export const BOX_TITLE_VISIBILITY = {
  expandedHeight: 40,
} as const;

/**
 * 图标态收缩形态的窗口边长（逻辑 px）：Box 闲置时缩成一个小图标方块，
 * 边长略大于最大图标尺寸保证点击热区友好，内容按居中排布渲染
 */
export const BOX_ICON_STATE_SIZE = 56;

/**
 * Box 图标网格使用 Tailwind p-2.5，对应 10px；resize 吸附和收缩动画都依赖这个内容留白
 */
export const BOX_GRID_LAYOUT = {
  padding: 10,
} as const;

/**
 * 新建 Box 使用固定起点和错位步长，避免连续创建时窗口完全重叠
 */
export const BOX_WINDOW_PLACEMENT = {
  initialX: 96,
  initialY: 96,
  cascadeStep: 28,
} as const;

/**
 * Box 菜单只固定宽度和边缘间距；高度由菜单 DOM 内容测量后写入原生窗口。
 */
export const BOX_CONTEXT_MENU_LAYOUT = {
  width: 256,
  /**
   * 打开动画从隐藏窗口预备态切到可见态，时长需要短到点击后立即有反馈
   */
  openAnimationMs: 140,
  closeAnimationMs: 90,
  /**
   * 预备态只等待 Vue 写入首帧透明样式，超时后仍展示窗口，避免菜单点击被异常事件阻塞
   */
  preparedWaitMs: 180,
  triggerGap: 14,
  viewportPadding: 8,
} as const;

/**
 * Box 图标内部拖拽使用全局鼠标轮询和跨窗口事件，间隔需要兼顾手感和 WebView 负载
 */
export const BOX_ITEM_DRAG_INTERACTION = {
  acceptFallbackDelayMs: 260,
  /**
   * 外部 Windows 拖拽离开 Box 后仍要等鼠标释放，避免系统拖拽期间触发窗口收缩/重排
   */
  externalReleaseFallbackMs: 15000,
  externalReleaseMinHoldMs: 320,
  externalReleasePollIntervalMs: 50,
  pollIntervalMs: 16,
  /**
   * Drop 事件跨 WebView 派发需要一个短窗口，来源图标延迟恢复可保证最终插入点仍按折叠布局计算
   */
  sourceLayoutReleaseDelayMs: 80,
} as const;

/**
 * Box 拖动和缩放保存节流参数统一维护，避免多个窗口交互参数分散
 */
export const BOX_WINDOW_INTERACTION_TIMING = {
  /**
   * 菜单窗口失焦可能早于更多按钮 click，短保护期内把这次 click 视为关闭动作而不是重新打开
   */
  menuToggleCloseGuardMs: 180,
  positionApplyLockMs: 80,
  /**
   * 原生 resize 刚启动时 WebView 可能短暂读不到左键按下，先给系统拖拽一点接管时间
   */
  resizeReleaseProbeMinMs: 220,
  /**
   * resize 释放轮询需要连续确认，避免单帧误判导致 Box 闪一下就自动收起
   */
  resizeReleaseProbeStableTicks: 2,
  resizePersistSettleMs: 180,
} as const;

/**
 * 设置窗聚焦同步需要避开标题栏拖动首帧，延迟值集中维护便于后续调优
 */
export const SETTINGS_WINDOW_SYNC_TIMING = {
  focusSyncDelayMs: 260,
  dragReleaseFallbackMs: 1200,
} as const;

/**
 * 主题过渡只在主动切换时短暂启用，避免启动和拖动时出现多余动画
 */
export const THEME_TRANSITION_CONFIG = {
  className: "theme-transition",
  timeoutMs: 260,
} as const;
