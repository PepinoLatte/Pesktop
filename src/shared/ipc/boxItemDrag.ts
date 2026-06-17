import { emit, listen, type Event, type UnlistenFn } from "@tauri-apps/api/event";
import type { DesktopItem, DesktopNameDisplayMode } from "@/entities/desktopItem/types";

/**
 * Box 图标拖拽事件在多个独立 WebView 窗口之间同步，真实排序和映射仍由目标 Box 落库。
 */
export const BOX_ITEM_DRAG_EVENT = "dasktop-box-item-drag";

/**
 * 目标 Box 接收 Drop 后回执给来源窗口，来源窗口据此避免把项目误判为拖出 Box。
 */
export const BOX_ITEM_DRAG_ACCEPTED_EVENT = "dasktop-box-item-drag-accepted";

/**
 * 拖拽阶段使用显式枚举，预览窗、来源窗和目标窗可以按阶段各自处理视觉和数据提交。
 */
export type BoxItemDragPhase = "start" | "move" | "drop" | "cancel";

/**
 * 拖影视觉配置跟随来源 Box 的用户偏好，避免拖动时图标大小和真实 Box 不一致。
 */
export interface BoxItemDragPreviewOptions {
  iconSize: number;
  labelTextSize: number;
  labelWidth: number;
  /**
   * 拖影文件名和 Box 内文件名使用同一显示策略，避免隐藏后缀设置在拖动时失效。
   */
  nameDisplayMode: DesktopNameDisplayMode;
  radiusSize: number;
  showItemLabels: boolean;
  showShortcutArrow: boolean;
}

/**
 * 跨窗口拖拽载荷使用屏幕物理坐标，便于每个 Box 按自身 DPI 换算成本地 client 坐标。
 */
export interface BoxItemDragPayload {
  item: DesktopItem;
  phase: BoxItemDragPhase;
  preview: BoxItemDragPreviewOptions;
  screenX: number;
  screenY: number;
  sessionId: string;
  sourceBoxId: string;
}

/**
 * Drop 回执只记录会话和目标 Box，来源窗口不依赖目标窗口的内部排序细节。
 */
export interface BoxItemDragAcceptedPayload {
  sessionId: string;
  targetBoxId: string;
}

/**
 * 广播 Box 图标拖拽状态；接收方必须自行判断坐标是否落入自己的窗口。
 */
export async function notifyBoxItemDrag(payload: BoxItemDragPayload): Promise<void> {
  await emit(BOX_ITEM_DRAG_EVENT, payload);
}

/**
 * 监听跨窗口 Box 图标拖拽状态，用于插入线、拖影和最终 Drop 提交。
 */
export async function listenBoxItemDrag(
  handler: (event: Event<BoxItemDragPayload>) => void | Promise<void>,
): Promise<UnlistenFn> {
  return listen<BoxItemDragPayload>(BOX_ITEM_DRAG_EVENT, handler);
}

/**
 * 广播目标 Box 已经接收 Drop，来源窗口收到后不会再执行拖出删除映射。
 */
export async function notifyBoxItemDragAccepted(
  payload: BoxItemDragAcceptedPayload,
): Promise<void> {
  await emit(BOX_ITEM_DRAG_ACCEPTED_EVENT, payload);
}

/**
 * 监听 Drop 接收回执，来源窗口用它区分“拖到其他 Box”和“拖到桌面外部”。
 */
export async function listenBoxItemDragAccepted(
  handler: (event: Event<BoxItemDragAcceptedPayload>) => void | Promise<void>,
): Promise<UnlistenFn> {
  return listen<BoxItemDragAcceptedPayload>(BOX_ITEM_DRAG_ACCEPTED_EVENT, handler);
}
