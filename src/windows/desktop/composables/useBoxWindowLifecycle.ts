import { nextTick, onMounted, onUnmounted, watch } from "vue";
import type { ComputedRef } from "vue";
import type { UnlistenFn } from "@tauri-apps/api/event";
import type { DragDropEvent, Window } from "@tauri-apps/api/window";
import { registerBoxNativeDropTarget, unregisterBoxNativeDropTarget } from "@/entities/desktopItem/api";
import type { DesktopBox } from "@/entities/desktopBox/types";
import { preloadBoxContextMenuWindow } from "@/entities/desktopBox/windows";
import { listenBoxContextMenuState } from "@/shared/ipc/boxContextMenu";
import { notifyBoxWindowReady } from "@/shared/ipc/desktop";
import {
  listenBoxFileDrag,
  listenBoxFileDragAccepted,
  type BoxFileDragPayload,
} from "@/shared/ipc/boxFileDrag";

/**
 * 生命周期层只编排窗口启动和清理，不直接实现文件、拖拽或几何业务。
 */
interface BoxWindowLifecycleOptions {
  animateBoxIdleOpacity: (shouldAnimate: boolean) => void;
  applyCollapseWindowSize: (shouldAnimate: boolean) => Promise<void>;
  box: ComputedRef<DesktopBox | undefined>;
  boxId: string;
  boxIdleOpacity: ComputedRef<number>;
  canResizeBox: ComputedRef<boolean>;
  cancelFileDragSession: () => void;
  clearAcceptedFileDragWaiters: () => void;
  clearCollapseWindowAnimation: () => void;
  clearResizePersistState: () => void;
  closeContextMenu: () => void;
  currentWindow: Window;
  ensureWindowInsideMonitor: () => Promise<void>;
  handleBoxContextMenuState: (
    boxId: string,
    isOpen: boolean,
    reason?: "blur" | "request",
  ) => void;
  handleBoxFileDragPayload: (payload: BoxFileDragPayload) => Promise<void>;
  handleGlobalFileViewKeydown: (event: KeyboardEvent) => void;
  handleNativeDragDropEvent: (payload: {
    paths?: string[];
    type: "enter" | "over" | "drop" | "leave";
  }) => Promise<void>;
  handleWindowMoved: (x: number, y: number) => Promise<void>;
  initializeStore: () => Promise<void>;
  initializeStoreFromSnapshot: (token: string) => Promise<boolean>;
  isBoxCollapsedToTitle: ComputedRef<boolean>;
  markFileDragAccepted: (sessionId: string) => void;
  refreshBoxFolderItems: (options?: { silent?: boolean }) => Promise<void>;
  scheduleResizePersist: () => void;
  setLastError: (message: string) => void;
  startFolderRefreshPolling: () => void;
  stopFolderRefreshPolling: () => void;
  stopManualDragging: (shouldPersist: boolean) => void;
  stopSelectionRectangle: () => void;
  syncNativeWindowResizable: () => void;
  syncWindowBoundsFromStore: () => Promise<void>;
}

/**
 * 绑定 Box 窗口的启动初始化、Tauri 事件监听和卸载清理，避免壳组件继续膨胀。
 */
export function useBoxWindowLifecycle(options: BoxWindowLifecycleOptions): void {
  const searchParams = new URLSearchParams(window.location.search);
  const unlistenFns: UnlistenFn[] = [];

  watch(options.isBoxCollapsedToTitle, () => {
    void options.applyCollapseWindowSize(true);
  });

  watch(
    () => options.box.value?.titlePosition,
    () => {
      void options.applyCollapseWindowSize(true);
    },
  );

  watch(
    () => options.box.value?.folderPath,
    () => {
      void options.refreshBoxFolderItems();
    },
  );

  watch(options.boxIdleOpacity, () => {
    options.animateBoxIdleOpacity(true);
  });

  watch(
    options.canResizeBox,
    () => {
      options.syncNativeWindowResizable();
    },
    { immediate: true },
  );

  onMounted(async () => {
    if (!(await initializeDesktopStoreForBoxWindow())) {
      await options.initializeStore();
    }
    await options.syncWindowBoundsFromStore();
    await options.applyCollapseWindowSize(false);
    await nextTick();
    options.animateBoxIdleOpacity(false);
    options.syncNativeWindowResizable();
    await options.refreshBoxFolderItems();
    options.startFolderRefreshPolling();
    void preloadBoxContextMenuWindow().catch(reportError);

    unlistenFns.push(
      await options.currentWindow.onMoved(async ({ payload }) => {
        await options.handleWindowMoved(payload.x, payload.y);
      }),
    );
    unlistenFns.push(
      await listenBoxContextMenuState(({ payload }) => {
        options.handleBoxContextMenuState(payload.boxId, payload.isOpen, payload.reason);
      }),
    );
    unlistenFns.push(await options.currentWindow.onResized(() => options.scheduleResizePersist()));
    unlistenFns.push(
      await options.currentWindow.onScaleChanged(async () => {
        await options.ensureWindowInsideMonitor();
      }),
    );
    unlistenFns.push(
      await options.currentWindow.onDragDropEvent(async ({ payload }) => {
        await options.handleNativeDragDropEvent(normalizeDragDropPayload(payload));
      }),
    );
    unlistenFns.push(
      await listenBoxFileDrag(async ({ payload }) => {
        await options.handleBoxFileDragPayload(payload);
      }),
    );
    unlistenFns.push(
      await listenBoxFileDragAccepted(({ payload }) => {
        options.markFileDragAccepted(payload.sessionId);
      }),
    );
    await registerBoxNativeDropTarget(options.currentWindow.label).catch(reportError);
    window.addEventListener("keydown", options.handleGlobalFileViewKeydown);

    void notifyBoxWindowReady(options.boxId).catch(reportError);
  });

  onUnmounted(() => {
    options.stopFolderRefreshPolling();
    options.stopSelectionRectangle();
    options.cancelFileDragSession();
    options.clearAcceptedFileDragWaiters();
    window.removeEventListener("keydown", options.handleGlobalFileViewKeydown);
    void unregisterBoxNativeDropTarget(options.currentWindow.label).catch(() => undefined);
    options.stopManualDragging(false);
    options.closeContextMenu();
    options.clearCollapseWindowAnimation();
    options.clearResizePersistState();
    for (const unlisten of unlistenFns) {
      unlisten();
    }
  });

  /**
   * Box 窗口优先消费设置页准备好的启动快照，失败时再走完整初始化兜底。
   */
  async function initializeDesktopStoreForBoxWindow(): Promise<boolean> {
    const startupSnapshotToken = searchParams.get("startupSnapshot");
    if (!startupSnapshotToken) {
      return false;
    }

    return options.initializeStoreFromSnapshot(startupSnapshotToken);
  }

  /**
   * Tauri 拖放事件存在多种阶段，这里只保留文件路径和阶段给文件拖拽逻辑处理。
   */
  function normalizeDragDropPayload(payload: DragDropEvent): {
    paths?: string[];
    type: "enter" | "over" | "drop" | "leave";
  } {
    return payload as {
      paths?: string[];
      type: "enter" | "over" | "drop" | "leave";
    };
  }

  /**
   * 监听注册阶段的错误统一回写到 Store，避免多个 catch 分支复制 unknown 处理。
   */
  function reportError(error: unknown): void {
    options.setLastError(error instanceof Error ? error.message : String(error));
  }
}
