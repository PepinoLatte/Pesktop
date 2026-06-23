import { emit, listen, type Event, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * Box 更多菜单运行在可复用独立 WebView 中，打开请求只携带当前 Box，避免每次点击重新创建窗口
 */
export const BOX_CONTEXT_MENU_OPEN_EVENT = "dasktop-box-context-menu-open";

/**
 * Box 更多菜单预载完成后广播 ready，调用方据此避免首个 open 事件在监听注册前丢失
 */
export const BOX_CONTEXT_MENU_READY_EVENT = "dasktop-box-context-menu-ready";

/**
 * Box 更多菜单运行在独立 WebView 中，关闭请求用事件传递以保留退出动画
 */
export const BOX_CONTEXT_MENU_CLOSE_EVENT = "dasktop-box-context-menu-close";

/**
 * 菜单状态事件同步给各 Box 窗口，更多按钮由此实现再次点击关闭而不重新定位
 */
export const BOX_CONTEXT_MENU_STATE_EVENT = "dasktop-box-context-menu-state";

/**
 * 菜单预备事件先让隐藏窗口写入首帧样式，原生窗口 show 时不会露出旧内容或终态内容
 */
export const BOX_CONTEXT_MENU_PREPARED_EVENT = "dasktop-box-context-menu-prepared";

/**
 * ready 标记写入同源 storage，用于跨 WebView 判断单例菜单是否已经完成监听注册
 */
const BOX_CONTEXT_MENU_READY_STORAGE_KEY = "dasktop.box-context-menu.ready";

/**
 * 打开事件只携带目标 Box id，菜单窗口内部自行按最新 Store 状态渲染内容
 */
export interface BoxContextMenuOpenPayload {
  boxId: string;
  /**
   * 打开请求序号用于区分准备阶段和播放阶段，避免旧动画事件覆盖新菜单
   */
  requestId: string;
  /**
   * hidden 阶段只准备 DOM 首帧样式；visible 阶段才真正播放进入动画
   */
  stage: "hidden" | "visible";
}

/**
 * 菜单关闭事件允许指定 Box id；未指定时表示关闭当前激活菜单
 */
export interface BoxContextMenuClosePayload {
  boxId?: string;
}

/**
 * 菜单状态同步时携带激活 Box id，其他 Box 需要据此清理自身按钮状态
 */
export interface BoxContextMenuStatePayload {
  boxId: string;
  isOpen: boolean;
  reason?: "blur" | "request";
}

/**
 * 预备完成回执带上请求序号，避免快速点击时旧回执误解锁新的打开流程
 */
export interface BoxContextMenuPreparedPayload {
  /**
   * 菜单 DOM 完整渲染后的理想逻辑高度，调用方据此动态调整原生窗口高度。
   */
  height: number;
  requestId: string;
}

/**
 * 请求菜单窗口展示指定 Box 的配置内容
 */
export async function requestBoxContextMenuOpen(
  boxId: string,
  requestId: string,
  stage: BoxContextMenuOpenPayload["stage"],
): Promise<void> {
  await emit(BOX_CONTEXT_MENU_OPEN_EVENT, { boxId, requestId, stage });
}

/**
 * 菜单窗口完成首帧隐藏态后通知调用方可以展示原生窗口
 */
export async function notifyBoxContextMenuPrepared(
  requestId: string,
  height: number,
): Promise<void> {
  await emit(BOX_CONTEXT_MENU_PREPARED_EVENT, { height, requestId });
}

/**
 * 菜单窗口初始化完成后通知外部，预载流程会等待该事件再认定窗口可复用
 */
export async function notifyBoxContextMenuReady(): Promise<void> {
  markBoxContextMenuReady();
  await emit(BOX_CONTEXT_MENU_READY_EVENT, {});
}

/**
 * 请求目标 Box 的菜单播放关闭动画，真正隐藏由菜单窗口自身完成
 */
export async function requestBoxContextMenuClose(boxId?: string): Promise<void> {
  await emit(BOX_CONTEXT_MENU_CLOSE_EVENT, { boxId });
}

/**
 * 菜单窗口把打开和关闭状态同步给所有 Box 窗口，避免各窗口用 WebView 是否存在猜状态
 */
export async function notifyBoxContextMenuState(payload: BoxContextMenuStatePayload): Promise<void> {
  await emit(BOX_CONTEXT_MENU_STATE_EVENT, payload);
}

/**
 * 创建新的菜单窗口前清理旧 ready 标记，避免异常关闭后的陈旧状态让 open 事件过早发送
 */
export function clearBoxContextMenuReady(): void {
  window.localStorage.removeItem(BOX_CONTEXT_MENU_READY_STORAGE_KEY);
}

/**
 * 判断菜单窗口是否已经注册事件监听，隐藏预载窗口和普通 Box 窗口可以通过同源 storage 共享该状态
 */
export function isBoxContextMenuReady(): boolean {
  return window.localStorage.getItem(BOX_CONTEXT_MENU_READY_STORAGE_KEY) === "true";
}

/**
 * 菜单窗口 ready 时先写入本地标记，再广播 ready 事件，解决事件监听跨 WebView 时的时序窗口
 */
function markBoxContextMenuReady(): void {
  window.localStorage.setItem(BOX_CONTEXT_MENU_READY_STORAGE_KEY, "true");
}

/**
 * 监听菜单打开请求；组件收到 hidden 阶段准备首帧，visible 阶段播放进入动画
 */
export async function listenBoxContextMenuOpen(
  handler: (event: Event<BoxContextMenuOpenPayload>) => void | Promise<void>,
): Promise<UnlistenFn> {
  return listen<BoxContextMenuOpenPayload>(BOX_CONTEXT_MENU_OPEN_EVENT, handler);
}

/**
 * 监听菜单 ready 事件；预载创建窗口时用它确认 Vue 侧监听已经注册
 */
export async function listenBoxContextMenuReady(
  handler: (event: Event<Record<string, never>>) => void | Promise<void>,
): Promise<UnlistenFn> {
  return listen<Record<string, never>>(BOX_CONTEXT_MENU_READY_EVENT, handler);
}

/**
 * 监听菜单预备完成事件；窗口 helper 用它把 show 调整到内容首帧之后
 */
export async function listenBoxContextMenuPrepared(
  handler: (event: Event<BoxContextMenuPreparedPayload>) => void | Promise<void>,
): Promise<UnlistenFn> {
  return listen<BoxContextMenuPreparedPayload>(BOX_CONTEXT_MENU_PREPARED_EVENT, handler);
}

/**
 * 监听菜单关闭请求；组件收到后应先更新可见状态，再关闭当前窗口
 */
export async function listenBoxContextMenuClose(
  handler: (event: Event<BoxContextMenuClosePayload>) => void | Promise<void>,
): Promise<UnlistenFn> {
  return listen<BoxContextMenuClosePayload>(BOX_CONTEXT_MENU_CLOSE_EVENT, handler);
}

/**
 * 监听菜单状态变化；Box 窗口用它控制更多按钮的切换语义和收缩临时展开状态
 */
export async function listenBoxContextMenuState(
  handler: (event: Event<BoxContextMenuStatePayload>) => void | Promise<void>,
): Promise<UnlistenFn> {
  return listen<BoxContextMenuStatePayload>(BOX_CONTEXT_MENU_STATE_EVENT, handler);
}
