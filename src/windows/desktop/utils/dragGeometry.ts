import type { CSSProperties } from "vue";
import { DESKTOP_ICON_VIEW } from "../config/desktopIcon";
import type { DesktopBoxItemDropPlacement } from "@/entities/desktopBox/types";

/**
 * Box 内空隙拖拽命中结果同时保存排序目标和插入线坐标，避免竖线挂在某个图标边缘抖动
 */
export interface BoxGridDragInsertTarget {
  indicatorStyle: CSSProperties;
  path: string;
  placement: DesktopBoxItemDropPlacement;
}

/**
 * 拖拽排序只需要 DOM 边界和业务路径，额外属性不进入几何计算以降低组件耦合
 */
interface BoxGridDragCandidate {
  path: string;
  rect: DOMRect;
}

/**
 * 根据同一行图标中心点计算稳定插入槽位，并把竖线放在相邻图标间隙的视觉中心
 */
export function resolveBoxGridDragInsertTarget(
  clientX: number,
  clientY: number,
  container: HTMLElement,
  excludedPath: string,
): BoxGridDragInsertTarget | null {
  const containerRect = container.getBoundingClientRect();
  const candidates = Array.from(
    container.querySelectorAll<HTMLElement>("[data-box-item-path]"),
  )
    .map((iconElement) => ({
      path: iconElement.dataset.boxItemPath ?? "",
      rect: iconElement.getBoundingClientRect(),
    }))
    .filter(({ path }) => path && path !== excludedPath);

  if (candidates.length === 0) {
    return null;
  }

  const rowItems = candidates
    .filter(({ rect }) => {
      const verticalDistance =
        clientY < rect.top ? rect.top - clientY : Math.max(clientY - rect.bottom, 0);

      return verticalDistance <= DESKTOP_ICON_VIEW.dragInsertRowTolerance;
    })
    .sort((left, right) => left.rect.left - right.rect.left);

  if (rowItems.length === 0) {
    return null;
  }

  const insertIndex = rowItems.reduce(
    (count, item) => (clientX > item.rect.left + item.rect.width / 2 ? count + 1 : count),
    0,
  );
  const targetIndex = Math.min(insertIndex, rowItems.length - 1);
  const targetItem = rowItems[targetIndex];
  const placement: DesktopBoxItemDropPlacement =
    insertIndex >= rowItems.length ? "after" : "before";
  const lineX = resolveDragInsertLineX(rowItems, insertIndex);
  const rowTop = Math.min(...rowItems.map((item) => item.rect.top));
  const rowBottom = Math.max(...rowItems.map((item) => item.rect.bottom));

  return {
    indicatorStyle: {
      height: `${Math.max(rowBottom - rowTop - 8, 24)}px`,
      left: `${lineX - containerRect.left + container.scrollLeft}px`,
      top: `${rowTop - containerRect.top + container.scrollTop + 4}px`,
    },
    path: targetItem.path,
    placement,
  };
}

/**
 * 插入线坐标优先取相邻图标的真实间隙中心，首尾位置则贴近目标图标外侧但不挤到内容上
 */
export function resolveDragInsertLineX(
  rowItems: BoxGridDragCandidate[],
  insertIndex: number,
): number {
  if (insertIndex <= 0) {
    return rowItems[0].rect.left - DESKTOP_ICON_VIEW.dragInsertEdgeOffset;
  }

  if (insertIndex >= rowItems.length) {
    return rowItems[rowItems.length - 1].rect.right + DESKTOP_ICON_VIEW.dragInsertEdgeOffset;
  }

  return (rowItems[insertIndex - 1].rect.right + rowItems[insertIndex].rect.left) / 2;
}
