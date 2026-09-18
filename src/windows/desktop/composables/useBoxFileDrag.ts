import { computed, ref } from "vue";
import type { ComputedRef, Ref } from "vue";
import { cursorPosition } from "@tauri-apps/api/window";
import { isPrimaryMouseButtonPressed } from "@/entities/desktopItem/api";
import {
  handleBoxDraggedPathsToDesktop,
  handleBoxDroppedPaths,
} from "@/entities/desktopBox/api";
import {
  BOX_ITEM_DRAG_INTERACTION,
} from "@/entities/desktopBox/layout";
import type { DesktopBox } from "@/entities/desktopBox/types";
import type { DesktopItem } from "@/entities/desktopItem/types";
import type { AppSettings } from "@/entities/appSettings/types";
import {
  notifyBoxFileDrag,
  notifyBoxFileDragAccepted,
  type BoxFileDragPayload,
  type BoxFileDragPreviewOptions,
} from "@/shared/ipc/boxFileDrag";
import { openDragPreviewWindow } from "@/windows/dragPreview/lifecycle";
import { DESKTOP_ICON_VIEW } from "@/windows/desktop/config/desktopIcon";
import type {
  ActiveBoxFileDragState,
  BoxSortInsertionPreview,
  BoxScreenPoint,
} from "@/windows/desktop/model/fileDrag";

/**
 * 文件拖拽只使用当前窗口需要的 Tauri 能力，避免组合式逻辑依赖完整窗口实现。
 */
interface BoxFileDragWindowHandle {
  label: string;
  outerPosition: () => Promise<{ x: number; y: number }>;
  outerSize: () => Promise<{ height: number; width: number }>;
  scaleFactor: () => Promise<number>;
  setFocus: () => Promise<void>;
}

/**
 * 跨窗口文件拖拽依赖当前 Box、选择状态、设置和几个窗口反馈回调。
 */
interface BoxFileDragOptions {
  box: ComputedRef<DesktopBox | undefined>;
  currentWindow: BoxFileDragWindowHandle;
  getDesktopPath: () => string;
  getSettings: () => AppSettings;
  isFileRenaming: () => boolean;
  appendBoxShellItems: (shellIds: string[]) => Promise<void>;
  removeBoxShellItems: (shellIds: string[]) => Promise<void>;
  refreshBoxFolderItems: (options?: { silent?: boolean }) => Promise<void>;
  refreshCollapsedPreviewCloseSchedule: () => void;
  refreshDesktopSnapshot: () => Promise<void>;
  resolveSortInsertionPreview: (
    paths: string[],
    point: BoxScreenPoint,
  ) => BoxSortInsertionPreview | null;
  resolveSelectedItems: () => DesktopItem[];
  selectedPaths: Ref<Set<string>>;
  setDragHoveringBox: (isHovering: boolean) => void;
  setSortInsertionPreview: (preview: BoxSortInsertionPreview | null) => void;
  setLastError: (message: string) => void;
  placeIncomingItemsInCurrentBox: (
    paths: string[],
    preview: BoxSortInsertionPreview | null,
  ) => Promise<void>;
  showSortInsertionPreview: (paths: string[], point: BoxScreenPoint) => void;
  sortItemsInSourceBox: (paths: string[], point: BoxScreenPoint) => Promise<void>;
}

/**
 * Box 文件拖拽状态和事件处理器，供文件网格与窗口生命周期复用。
 */
export interface BoxFileDragState {
  cancelFileDragSession: () => void;
  clearAcceptedFileDragWaiters: () => void;
  consumeSuppressedItemClick: () => boolean;
  handleBoxFileDragPayload: (payload: BoxFileDragPayload) => Promise<void>;
  handleItemPointerDown: (event: PointerEvent, item: DesktopItem) => void;
  handleNativeDragDropEvent: (payload: {
    paths?: string[];
    position?: NativeDropPosition;
    shellItems?: NativeShellDropItem[];
    type: "enter" | "over" | "drop" | "leave";
  }) => Promise<void>;
  isBoxFileDragActive: Readonly<Ref<boolean>>;
  isItemDragging: (item: DesktopItem) => boolean;
  markFileDragAccepted: (sessionId: string) => void;
  resolveScreenPointInCurrentWindow: (screenX: number, screenY: number) => Promise<BoxScreenPoint>;
}

/**
 * 原生 DropTarget 只把可支持的 Shell 虚拟项 ID 传给前端，避免持久化 COM/PIDL 细节。
 */
interface NativeShellDropItem {
  shellId: string;
}

/**
 * Tauri 和自定义 OLE DropTarget 都以上报本窗口内物理坐标为主，前端再换算成 CSS 坐标。
 */
interface NativeDropPosition {
  x: number;
  y: number;
}

/**
 * 原生拖入的 over/drop 事件可能不再携带完整路径，前端在 enter 阶段缓存路径供排序落点复用。
 */
interface NativeDropSessionState {
  paths: string[];
}

/**
 * 原生拖入坐标只用于 Box 内排序，需把窗口物理坐标换成 DOM 使用的逻辑坐标。
 */
interface NativeDropPoint {
  localPoint: BoxScreenPoint;
}

/**
 * 管理 Box 内文件拖拽、拖到其他 Box、拖出到桌面以及拖影窗口同步。
 */
export function useBoxFileDrag(options: BoxFileDragOptions): BoxFileDragState {
  const draggingItemPath = ref<string | null>(null);
  const draggingFilePaths = ref<Set<string>>(new Set());
  const suppressNextItemClick = ref(false);
  const isBoxFileDragActive = computed(() => Boolean(draggingItemPath.value));
  let fileDragState: ActiveBoxFileDragState | null = null;
  let fileDragPollTimer: ReturnType<typeof window.setInterval> | null = null;
  let isFileDragPollPending = false;
  const acceptedFileDragSessionIds = new Set<string>();
  const acceptedFileDragWaiters = new Map<string, () => void>();
  let nativeDropSessionState: NativeDropSessionState | null = null;

  /**
   * 拖拽中所有来源文件都降透明，框选多文件时用户能确认整组选区都进入移动状态。
   */
  function isItemDragging(item: DesktopItem): boolean {
    return draggingFilePaths.value.has(item.path);
  }

  /**
   * 拖拽启动后的 pointerup 会伴随一次 click，消费这次 click 防止选区被重置。
   */
  function consumeSuppressedItemClick(): boolean {
    if (!suppressNextItemClick.value) {
      return false;
    }

    suppressNextItemClick.value = false;
    return true;
  }

  /**
   * Box 内文件拖动使用 pointer 候选态，超过阈值后才广播跨窗口拖拽，避免普通点击误触移动文件。
   */
  function handleItemPointerDown(event: PointerEvent, item: DesktopItem): void {
    if (event.button !== 0 || options.isFileRenaming()) {
      return;
    }

    const startX = event.screenX;
    const startY = event.screenY;
    let hasStarted = false;
    const startDrag = async (): Promise<void> => {
      if (hasStarted || !options.box.value) {
        return;
      }

      hasStarted = true;
      suppressNextItemClick.value = true;
      const dragItems = resolveDragItems(item);
      const dragPaths = dragItems.map((dragItem) => dragItem.path);

      draggingItemPath.value = item.path;
      draggingFilePaths.value = new Set(dragPaths);
      options.selectedPaths.value = new Set(dragPaths);
      fileDragState = {
        item,
        paths: dragPaths,
        preview: resolveFileDragPreviewOptions(),
        sessionId: crypto.randomUUID(),
        sourceBoxId: options.box.value.id,
      };
      clearFileDragPolling();
      fileDragPollTimer = window.setInterval(() => {
        void pollFileDragCursor();
      }, BOX_ITEM_DRAG_INTERACTION.pollIntervalMs);
      await openDragPreviewWindow().catch((error) => {
        options.setLastError(error instanceof Error ? error.message : String(error));
      });
      await emitFileDragPhase("start", Math.round(startX), Math.round(startY));
    };
    const handlePointerMove = (moveEvent: PointerEvent): void => {
      const distance = Math.hypot(moveEvent.screenX - startX, moveEvent.screenY - startY);
      if (distance < DESKTOP_ICON_VIEW.dragStartThreshold) {
        return;
      }

      void startDrag();
      moveEvent.preventDefault();
    };
    const cleanupCandidate = (): void => {
      window.removeEventListener("pointermove", handlePointerMove, { capture: true });
      window.removeEventListener("pointerup", handlePointerRelease, { capture: true });
      window.removeEventListener("pointercancel", handlePointerCancel, { capture: true });
    };
    const handlePointerRelease = (): void => {
      cleanupCandidate();
      if (!hasStarted) {
        return;
      }

      void finishFileDragAtCurrentCursor();
      window.setTimeout(() => {
        suppressNextItemClick.value = false;
      }, 0);
    };
    const handlePointerCancel = (): void => {
      cleanupCandidate();
      cancelFileDragSession();
    };

    window.addEventListener("pointermove", handlePointerMove, { capture: true });
    window.addEventListener("pointerup", handlePointerRelease, { capture: true, once: true });
    window.addEventListener("pointercancel", handlePointerCancel, { capture: true, once: true });
  }

  /**
   * 文件拖拽期间轮询全局鼠标坐标，保证拖出当前 WebView 后目标 Box 仍能收到移动和释放。
   */
  async function pollFileDragCursor(): Promise<void> {
    if (!fileDragState || isFileDragPollPending) {
      return;
    }

    isFileDragPollPending = true;
    try {
      const [cursor, isPressed] = await Promise.all([
        cursorPosition(),
        isPrimaryMouseButtonPressed(),
      ]);
      if (!fileDragState) {
        return;
      }
      if (!isPressed) {
        await finishFileDragAt(cursor.x, cursor.y);
        return;
      }

      await updateSourceSortInsertionPreview(cursor.x, cursor.y);
      await emitFileDragPhase("move", cursor.x, cursor.y);
    } finally {
      isFileDragPollPending = false;
    }
  }

  /**
   * pointerup 在当前窗口内发生时直接读取系统鼠标坐标结束拖拽，避免等待下一帧轮询。
   */
  async function finishFileDragAtCurrentCursor(): Promise<void> {
    const cursor = await cursorPosition().catch(() => undefined);
    if (!cursor) {
      cancelFileDragSession();
      return;
    }

    await finishFileDragAt(cursor.x, cursor.y);
  }

  /**
   * 结束拖拽时广播 drop，并让目标 Box 根据坐标决定是否执行真实文件移动。
   */
  async function finishFileDragAt(screenX: number, screenY: number): Promise<void> {
    if (!fileDragState) {
      return;
    }

    const dragState = fileDragState;
    const sourceDropPoint = await resolveScreenPointInCurrentWindow(screenX, screenY).catch(
      () => undefined,
    );
    const isDroppedInsideSourceBox = Boolean(sourceDropPoint?.inside);
    await emitFileDragPhase("drop", screenX, screenY);
    clearFileDragPolling();
    draggingItemPath.value = null;
    draggingFilePaths.value = new Set();
    options.setSortInsertionPreview(null);
    fileDragState = null;
    if (isDroppedInsideSourceBox && sourceDropPoint) {
      await options.sortItemsInSourceBox(dragState.paths, sourceDropPoint).catch((error) => {
        options.setLastError(error instanceof Error ? error.message : String(error));
      });
    } else {
      const isAcceptedByBox = await waitForFileDragAccepted(dragState.sessionId);
      if (!isAcceptedByBox) {
        await handleFileDragOutToDesktop(dragState);
      } else {
        await removeShellItemsFromSourceBox(dragState.paths);
      }
    }
    void options.refreshBoxFolderItems({ silent: true });
    window.setTimeout(() => {
      void options.refreshBoxFolderItems({ silent: true });
    }, BOX_ITEM_DRAG_INTERACTION.acceptFallbackDelayMs);
  }

  /**
   * 取消拖拽时广播 cancel 并清理本地状态，避免目标 Box 保留 hover 高亮。
   */
  function cancelFileDragSession(): void {
    const dragState = fileDragState;
    clearFileDragPolling();
    draggingItemPath.value = null;
    draggingFilePaths.value = new Set();
    options.setSortInsertionPreview(null);
    acceptedFileDragSessionIds.delete(dragState?.sessionId ?? "");
    fileDragState = null;
    if (dragState) {
      void notifyBoxFileDrag({
        item: dragState.item,
        phase: "cancel",
        paths: dragState.paths,
        preview: dragState.preview,
        screenX: 0,
        screenY: 0,
        sessionId: dragState.sessionId,
        sourceBoxId: dragState.sourceBoxId,
      });
    }
  }

  /**
   * 清理文件拖拽轮询，防止一次拖拽结束后继续发送旧坐标。
   */
  function clearFileDragPolling(): void {
    if (!fileDragPollTimer) {
      return;
    }

    window.clearInterval(fileDragPollTimer);
    fileDragPollTimer = null;
    isFileDragPollPending = false;
  }

  /**
   * 记录目标 Box 的接收回执；等待中的来源窗口会立即停止拖出到桌面的兜底逻辑。
   */
  function markFileDragAccepted(sessionId: string): void {
    acceptedFileDragSessionIds.add(sessionId);
    const waiter = acceptedFileDragWaiters.get(sessionId);
    if (!waiter) {
      return;
    }

    acceptedFileDragWaiters.delete(sessionId);
    waiter();
  }

  /**
   * Drop 事件跨 WebView 派发需要短暂等待；超时仍未被其他 Box 接收时才视为拖到桌面。
   */
  function waitForFileDragAccepted(sessionId: string): Promise<boolean> {
    if (acceptedFileDragSessionIds.has(sessionId)) {
      acceptedFileDragSessionIds.delete(sessionId);
      return Promise.resolve(true);
    }

    return new Promise<boolean>((resolve) => {
      const timeout = window.setTimeout(() => {
        acceptedFileDragWaiters.delete(sessionId);
        acceptedFileDragSessionIds.delete(sessionId);
        resolve(false);
      }, BOX_ITEM_DRAG_INTERACTION.acceptFallbackDelayMs);

      acceptedFileDragWaiters.set(sessionId, () => {
        window.clearTimeout(timeout);
        acceptedFileDragSessionIds.delete(sessionId);
        resolve(true);
      });
    });
  }

  /**
   * 窗口卸载时释放等待回执的闭包，防止开发热更新或关闭 Box 后仍保留旧拖拽状态。
   */
  function clearAcceptedFileDragWaiters(): void {
    acceptedFileDragSessionIds.clear();
    for (const waiter of acceptedFileDragWaiters.values()) {
      waiter();
    }
    acceptedFileDragWaiters.clear();
  }

  /**
   * 拖拽广播统一收口，保证 sessionId、来源 Box 和真实路径始终一致。
   */
  async function emitFileDragPhase(
    phase: BoxFileDragPayload["phase"],
    screenX: number,
    screenY: number,
  ): Promise<void> {
    if (!fileDragState) {
      return;
    }

    await notifyBoxFileDrag({
      item: fileDragState.item,
      phase,
      paths: fileDragState.paths,
      preview: fileDragState.preview,
      screenX,
      screenY,
      sessionId: fileDragState.sessionId,
      sourceBoxId: fileDragState.sourceBoxId,
    });
  }

  /**
   * 所有 Box 都监听文件拖拽，只有鼠标落入当前窗口时才展示 hover 或执行 drop。
   */
  async function handleBoxFileDragPayload(payload: BoxFileDragPayload): Promise<void> {
    if (!options.box.value) {
      return;
    }
    if (payload.phase === "cancel") {
      options.setDragHoveringBox(false);
      options.setSortInsertionPreview(null);
      options.refreshCollapsedPreviewCloseSchedule();
      return;
    }

    const localPoint = await resolveScreenPointInCurrentWindow(payload.screenX, payload.screenY);
    if (!localPoint.inside) {
      options.setDragHoveringBox(false);
      options.setSortInsertionPreview(null);
      if (payload.phase === "drop") {
        options.refreshCollapsedPreviewCloseSchedule();
      }
      return;
    }

    if (payload.phase !== "drop") {
      options.setDragHoveringBox(true);
      options.showSortInsertionPreview(payload.paths, localPoint);
      return;
    }

    options.setDragHoveringBox(false);
    options.refreshCollapsedPreviewCloseSchedule();
    const insertionPreview = options.resolveSortInsertionPreview(payload.paths, localPoint);
    options.setSortInsertionPreview(null);
    if (payload.sourceBoxId === options.box.value.id) {
      return;
    }

    try {
      await notifyBoxFileDragAccepted({
        sessionId: payload.sessionId,
        targetBoxId: options.box.value.id,
      });
      const shellIds = payload.paths
        .filter((path) => path.startsWith("shell::"))
        .map((path) => path.slice("shell::".length));
      const filePaths = payload.paths.filter((path) => !path.startsWith("shell::"));
      const incomingOrderPaths = shellIds.map((shellId) => `shell::${shellId}`);
      if (shellIds.length > 0) {
        await options.appendBoxShellItems(shellIds);
      }

      if (filePaths.length > 0) {
        await focusCurrentBoxForShellDialog();
        incomingOrderPaths.push(
          ...(await handleBoxDroppedPaths(
            options.box.value.folderPath,
            filePaths,
            "move",
            options.getSettings().boxConflictPolicy,
          )),
        );
      }
      await options.refreshBoxFolderItems();
      await options.placeIncomingItemsInCurrentBox(incomingOrderPaths, insertionPreview);
    } catch (error) {
      options.setLastError(error instanceof Error ? error.message : String(error));
    }
  }

  /**
   * 原生拖放同时支持真实文件和 Shell 虚拟项；虚拟项只写 Box 引用，真实文件继续交给 Rust 执行。
   */
  async function handleNativeDragDropEvent(payload: {
    paths?: string[];
    position?: NativeDropPosition;
    shellItems?: NativeShellDropItem[];
    type: "enter" | "over" | "drop" | "leave";
  }): Promise<void> {
    if (payload.type === "enter" || payload.type === "over") {
      options.setDragHoveringBox(true);
      if (payload.type === "enter") {
        startNativeDropSession(payload);
      }
      await updateNativeDropSortInsertionPreview(payload);
      return;
    }

    if (payload.type === "leave") {
      options.setDragHoveringBox(false);
      options.setSortInsertionPreview(null);
      options.refreshCollapsedPreviewCloseSchedule();
      clearNativeDropSession();
      return;
    }

    options.setDragHoveringBox(false);
    options.refreshCollapsedPreviewCloseSchedule();
    const payloadPaths = resolveNativeDropPaths(payload, nativeDropSessionState?.paths ?? []);
    const shellIds = resolveUniqueShellIds([
      ...resolveNativeDropShellIds(payload.shellItems),
      ...resolveShellIdsFromVirtualPaths(payloadPaths),
    ]);
    const filePaths = payloadPaths.filter((path) => !path.startsWith("shell::"));
    const dropPoint = await resolveNativeDropPoint(payload.position).catch(() => null);
    const insertionPreview = dropPoint
      ? options.resolveSortInsertionPreview(payloadPaths, dropPoint.localPoint)
      : null;
    options.setSortInsertionPreview(null);
    clearNativeDropSession();
    if (!options.box.value || (filePaths.length === 0 && shellIds.length === 0)) {
      return;
    }

    try {
      const incomingOrderPaths = shellIds.map((shellId) => `shell::${shellId}`);
      if (shellIds.length > 0) {
        await options.appendBoxShellItems(shellIds);
      }
      if (filePaths.length > 0) {
        await focusCurrentBoxForShellDialog();
        incomingOrderPaths.push(
          ...(await handleBoxDroppedPaths(
            options.box.value.folderPath,
            filePaths,
            options.getSettings().boxDropAction,
            options.getSettings().boxConflictPolicy,
          )),
        );
      }
      await options.refreshBoxFolderItems();
      await options.placeIncomingItemsInCurrentBox(incomingOrderPaths, insertionPreview);
    } catch (error) {
      options.setLastError(error instanceof Error ? error.message : String(error));
    }
  }

  /**
   * 原生拖入不显示自绘拖影，只缓存路径用于后续 over/drop 排序；真正文件处理仍等 drop。
   */
  function startNativeDropSession(payload: {
    paths?: string[];
    shellItems?: NativeShellDropItem[];
  }): void {
    const paths = resolveNativeDropPaths(payload);
    if (!options.box.value || paths.length === 0) {
      nativeDropSessionState = null;
      return;
    }

    nativeDropSessionState = {
      paths,
    };
  }

  /**
   * 原生拖入的 over 事件通常不再携带路径，排序提示使用 enter 阶段缓存的路径兜底。
   */
  async function updateNativeDropSortInsertionPreview(payload: {
    paths?: string[];
    position?: NativeDropPosition;
    shellItems?: NativeShellDropItem[];
  }): Promise<void> {
    const paths = resolveNativeDropPaths(payload, nativeDropSessionState?.paths ?? []);
    if (paths.length === 0) {
      options.setSortInsertionPreview(null);
      return;
    }

    const dropPoint = await resolveNativeDropPoint(payload.position).catch(() => null);
    if (!dropPoint?.localPoint.inside) {
      options.setSortInsertionPreview(null);
      return;
    }

    options.showSortInsertionPreview(paths, dropPoint.localPoint);
  }

  /**
   * 原生拖放结束或离开窗口时清掉路径缓存，避免下一次拖入继承旧排序数据。
   */
  function clearNativeDropSession(): void {
    nativeDropSessionState = null;
  }

  /**
   * 将原生 DropTarget 的本窗口物理坐标转换成 DOM 使用的逻辑坐标，跨 DPI 时排序线仍能跟手。
   */
  async function resolveNativeDropPoint(
    position: NativeDropPosition | undefined,
  ): Promise<NativeDropPoint> {
    if (!position) {
      const cursor = await cursorPosition();
      return {
        localPoint: await resolveScreenPointInCurrentWindow(cursor.x, cursor.y),
      };
    }

    const [windowSize, scaleFactor] = await Promise.all([
      options.currentWindow.outerSize(),
      options.currentWindow.scaleFactor(),
    ]);
    const localX = position.x / scaleFactor;
    const localY = position.y / scaleFactor;

    return {
      localPoint: {
        inside:
          position.x >= 0 &&
          position.y >= 0 &&
          position.x <= windowSize.width &&
          position.y <= windowSize.height,
        x: localX,
        y: localY,
      },
    };
  }

  /**
   * 原生拖放可能同时通过 paths 和 shellItems 暴露系统图标，统一折成路径键后按出现顺序去重。
   */
  function resolveNativeDropPaths(
    payload: {
      paths?: string[];
      shellItems?: NativeShellDropItem[];
    },
    fallbackPaths: string[] = [],
  ): string[] {
    const paths = [
      ...(payload.paths ?? []),
      ...resolveNativeDropShellIds(payload.shellItems).map((shellId) => `shell::${shellId}`),
    ];

    return resolveUniquePaths(paths.length > 0 ? paths : fallbackPaths);
  }

  /**
   * 路径去重要保留首个出现位置，避免多选拖入时拖影数量和排序顺序被 Set 序列化扰动。
   */
  function resolveUniquePaths(paths: string[]): string[] {
    const seenPaths = new Set<string>();
    const uniquePaths: string[] = [];

    for (const path of paths) {
      if (seenPaths.has(path)) {
        continue;
      }

      seenPaths.add(path);
      uniquePaths.push(path);
    }

    return uniquePaths;
  }

  /**
   * 释放点不属于任何 Box 时，把当前拖拽文件按设置处理到 Windows 桌面目录。
   */
  async function handleFileDragOutToDesktop(dragState: {
    paths: string[];
    sessionId: string;
  }): Promise<void> {
    const filePaths = dragState.paths.filter((path) => !path.startsWith("shell::"));
    await removeShellItemsFromSourceBox(dragState.paths);
    if (filePaths.length === 0) {
      return;
    }

    const desktopPath = await resolveDesktopPathForDragOut();
    if (!desktopPath) {
      options.setLastError("无法定位桌面路径，已停止拖出文件");
      return;
    }

    try {
      await focusCurrentBoxForShellDialog();
      await handleBoxDraggedPathsToDesktop(
        desktopPath,
        filePaths,
        options.getSettings().boxDragOutAction,
        options.getSettings().boxConflictPolicy,
      );
    } catch (error) {
      options.setLastError(error instanceof Error ? error.message : String(error));
    }
  }

  /**
   * Box 启动快照通常已包含桌面路径；为空时主动刷新一次，避免拖出功能卡在旧启动状态。
   */
  async function resolveDesktopPathForDragOut(): Promise<string> {
    if (options.getDesktopPath().trim()) {
      return options.getDesktopPath().trim();
    }

    await options.refreshDesktopSnapshot();
    return options.getDesktopPath().trim();
  }

  /**
   * 真实文件操作可能弹出 Windows 冲突处理框，先聚焦当前 Box 可让 Shell 对话框更稳定地出现在前台。
   */
  async function focusCurrentBoxForShellDialog(): Promise<void> {
    await options.currentWindow.setFocus().catch(() => undefined);
  }

  /**
   * 拖动仍在来源 Box 内时实时刷新插入位置提示；离开来源窗口立即清空避免残留蓝线。
   */
  async function updateSourceSortInsertionPreview(screenX: number, screenY: number): Promise<void> {
    if (!fileDragState) {
      return;
    }

    const localPoint = await resolveScreenPointInCurrentWindow(screenX, screenY).catch(
      () => undefined,
    );
    if (!localPoint?.inside) {
      options.setSortInsertionPreview(null);
      return;
    }

    options.showSortInsertionPreview(fileDragState.paths, localPoint);
  }

  /**
   * 将屏幕物理坐标换算到当前无边框窗口，跨 DPI 显示器拖动时命中判断仍保持稳定。
   */
  async function resolveScreenPointInCurrentWindow(
    screenX: number,
    screenY: number,
  ): Promise<BoxScreenPoint> {
    const [position, size, scaleFactor] = await Promise.all([
      options.currentWindow.outerPosition(),
      options.currentWindow.outerSize(),
      options.currentWindow.scaleFactor(),
    ]);
    const inside =
      screenX >= position.x &&
      screenY >= position.y &&
      screenX <= position.x + size.width &&
      screenY <= position.y + size.height;

    return {
      inside,
      x: (screenX - position.x) / scaleFactor,
      y: (screenY - position.y) / scaleFactor,
    };
  }

  /**
   * 拖拽启动时若指针按在已选项上，就拖动当前全部选区；否则先把选区收敛到当前文件。
   */
  function resolveDragItems(pointerItem: DesktopItem): DesktopItem[] {
    if (!options.selectedPaths.value.has(pointerItem.path)) {
      return [pointerItem];
    }

    const selectedItems = options.resolveSelectedItems();
    if (selectedItems.length === 0) {
      return [pointerItem];
    }

    return [
      pointerItem,
      ...selectedItems.filter((selectedItem) => selectedItem.path !== pointerItem.path),
    ];
  }

  /**
   * 原生拖放可能重复上报同一个 Shell 项，进入持久化前按出现顺序去重。
   */
  function resolveNativeDropShellIds(shellItems: NativeShellDropItem[] | undefined): string[] {
    return resolveUniqueShellIds((shellItems ?? []).map((shellItem) => shellItem.shellId));
  }

  /**
   * 原生 DropTarget 会把系统图标折成 `shell::` 虚拟路径，复用 Tauri 标准 paths 通道避免扩展字段丢失。
   */
  function resolveShellIdsFromVirtualPaths(paths: string[]): string[] {
    return paths
      .filter((path) => path.startsWith("shell::"))
      .map((path) => path.slice("shell::".length));
  }

  /**
   * 同一次拖放可能同时携带扩展字段和虚拟路径，入库前按出现顺序去重。
   */
  function resolveUniqueShellIds(shellIds: string[]): string[] {
    const seenShellIds = new Set<string>();
    const uniqueShellIds: string[] = [];

    for (const shellId of shellIds) {
      if (seenShellIds.has(shellId)) {
        continue;
      }

      seenShellIds.add(shellId);
      uniqueShellIds.push(shellId);
    }

    return uniqueShellIds;
  }

  /**
   * Shell 虚拟项没有可复制到桌面的真实文件，拖出或跨 Box 接收后都只移除来源 Box 引用。
   */
  async function removeShellItemsFromSourceBox(paths: string[]): Promise<void> {
    const shellIds = paths
      .filter((path) => path.startsWith("shell::"))
      .map((path) => path.slice("shell::".length));
    if (shellIds.length === 0) {
      return;
    }

    await options.removeBoxShellItems(shellIds);
    await options.refreshBoxFolderItems({ silent: true });
  }

  /**
   * 拖影视觉参数来自当前 Box 设置，保证拖动中图标尺寸、文件名和快捷方式角标保持一致。
   */
  function resolveFileDragPreviewOptions(): BoxFileDragPreviewOptions {
    const settings = options.getSettings();

    return {
      iconSize: settings.boxIconSize,
      labelTextSize: settings.boxLabelTextSize,
      labelWidth: settings.boxFilenameWidth,
      nameDisplayMode: settings.nameDisplayMode,
      radiusSize: settings.boxCornerRadius,
      showItemLabels: settings.showItemLabels,
      showShortcutArrow: settings.showShortcutArrow,
    };
  }

  return {
    cancelFileDragSession,
    clearAcceptedFileDragWaiters,
    consumeSuppressedItemClick,
    handleBoxFileDragPayload,
    handleItemPointerDown,
    handleNativeDragDropEvent,
    isBoxFileDragActive,
    isItemDragging,
    markFileDragAccepted,
    resolveScreenPointInCurrentWindow,
  };
}
