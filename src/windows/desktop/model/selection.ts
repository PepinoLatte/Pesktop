/**
 * 文件网格内部坐标使用滚动容器局部坐标，框选滚动后仍能稳定命中图标。
 */
export interface GridPoint {
  x: number;
  y: number;
}

/**
 * 框选和图标命中都归一化成 AABB 矩形，降低选择逻辑对具体网格列数的依赖。
 */
export interface GridRect {
  height: number;
  left: number;
  top: number;
  width: number;
}

/**
 * 统一归一化拖拽矩形，调用方不需要关心用户从哪个方向框选。
 */
export function normalizeRect(start: GridPoint, end: GridPoint): GridRect {
  return {
    height: Math.abs(end.y - start.y),
    left: Math.min(start.x, end.x),
    top: Math.min(start.y, end.y),
    width: Math.abs(end.x - start.x),
  };
}

/**
 * 标准 AABB 矩形相交判断，用于框选区域和图标按钮区域的命中计算。
 */
export function rectsIntersect(leftRect: GridRect, rightRect: GridRect): boolean {
  return (
    leftRect.left < rightRect.left + rightRect.width &&
    leftRect.left + leftRect.width > rightRect.left &&
    leftRect.top < rightRect.top + rightRect.height &&
    leftRect.top + leftRect.height > rightRect.top
  );
}
