import { emit, listen, type Event, type UnlistenFn } from "@tauri-apps/api/event";
import type { DesktopItem, DesktopNameDisplayMode } from "@/entities/desktopItem/types";

/**
 * Box 文件拖拽事件在多个独立 WebView 窗口之间同步，真实文件移动由目标 Box 执行。
 */
const BOX_FILE_DRAG_EVENT = "dasktop-box-file-drag";

/**
 * 目标 Box 接收 Drop 后回执给来源窗口，来源窗口据此避免同一次拖拽再落到桌面。
 */
const BOX_FILE_DRAG_ACCEPTED_EVENT = "dasktop-box-file-drag-accepted";

/**
 * 拖拽阶段使用显式枚举，目标 Box 可以按阶段处理 hover、清理和最终 drop。
 */
export type BoxFileDragPhase = "start" | "move" | "drop" | "cancel";

/**
 * 拖影视觉配置跟随来源 Box 的用户偏好，避免拖动时图标大小和真实 Box 不一致。
 */
export interface BoxFileDragPreviewOptions {
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
 * 跨窗口拖拽载荷使用屏幕物理坐标，便于每个 Box 按自身 DPI 换算成本地坐标。
 */
export interface BoxFileDragPayload {
  item: DesktopItem;
  phase: BoxFileDragPhase;
  paths: string[];
  preview: BoxFileDragPreviewOptions;
  screenX: number;
  screenY: number;
  sessionId: string;
  sourceBoxId: string;
}

/**
 * Drop 回执只记录会话和目标 Box，来源窗口不依赖目标窗口内部文件操作细节。
 */
export interface BoxFileDragAcceptedPayload {
  sessionId: string;
  targetBoxId: string;
}

/**
 * 广播 Box 文件拖拽状态；接收方必须自行判断坐标是否落入自己的窗口。
 */
export async function notifyBoxFileDrag(payload: BoxFileDragPayload): Promise<void> {
  await emit(BOX_FILE_DRAG_EVENT, payload);
}

/**
 * 监听跨窗口 Box 文件拖拽状态，用于 hover 反馈和最终真实文件移动。
 */
export async function listenBoxFileDrag(
  handler: (event: Event<BoxFileDragPayload>) => void | Promise<void>,
): Promise<UnlistenFn> {
  return listen<BoxFileDragPayload>(BOX_FILE_DRAG_EVENT, handler);
}

/**
 * 广播目标 Box 已经接收 Drop，来源窗口收到后不会执行拖出到桌面的兜底操作。
 */
export async function notifyBoxFileDragAccepted(
  payload: BoxFileDragAcceptedPayload,
): Promise<void> {
  await emit(BOX_FILE_DRAG_ACCEPTED_EVENT, payload);
}

/**
 * 监听 Box 文件拖拽接收回执，用于区分“拖到其他 Box”和“拖出到桌面”。
 */
export async function listenBoxFileDragAccepted(
  handler: (event: Event<BoxFileDragAcceptedPayload>) => void | Promise<void>,
): Promise<UnlistenFn> {
  return listen<BoxFileDragAcceptedPayload>(BOX_FILE_DRAG_ACCEPTED_EVENT, handler);
}
