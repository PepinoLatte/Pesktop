/**
 * Box 图标的通用尺寸配置，避免模板里散落不可解释的图标数值。
 */
export const DESKTOP_ICON_VIEW = {
  /**
   * 图标按钮横向内边距参与网格列宽计算，避免快捷方式标记被裁切。
   */
  itemInlinePadding: 8,
  /**
   * 图标按钮纵向内边距用于给换行文件名预留稳定空间。
   */
  itemBlockPadding: 6,
  /**
   * 文件名和图标之间的固定间距，保持 Windows 桌面图标的紧凑节奏。
   */
  labelGap: 6,
  /**
   * 后备 Lucide 图标最小尺寸，保证没有 Shell 图标时仍然清晰可辨。
   */
  fallbackIconSize: 20,
  /**
   * 后备图标按系统图标尺寸等比缩放，避免线性图标撑满容器后显得过重。
   */
  fallbackIconScale: 0.72,
  /**
   * 单击打开模式只响应真实 click，不把双击的第二次 click 当成单击处理。
   */
  openClickDetail: 1,
  /**
   * 指针处在图标上下少量空隙时仍视为同一行，保证拖到两个图标之间也能显示插入线。
   */
  dragInsertRowTolerance: 12,
  /**
   * 内部排序使用 pointer 拖拽，超过阈值才进入拖动状态，避免普通单击或双击误触排序。
   */
  dragStartThreshold: 5,
} as const;

/**
 * 快捷方式角标按白底 Windows 蓝箭头绘制，保持原生快捷方式语义但减少旧样式留白。
 */
export const WINDOWS_SHORTCUT_BADGE = {
  viewBox: "0 0 16 16",
  arrowPaths: ["M4 12L12 4", "M7 4H12V9"],
  offset: -4,
  overlayRadius: 6,
  overlaySize: 22,
  referenceIconSize: 64,
  shadow: "0 1px 4px rgba(0,0,0,.2)",
  strokeColor: "#0964d8",
  strokeWidth: 2.3,
  /**
   * SVG 折线视觉重心偏右下，渲染时轻微左移以贴近 Windows 角标观感。
   */
  svgOffsetX: -1,
  /**
   * SVG 折线视觉重心偏右下，渲染时轻微上移以贴近 Windows 角标观感。
   */
  svgOffsetY: -0.5,
  svgSize: 18,
} as const;
