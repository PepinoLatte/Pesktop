import type { AppSettings } from "@/entities/appSettings/types";
import { BOX_GRID_LAYOUT, BOX_TITLE_VISIBILITY, BOX_WINDOW_SIZE } from "@/entities/desktopBox/layout";
import type { ResizeDirection } from "../composables/useBoxWindowFrame";
import { DESKTOP_ICON_VIEW } from "../config/desktopIcon";

/**
 * resize 读取到的是完整窗口边界，吸附时需要同时处理位置和尺寸
 */
export interface BoxResizeWindowBounds {
  height: number;
  width: number;
  x: number;
  y: number;
}

/**
 * 图标网格单元尺寸由用户配置实时决定，窗口 resize 时据此换算列数和行数
 */
interface BoxGridCellMetrics {
  height: number;
  width: number;
}

/**
 * 将任意 Box 窗口边界吸附到最近的“列 x 行”网格尺寸，并根据拖拽边保持对侧锚点不漂移
 */
export function resolveBoxResizeGridSnappedBounds(
  bounds: BoxResizeWindowBounds,
  settings: AppSettings,
  resizeDirection: ResizeDirection | null,
): BoxResizeWindowBounds {
  const cell = resolveBoxGridCellMetrics(settings);
  const snappedWidth = resolveSnappedAxisSize({
    cellSize: cell.width,
    gap: settings.boxIconGapX,
    minimumSize: BOX_WINDOW_SIZE.min.width,
    paddingBeforeAndAfter: BOX_GRID_LAYOUT.padding * 2,
    size: bounds.width,
  });
  const snappedHeight = resolveSnappedAxisSize({
    cellSize: cell.height,
    gap: settings.boxIconGapY,
    minimumSize: BOX_WINDOW_SIZE.min.height,
    paddingBeforeAndAfter: BOX_TITLE_VISIBILITY.expandedHeight + BOX_GRID_LAYOUT.padding * 2,
    size: bounds.height,
  });

  return {
    height: snappedHeight,
    width: snappedWidth,
    x: shouldAnchorRightEdge(resizeDirection)
      ? bounds.x + bounds.width - snappedWidth
      : bounds.x,
    y: shouldAnchorBottomEdge(resizeDirection)
      ? bounds.y + bounds.height - snappedHeight
      : bounds.y,
  };
}

/**
 * 图标按钮真实高度来自图标、可选双行标签、按钮内边距和图标/标签间距
 */
function resolveBoxGridCellMetrics(settings: AppSettings): BoxGridCellMetrics {
  const labelLineHeight = Math.max(14, Math.ceil(settings.boxLabelTextSize * 1.32));
  const labelHeight = settings.showItemLabels ? labelLineHeight * 2 + 2 : 0;
  const labelGap = settings.showItemLabels ? DESKTOP_ICON_VIEW.labelGap : 0;

  return {
    height:
      settings.boxIconSize +
      labelGap +
      labelHeight +
      DESKTOP_ICON_VIEW.itemBlockPadding * 2,
    width: Math.max(
      settings.boxFilenameWidth,
      settings.boxIconSize + DESKTOP_ICON_VIEW.itemInlinePadding * 2,
    ),
  };
}

/**
 * 单轴尺寸按最近单元数量吸附，同时保证最终尺寸不会低于 Tauri 窗口最小尺寸
 */
function resolveSnappedAxisSize(options: {
  cellSize: number;
  gap: number;
  minimumSize: number;
  paddingBeforeAndAfter: number;
  size: number;
}): number {
  const stride = options.cellSize + options.gap;
  const rawContentSize = Math.max(options.size - options.paddingBeforeAndAfter, options.cellSize);
  const minimumContentSize = Math.max(
    options.minimumSize - options.paddingBeforeAndAfter,
    options.cellSize,
  );
  const minimumCount = Math.max(1, Math.ceil((minimumContentSize + options.gap) / stride));
  const nearestCount = Math.max(
    minimumCount,
    Math.round((rawContentSize + options.gap) / stride),
  );

  return Math.round(
    options.paddingBeforeAndAfter +
      nearestCount * options.cellSize +
      Math.max(0, nearestCount - 1) * options.gap,
  );
}

/**
 * 从左侧缩放时右边缘应保持在用户拖动后的原位置，吸附只移动左边缘
 */
function shouldAnchorRightEdge(direction: ResizeDirection | null): boolean {
  return direction === "West" || direction === "NorthWest" || direction === "SouthWest";
}

/**
 * 从上方缩放时底边缘应保持在用户拖动后的原位置，吸附只移动上边缘
 */
function shouldAnchorBottomEdge(direction: ResizeDirection | null): boolean {
  return direction === "North" || direction === "NorthEast" || direction === "NorthWest";
}
