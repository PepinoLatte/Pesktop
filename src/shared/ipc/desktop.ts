import { emit, listen, type Event, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppSettings } from "@/entities/appSettings/types";
import type { DesktopItem } from "@/entities/desktopItem/types";
import type { DesktopBox, DesktopBoxItem } from "@/entities/desktopBox/types";

/**
 * 桌面状态同步事件用于连接设置窗和多个独立 Box 窗口，避免各 WebView 的 Pinia 状态互相滞后
 */
export const DESKTOP_STATE_CHANGED_EVENT = "dasktop-desktop-state-changed";

/**
 * Box 窗口完成启动水合和首帧布局后通知设置窗，批量启动时用于统一显示所有窗口
 */
export const BOX_WINDOW_READY_EVENT = "dasktop-box-window-ready";

/**
 * 启动快照请求/响应走 Tauri 事件总线，避免把大量 iconDataUrl 写入 localStorage 触发容量上限
 */
export const DESKTOP_STARTUP_SNAPSHOT_REQUEST_EVENT =
  "dasktop-desktop-startup-snapshot-request";
export const DESKTOP_STARTUP_SNAPSHOT_RESPONSE_EVENT =
  "dasktop-desktop-startup-snapshot-response";

/**
 * 状态变更范围用于接收方判断刷新成本，设置变更只需要重读偏好，Box 变更才需要同步布局映射
 */
export type DesktopStateChangeScope = "settings" | "boxes" | "desktop";

/**
 * 跨窗口事件必须带上来源标识，防止当前窗口收到自己广播后重复读取数据库造成交互抖动
 */
export interface DesktopStateChangedPayload {
  sourceId: string;
  scope: DesktopStateChangeScope;
}

/**
 * ready 事件只携带 Box id，设置窗据此判断当前启动批次中哪些窗口已经可以展示
 */
export interface BoxWindowReadyPayload {
  boxId: string;
}

/**
 * Box 窗口通过 token 向设置窗请求当前启动批次的桌面快照
 */
export interface DesktopStartupSnapshotRequestPayload {
  requestId: string;
  token: string;
}

/**
 * 响应只返回匹配请求的快照；null 表示当前批次已结束或 token 不匹配
 */
export interface DesktopStartupSnapshotResponsePayload {
  requestId: string;
  snapshot: DesktopStartupSnapshot | null;
}

/**
 * 启动快照只在主窗口批量打开 Box 时使用，避免每个 Box WebView 重复扫描桌面和读取 SQLite
 */
export interface DesktopStartupSnapshot {
  boxItems: DesktopBoxItem[];
  boxes: DesktopBox[];
  desktopItems: DesktopItem[];
  desktopPath: string;
  settings: AppSettings;
}

/**
 * 设置窗持有当前批次快照服务，所有 Box 完成启动后应释放监听和闭包中的大对象
 */
export interface DesktopStartupSnapshotServer {
  dispose: () => void;
  token: string;
}

/**
 * 广播桌面扩展状态变更，真实数据仍以 SQLite 为准，事件只负责提示其他窗口重新读取
 */
export async function notifyDesktopStateChanged(payload: DesktopStateChangedPayload): Promise<void> {
  await emit(DESKTOP_STATE_CHANGED_EVENT, payload);
}

/**
 * 监听其他窗口的桌面状态变更，回调内部应从持久化层读取最新数据而不是信任事件载荷
 */
export async function listenDesktopStateChanged(
  handler: (event: Event<DesktopStateChangedPayload>) => void | Promise<void>,
): Promise<UnlistenFn> {
  return listen<DesktopStateChangedPayload>(DESKTOP_STATE_CHANGED_EVENT, handler);
}

/**
 * Box 窗口完成初始化后广播 ready，避免批量启动时窗口一个个露出空白或半初始化状态
 */
export async function notifyBoxWindowReady(boxId: string): Promise<void> {
  await emit(BOX_WINDOW_READY_EVENT, { boxId });
}

/**
 * 设置窗监听 Box ready 状态，用于把隐藏创建的窗口在同一批次统一显示
 */
export async function listenBoxWindowReady(
  handler: (event: Event<BoxWindowReadyPayload>) => void | Promise<void>,
): Promise<UnlistenFn> {
  return listen<BoxWindowReadyPayload>(BOX_WINDOW_READY_EVENT, handler);
}

/**
 * 设置窗创建一次启动快照服务，Box 窗口用 token 请求同一份内存快照
 */
export async function createDesktopStartupSnapshotServer(
  snapshot: DesktopStartupSnapshot,
): Promise<DesktopStartupSnapshotServer> {
  const token = crypto.randomUUID();
  const unlisten = await listen<DesktopStartupSnapshotRequestPayload>(
    DESKTOP_STARTUP_SNAPSHOT_REQUEST_EVENT,
    async ({ payload }) => {
      if (payload.token !== token) {
        return;
      }

      await emit(DESKTOP_STARTUP_SNAPSHOT_RESPONSE_EVENT, {
        requestId: payload.requestId,
        snapshot,
      } satisfies DesktopStartupSnapshotResponsePayload);
    },
  );

  return {
    dispose: unlisten,
    token,
  };
}

/**
 * Box 窗口请求启动快照；超时或监听失败时返回 null，让调用方回退完整初始化
 */
export async function requestDesktopStartupSnapshot(
  token: string,
  timeoutMs = 4_000,
): Promise<DesktopStartupSnapshot | null> {
  const requestId = crypto.randomUUID();

  return new Promise<DesktopStartupSnapshot | null>((resolve) => {
    let unlistenResponse: UnlistenFn | null = null;
    let isSettled = false;
    const timeoutTimer = window.setTimeout(() => settle(null), timeoutMs);

    const settle = (snapshot: DesktopStartupSnapshot | null): void => {
      if (isSettled) {
        return;
      }

      isSettled = true;
      window.clearTimeout(timeoutTimer);
      unlistenResponse?.();
      resolve(snapshot);
    };

    listen<DesktopStartupSnapshotResponsePayload>(
      DESKTOP_STARTUP_SNAPSHOT_RESPONSE_EVENT,
      ({ payload }) => {
        if (payload.requestId === requestId) {
          settle(payload.snapshot);
        }
      },
    )
      .then(async (unlisten) => {
        if (isSettled) {
          unlisten();
          return;
        }

        unlistenResponse = unlisten;
        await emit(DESKTOP_STARTUP_SNAPSHOT_REQUEST_EVENT, {
          requestId,
          token,
        } satisfies DesktopStartupSnapshotRequestPayload);
      })
      .catch(() => {
        settle(null);
      });
  });
}
