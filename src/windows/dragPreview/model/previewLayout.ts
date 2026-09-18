import type { CSSProperties } from "vue";
import type { BoxFileDragPreviewOptions } from "@/shared/ipc/boxFileDrag";
import { DESKTOP_ICON_VIEW } from "@/windows/desktop/config/desktopIcon";

/**
 * 拖影窗口相对鼠标保留偏移，既能看见拖动对象，又不遮挡用户判断插入位置。
 */
export const DRAG_PREVIEW_OFFSET = {
  x: 6,
  y: 6,
} as const;

/**
 * 透明预览窗额外留出少量画布，避免 Shell 图标阴影和快捷方式角标被窗口边缘裁切。
 */
export const DRAG_PREVIEW_WINDOW_MARGIN = 4;

/**
 * 动态窗口尺寸跟随 Box 图标配置，用户调大图标或文件名宽度时拖影也不会被固定窗口裁掉。
 */
export interface DragPreviewWindowSize {
  height: number;
  width: number;
}

/**
 * 拖影图标宽度和 Box 内图标按钮保持同一计算方式，避免拖动中视觉密度突然变化。
 */
export function resolvePreviewItemWidth(nextPreview: BoxFileDragPreviewOptions): number {
  return Math.max(
    nextPreview.labelWidth,
    nextPreview.iconSize + DESKTOP_ICON_VIEW.itemInlinePadding * 2,
  );
}

/**
 * 文件名行高按字号动态派生，确保小字号仍有可读高度，大字号不会压住下一行。
 */
export function resolvePreviewLabelLineHeight(nextPreview: BoxFileDragPreviewOptions): number {
  return Math.max(14, Math.ceil(nextPreview.labelTextSize * 1.32));
}

/**
 * 拖影最多展示两行名称，尺寸计算必须和视觉样式使用同一来源。
 */
export function resolvePreviewLabelBlockHeight(nextPreview: BoxFileDragPreviewOptions): number {
  return resolvePreviewLabelLineHeight(nextPreview) * 2 + 2;
}

/**
 * 拖影外层样式复用 Box 文件图标的间距规则，降低跨窗口拖动时的视觉跳变。
 */
export function resolvePreviewSurfaceStyle(nextPreview: BoxFileDragPreviewOptions): CSSProperties {
  return {
    borderRadius: `${nextPreview.radiusSize}px`,
    gap: nextPreview.showItemLabels ? `${DESKTOP_ICON_VIEW.labelGap}px` : "0px",
    padding: `${DESKTOP_ICON_VIEW.itemBlockPadding}px ${DESKTOP_ICON_VIEW.itemInlinePadding}px`,
    width: `${resolvePreviewItemWidth(nextPreview)}px`,
  };
}

/**
 * 文件名样式集中在布局模型中，保证渲染组件和窗口尺寸计算不会各自维护一套截断规则。
 */
export function resolvePreviewLabelStyle(nextPreview: BoxFileDragPreviewOptions): CSSProperties {
  const labelLineHeight = resolvePreviewLabelLineHeight(nextPreview);

  return {
    WebkitBoxOrient: "vertical",
    WebkitLineClamp: "2",
    display: "-webkit-box",
    fontSize: `${nextPreview.labelTextSize}px`,
    lineHeight: `${labelLineHeight}px`,
    maxHeight: `${resolvePreviewLabelBlockHeight(nextPreview)}px`,
    paddingBottom: "2px",
    textOverflow: "ellipsis",
    width: `${nextPreview.labelWidth}px`,
  };
}

/**
 * 预览窗外壳按当前 Box 图标配置动态缩放，内部布局仍使用与 Box 图标一致的 padding/gap。
 */
export function resolvePreviewWindowSize(
  nextPreview: BoxFileDragPreviewOptions,
): DragPreviewWindowSize {
  const labelHeight = nextPreview.showItemLabels
    ? DESKTOP_ICON_VIEW.labelGap + resolvePreviewLabelBlockHeight(nextPreview)
    : 0;
  const contentHeight =
    DESKTOP_ICON_VIEW.itemBlockPadding * 2 + nextPreview.iconSize + labelHeight;

  return {
    height: Math.ceil(contentHeight + DRAG_PREVIEW_WINDOW_MARGIN * 2),
    width: Math.ceil(resolvePreviewItemWidth(nextPreview) + DRAG_PREVIEW_WINDOW_MARGIN * 2),
  };
}
