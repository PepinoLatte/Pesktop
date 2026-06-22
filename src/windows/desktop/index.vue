<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import type { CSSProperties } from "vue";
import { MoreHorizontal } from "@lucide/vue";
import { cursorPosition, getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import DesktopIcon from "./components/DesktopIcon.vue";
import { DESKTOP_ICON_VIEW } from "./config/desktopIcon";
import { useBoxCollapsePreview } from "./composables/useBoxCollapsePreview";
import { useBoxContextMenu } from "./composables/useBoxContextMenu";
import { useBoxTitleEditing } from "./composables/useBoxTitleEditing";
import { useBoxWindowFrame } from "./composables/useBoxWindowFrame";
import { useDesktopStore } from "@/entities/desktopBox/store";
import {
  deleteDesktopItems,
  isPrimaryMouseButtonPressed,
  listBoxFolderItems,
  openDesktopItem,
  registerBoxNativeDropTarget,
  renameDesktopItem,
  showNativeItemContextMenu,
  unregisterBoxNativeDropTarget,
} from "@/entities/desktopItem/api";
import type { DesktopItem } from "@/entities/desktopItem/types";
import { handleBoxDraggedPathsToDesktop, handleBoxDroppedPaths } from "@/entities/desktopBox/api";
import { preloadBoxContextMenuWindow } from "@/entities/desktopBox/windows";
import { openDragPreviewWindow } from "@/windows/dragPreview/lifecycle";
import { listenBoxContextMenuState } from "@/shared/ipc/boxContextMenu";
import {
  listenBoxFileDragAccepted,
  listenBoxFileDrag,
  notifyBoxFileDragAccepted,
  notifyBoxFileDrag,
  type BoxFileDragPreviewOptions,
  type BoxFileDragPayload,
} from "@/shared/ipc/boxFileDrag";
import { notifyBoxWindowReady } from "@/shared/ipc/desktop";
import {
  BOX_ITEM_DRAG_INTERACTION,
  BOX_WINDOW_INTERACTION_TIMING,
} from "@/entities/desktopBox/layout";

const BOX_FILE_VIEW = {
  refreshIntervalMs: 1400,
} as const;

const props = defineProps<{
  boxId: string;
}>();

const searchParams = new URLSearchParams(window.location.search);
const desktopStore = useDesktopStore();
const currentWindow = getCurrentWindow();
const unlistenFns: UnlistenFn[] = [];
const boxSurfaceRef = ref<HTMLElement | null>(null);
const boxGridRef = ref<HTMLElement | null>(null);
const boxItems = ref<DesktopItem[]>([]);
const selectedPaths = ref<Set<string>>(new Set());
const editingPath = ref<string | null>(null);
const renameDraft = ref("");
const selectionStart = ref<{ x: number; y: number } | null>(null);
const selectionCurrent = ref<{ x: number; y: number } | null>(null);
const draggingItemPath = ref<string | null>(null);
const draggingFilePaths = ref<Set<string>>(new Set());
const suppressNextItemClick = ref(false);
const box = computed(() => desktopStore.boxes.find((item) => item.id === props.boxId));
const isSelecting = computed(() => Boolean(selectionStart.value && selectionCurrent.value));
const isBoxFileDragActive = computed(() => Boolean(draggingItemPath.value));
let readContextMenuOpen = (): boolean => false;
let readEditingTitle = (): boolean => false;
let readBoxCollapsedToTitle = (): boolean => false;
let readCollapseWindowSizeApplying = (): boolean => false;
let closeActiveContextMenu = (): void => undefined;
let openCollapsedPreviewForActiveInteractionHandler = (): void => undefined;
let refreshCollapsedPreviewCloseScheduleHandler = (): void => undefined;
let refreshTimer: ReturnType<typeof window.setInterval> | null = null;
let fileDragState: {
  item: DesktopItem;
  paths: string[];
  preview: BoxFileDragPreviewOptions;
  sessionId: string;
  sourceBoxId: string;
} | null = null;
let fileDragPollTimer: ReturnType<typeof window.setInterval> | null = null;
let isFileDragPollPending = false;
const acceptedFileDragSessionIds = new Set<string>();
const acceptedFileDragWaiters = new Map<string, () => void>();

/**
 * 窗口框架层继续负责位置、尺寸、吸附和 resize，文件区不再参与原生子窗口同步。
 */
const {
  applyCollapseWindowFrame,
  clearResizePersistState,
  ensureWindowInsideMonitor,
  handleResizeHandleMouseEnter,
  handleResizeHandleMouseLeave,
  handleWindowMoved,
  isManualDraggingBox,
  isResizeHandleHovered,
  isResizingBox,
  resizeHandles,
  scheduleResizePersist,
  startDragging,
  startResizing,
  stopManualDragging,
  syncNativeWindowResizable,
  syncWindowBoundsFromStore,
} = useBoxWindowFrame({
  box,
  closeContextMenu: () => closeActiveContextMenu(),
  currentWindow,
  getBoxes: () => desktopStore.boxes,
  getResizeGridSettings: () => desktopStore.settings,
  getSnapThreshold: () => desktopStore.settings.snapThreshold,
  getSnapToEdges: () => desktopStore.settings.snapToEdges,
  isBoxCollapsedToTitle: () => readBoxCollapsedToTitle(),
  isCollapseWindowSizeApplying: () => readCollapseWindowSizeApplying(),
  isEditingTitle: () => readEditingTitle(),
  openCollapsedPreviewForActiveInteraction: () => openCollapsedPreviewForActiveInteractionHandler(),
  refreshCollapsedPreviewCloseSchedule: () => refreshCollapsedPreviewCloseScheduleHandler(),
  setLastError: (message) => {
    desktopStore.lastError = message;
  },
  updateBox: (nextBox, options) => desktopStore.updateBox(nextBox, options),
});

/**
 * 收缩预览只控制当前 WebView 内部文件网格，避免多渲染树覆盖导致窗口闪烁。
 */
const {
  animateBoxIdleOpacity,
  applyCollapseWindowSize,
  boxBodyStyle,
  boxGridOverflowClass,
  boxIdleOpacity,
  boxSurfaceStyle,
  boxTitleAreaStyle,
  clearCollapseWindowAnimation,
  handleBoxMouseEnter,
  handleBoxMouseLeave,
  handleBoxTitleMouseEnter,
  handleBoxTitleMouseLeave,
  isApplyingCollapseWindowSize,
  isBoxCollapsedToTitle,
  openCollapsedPreviewForActiveInteraction,
  refreshCollapsedPreviewCloseSchedule,
  setDragHoveringBox,
} = useBoxCollapsePreview({
  applyWindowFrame: (frame) => applyCollapseWindowFrame(frame),
  box,
  boxSurfaceRef,
  getBoxBackgroundOpacity: () => desktopStore.settings.boxBackgroundOpacity,
  getBoxCollapseAnimationMs: () => desktopStore.getBoxCollapseAnimationMs(),
  getBoxCornerRadius: () => desktopStore.settings.boxCornerRadius,
  isContextMenuOpen: () => readContextMenuOpen(),
  isEditingTitle: () => readEditingTitle(),
  isManualDraggingBox: () => isManualDraggingBox.value,
  isResizeHandleHovered: () => isResizeHandleHovered.value,
  isResizingBox: () => isResizingBox.value,
  resolveCurrentWindowHeight: async () => {
    const [windowSize, scaleFactor] = await Promise.all([
      currentWindow.outerSize(),
      currentWindow.scaleFactor(),
    ]);

    return windowSize.height / scaleFactor;
  },
  resolvePointerLocalPoint: async () => ({ inside: false }),
  resizePersistSettleMs: BOX_WINDOW_INTERACTION_TIMING.resizePersistSettleMs,
});

readBoxCollapsedToTitle = () => isBoxCollapsedToTitle.value;
readCollapseWindowSizeApplying = () => isApplyingCollapseWindowSize();
openCollapsedPreviewForActiveInteractionHandler = openCollapsedPreviewForActiveInteraction;
refreshCollapsedPreviewCloseScheduleHandler = refreshCollapsedPreviewCloseSchedule;

const canResizeBox = computed(
  () =>
    Boolean(box.value) &&
    !isBoxFileDragActive.value &&
    !isBoxCollapsedToTitle.value &&
    !box.value?.locked,
);
const boxItemWidth = computed(() =>
  Math.max(
    desktopStore.settings.boxFilenameWidth,
    desktopStore.settings.boxIconSize + DESKTOP_ICON_VIEW.itemInlinePadding * 2,
  ),
);
const boxTitleOrderClass = computed(() =>
  box.value?.titlePosition === "bottom" ? "order-2" : "order-0",
);
const boxGridStyle = computed(
  () =>
    ({
      columnGap: `${desktopStore.settings.boxIconGapX}px`,
      gridTemplateColumns: `repeat(auto-fill, minmax(min(100%, ${boxItemWidth.value}px), ${boxItemWidth.value}px))`,
      rowGap: `${desktopStore.settings.boxIconGapY}px`,
    }) as CSSProperties,
);
const selectionRectStyle = computed<CSSProperties>(() => {
  if (!selectionStart.value || !selectionCurrent.value) {
    return { display: "none" };
  }

  const left = Math.min(selectionStart.value.x, selectionCurrent.value.x);
  const top = Math.min(selectionStart.value.y, selectionCurrent.value.y);
  const width = Math.abs(selectionCurrent.value.x - selectionStart.value.x);
  const height = Math.abs(selectionCurrent.value.y - selectionStart.value.y);

  return {
    display: width > 0 && height > 0 ? "block" : "none",
    height: `${height}px`,
    left: `${left}px`,
    top: `${top}px`,
    width: `${width}px`,
  };
});

const {
  closeContextMenu,
  handleBoxContextMenuState,
  isContextMenuOpen,
  toggleContextMenu,
} = useBoxContextMenu({
  boxId: () => props.boxId,
  menuToggleCloseGuardMs: BOX_WINDOW_INTERACTION_TIMING.menuToggleCloseGuardMs,
  openCollapsedPreviewForActiveInteraction: () => openCollapsedPreviewForActiveInteraction(),
  refreshCollapsedPreviewCloseSchedule: () => refreshCollapsedPreviewCloseSchedule(),
  setLastError: (message) => {
    desktopStore.lastError = message;
  },
});
readContextMenuOpen = () => isContextMenuOpen.value;
closeActiveContextMenu = closeContextMenu;

/**
 * 标题编辑只修改展示名，真实文件夹路径保持稳定。
 */
const {
  cancelTitleEditing,
  commitTitleEditing,
  isEditingTitle,
  setTitleInputRef,
  startTitleEditing,
  titleDraft,
} = useBoxTitleEditing(
  box,
  (nextBox) => desktopStore.updateBox(nextBox),
  () => closeContextMenu(),
);
readEditingTitle = () => isEditingTitle.value;

watch(isBoxCollapsedToTitle, () => {
  void applyCollapseWindowSize(true);
});

watch(
  () => box.value?.titlePosition,
  () => {
    void applyCollapseWindowSize(true);
  },
);

watch(
  () => box.value?.folderPath,
  () => {
    void refreshBoxFolderItems();
  },
);

watch(boxIdleOpacity, () => {
  animateBoxIdleOpacity(true);
});

watch(
  canResizeBox,
  () => {
    syncNativeWindowResizable();
  },
  { immediate: true },
);

onMounted(async () => {
  if (!(await initializeDesktopStoreForBoxWindow())) {
    await desktopStore.initialize();
  }
  await syncWindowBoundsFromStore();
  await applyCollapseWindowSize(false);
  await nextTick();
  animateBoxIdleOpacity(false);
  syncNativeWindowResizable();
  await refreshBoxFolderItems();
  startFolderRefreshPolling();
  void preloadBoxContextMenuWindow().catch((error) => {
    desktopStore.lastError = error instanceof Error ? error.message : String(error);
  });

  unlistenFns.push(
    await currentWindow.onMoved(async ({ payload }) => {
      await handleWindowMoved(payload.x, payload.y);
    }),
  );
  unlistenFns.push(
    await listenBoxContextMenuState(({ payload }) => {
      handleBoxContextMenuState(payload.boxId, payload.isOpen, payload.reason);
    }),
  );
  unlistenFns.push(await currentWindow.onResized(() => scheduleResizePersist()));
  unlistenFns.push(
    await currentWindow.onScaleChanged(async () => {
      await ensureWindowInsideMonitor();
    }),
  );
  unlistenFns.push(
    await currentWindow.onDragDropEvent(async ({ payload }) => {
      await handleNativeDragDropEvent(payload);
    }),
  );
  unlistenFns.push(
    await listenBoxFileDrag(async ({ payload }) => {
      await handleBoxFileDragPayload(payload);
    }),
  );
  unlistenFns.push(
    await listenBoxFileDragAccepted(({ payload }) => {
      markFileDragAccepted(payload.sessionId);
    }),
  );
  await registerBoxNativeDropTarget(currentWindow.label).catch((error) => {
    desktopStore.lastError = error instanceof Error ? error.message : String(error);
  });
  window.addEventListener("keydown", handleGlobalFileViewKeydown);

  void notifyBoxWindowReady(props.boxId).catch((error) => {
    desktopStore.lastError = error instanceof Error ? error.message : String(error);
  });
});

async function initializeDesktopStoreForBoxWindow(): Promise<boolean> {
  const startupSnapshotToken = searchParams.get("startupSnapshot");
  if (!startupSnapshotToken) {
    return false;
  }

  return desktopStore.initializeFromStartupSnapshot(startupSnapshotToken);
}

onUnmounted(() => {
  stopFolderRefreshPolling();
  stopSelectionRectangle();
  cancelFileDragSession();
  clearAcceptedFileDragWaiters();
  window.removeEventListener("keydown", handleGlobalFileViewKeydown);
  void unregisterBoxNativeDropTarget(currentWindow.label).catch(() => undefined);
  stopManualDragging(false);
  closeContextMenu();
  clearCollapseWindowAnimation();
  clearResizePersistState();
  for (const unlisten of unlistenFns) {
    unlisten();
  }
});

/**
 * 轮询是当前 WebView 文件视图的兜底刷新机制，保证用户在外部 Explorer 手动改文件后 Box 会自动追上。
 */
function startFolderRefreshPolling(): void {
  stopFolderRefreshPolling();
  refreshTimer = window.setInterval(() => {
    void refreshBoxFolderItems({ silent: true });
  }, BOX_FILE_VIEW.refreshIntervalMs);
}

function stopFolderRefreshPolling(): void {
  if (!refreshTimer) {
    return;
  }

  window.clearInterval(refreshTimer);
  refreshTimer = null;
}

async function refreshBoxFolderItems(options: { silent?: boolean } = {}): Promise<void> {
  if (!box.value?.folderPath) {
    boxItems.value = [];
    return;
  }

  try {
    const items = await listBoxFolderItems(box.value.folderPath);
    boxItems.value = items;
    pruneSelection(items);
  } catch (error) {
    if (!options.silent) {
      desktopStore.lastError = error instanceof Error ? error.message : String(error);
    }
  }
}

/**
 * 刷新后只保留仍存在的选中项，避免外部删除文件后键盘操作命中旧路径。
 */
function pruneSelection(items: DesktopItem[]): void {
  const existingPaths = new Set(items.map((item) => item.path));
  selectedPaths.value = new Set(
    [...selectedPaths.value].filter((path) => existingPaths.has(path)),
  );
}

/**
 * Tauri 文件拖放事件只提供真实文件路径；具体复制、移动和映射策略统一交给 Rust 执行。
 */
async function handleNativeDragDropEvent(payload: {
  paths?: string[];
  type: "enter" | "over" | "drop" | "leave";
}): Promise<void> {
  if (payload.type === "enter" || payload.type === "over") {
    setDragHoveringBox(true);
    return;
  }

  if (payload.type === "leave") {
    setDragHoveringBox(false);
    refreshCollapsedPreviewCloseSchedule();
    return;
  }

  setDragHoveringBox(false);
  refreshCollapsedPreviewCloseSchedule();
  if (!box.value || !payload.paths?.length) {
    return;
  }

  try {
    await focusCurrentBoxForShellDialog();
    await handleBoxDroppedPaths(
      box.value.folderPath,
      payload.paths,
      desktopStore.settings.boxDropAction,
      desktopStore.settings.boxConflictPolicy,
    );
    await refreshBoxFolderItems();
  } catch (error) {
    desktopStore.lastError = error instanceof Error ? error.message : String(error);
  }
}

/**
 * 选中态以真实路径为主键，刷新后同名文件也不会互相串选中状态。
 */
function isItemSelected(item: DesktopItem): boolean {
  return selectedPaths.value.has(item.path);
}

/**
 * 拖拽中所有来源文件都降透明，框选多文件时用户能确认整组选区都进入移动状态。
 */
function isItemDragging(item: DesktopItem): boolean {
  return draggingFilePaths.value.has(item.path);
}

/**
 * 单击负责选中，Ctrl/Meta 单击负责增删选区，行为尽量贴近 Explorer 的多选直觉。
 */
function handleItemClick(event: MouseEvent, item: DesktopItem): void {
  if (suppressNextItemClick.value) {
    suppressNextItemClick.value = false;
    return;
  }

  boxGridRef.value?.focus();
  closeContextMenu();
  if (event.ctrlKey || event.metaKey) {
    const nextSelection = new Set(selectedPaths.value);
    if (nextSelection.has(item.path)) {
      nextSelection.delete(item.path);
    } else {
      nextSelection.add(item.path);
    }
    selectedPaths.value = nextSelection;
    return;
  }

  selectedPaths.value = new Set([item.path]);
  if (!desktopStore.settings.doubleClickOpenItems && event.detail === DESKTOP_ICON_VIEW.openClickDetail) {
    void openItem(item);
  }
}

/**
 * 右键先修正选区但不打开旧 Box 菜单，为后续文件级菜单保留准确上下文。
 */
function handleItemContextMenu(event: MouseEvent, item: DesktopItem): void {
  boxGridRef.value?.focus();
  closeContextMenu();
  if (!selectedPaths.value.has(item.path)) {
    selectedPaths.value = new Set([item.path]);
  }
  event.preventDefault();
  void showNativeItemContextMenu(item.path, event.screenX, event.screenY)
    .then(() => refreshBoxFolderItems({ silent: true }))
    .catch((error) => {
      desktopStore.lastError = error instanceof Error ? error.message : String(error);
    });
}

/**
 * Box 内文件拖动使用 pointer 候选态，超过阈值后才广播跨窗口拖拽，避免普通点击误触移动文件。
 */
function handleItemPointerDown(event: PointerEvent, item: DesktopItem): void {
  if (event.button !== 0 || editingPath.value) {
    return;
  }

  const startX = event.screenX;
  const startY = event.screenY;
  let hasStarted = false;
  const startDrag = async (): Promise<void> => {
    if (hasStarted || !box.value) {
      return;
    }

    hasStarted = true;
    suppressNextItemClick.value = true;
    const dragItems = resolveDragItems(item);
    const dragPaths = dragItems.map((dragItem) => dragItem.path);

    draggingItemPath.value = item.path;
    draggingFilePaths.value = new Set(dragPaths);
    selectedPaths.value = new Set(dragPaths);
    fileDragState = {
      item,
      paths: dragPaths,
      preview: resolveFileDragPreviewOptions(),
      sessionId: crypto.randomUUID(),
      sourceBoxId: box.value.id,
    };
    clearFileDragPolling();
    fileDragPollTimer = window.setInterval(() => {
      void pollFileDragCursor();
    }, BOX_ITEM_DRAG_INTERACTION.pollIntervalMs);
    await openDragPreviewWindow().catch((error) => {
      desktopStore.lastError = error instanceof Error ? error.message : String(error);
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
  const isDroppedInsideSourceBox = await resolveScreenPointInCurrentWindow(screenX, screenY)
    .then((point) => point.inside)
    .catch(() => false);
  await emitFileDragPhase("drop", screenX, screenY);
  clearFileDragPolling();
  draggingItemPath.value = null;
  draggingFilePaths.value = new Set();
  fileDragState = null;
  if (!isDroppedInsideSourceBox) {
    const isAcceptedByBox = await waitForFileDragAccepted(dragState.sessionId);
    if (!isAcceptedByBox) {
      await handleFileDragOutToDesktop(dragState);
    }
  }
  void refreshBoxFolderItems({ silent: true });
  window.setTimeout(() => {
    void refreshBoxFolderItems({ silent: true });
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
  if (!box.value) {
    return;
  }
  if (payload.phase === "cancel") {
    setDragHoveringBox(false);
    refreshCollapsedPreviewCloseSchedule();
    return;
  }

  const localPoint = await resolveScreenPointInCurrentWindow(payload.screenX, payload.screenY);
  if (!localPoint.inside) {
    setDragHoveringBox(false);
    if (payload.phase === "drop") {
      refreshCollapsedPreviewCloseSchedule();
    }
    return;
  }

  if (payload.phase !== "drop") {
    setDragHoveringBox(true);
    return;
  }

  setDragHoveringBox(false);
  refreshCollapsedPreviewCloseSchedule();
  if (payload.sourceBoxId === box.value.id) {
    return;
  }

  try {
    await notifyBoxFileDragAccepted({
      sessionId: payload.sessionId,
      targetBoxId: box.value.id,
    });
    await focusCurrentBoxForShellDialog();
    await handleBoxDroppedPaths(
      box.value.folderPath,
      payload.paths,
      "move",
      desktopStore.settings.boxConflictPolicy,
    );
    await refreshBoxFolderItems();
  } catch (error) {
    desktopStore.lastError = error instanceof Error ? error.message : String(error);
  }
}

/**
 * 释放点不属于任何 Box 时，把当前拖拽文件按设置处理到 Windows 桌面目录。
 */
async function handleFileDragOutToDesktop(
  dragState: {
    paths: string[];
    sessionId: string;
  }
): Promise<void> {
  const desktopPath = await resolveDesktopPathForDragOut();
  if (!desktopPath) {
    desktopStore.lastError = "无法定位桌面路径，已停止拖出文件";
    return;
  }

  try {
    await focusCurrentBoxForShellDialog();
    await handleBoxDraggedPathsToDesktop(
      desktopPath,
      dragState.paths,
      desktopStore.settings.boxDragOutAction,
      desktopStore.settings.boxConflictPolicy,
    );
  } catch (error) {
    desktopStore.lastError = error instanceof Error ? error.message : String(error);
  }
}

/**
 * Box 启动快照通常已包含桌面路径；为空时主动刷新一次，避免拖出功能卡在旧启动状态。
 */
async function resolveDesktopPathForDragOut(): Promise<string> {
  if (desktopStore.desktopPath.trim()) {
    return desktopStore.desktopPath;
  }

  await desktopStore.refreshSnapshot(false);
  return desktopStore.desktopPath.trim();
}

/**
 * 真实文件操作可能弹出 Windows 冲突处理框，先聚焦当前 Box 可让 Shell 对话框更稳定地出现在前台。
 */
async function focusCurrentBoxForShellDialog(): Promise<void> {
  await currentWindow.setFocus().catch(() => undefined);
}

/**
 * 将屏幕物理坐标换算到当前无边框窗口，跨 DPI 显示器拖动时命中判断仍保持稳定。
 */
async function resolveScreenPointInCurrentWindow(
  screenX: number,
  screenY: number,
): Promise<{ inside: boolean; x: number; y: number }> {
  const [position, size, scaleFactor] = await Promise.all([
    currentWindow.outerPosition(),
    currentWindow.outerSize(),
    currentWindow.scaleFactor(),
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
 * 打开文件项统一走系统默认方式，单击/双击/Enter 只决定触发时机，不改变打开语义。
 */
async function openItem(item: DesktopItem): Promise<void> {
  try {
    await openDesktopItem(item.path);
  } catch (error) {
    desktopStore.lastError = error instanceof Error ? error.message : String(error);
  }
}

/**
 * 双击文件项走系统默认打开方式，前端不判断具体扩展名或快捷方式目标。
 */
async function handleItemDoubleClick(item: DesktopItem): Promise<void> {
  if (!desktopStore.settings.doubleClickOpenItems) {
    return;
  }

  await openItem(item);
}

/**
 * 空白区域按下鼠标才启动框选，避免和图标点击、窗口拖动、缩放热区互相抢事件。
 */
function handleGridPointerDown(event: PointerEvent): void {
  if (event.button !== 0 || !boxGridRef.value) {
    return;
  }
  const target = event.target;
  if (target instanceof HTMLElement && target.closest("[data-box-item-path]")) {
    return;
  }

  boxGridRef.value.focus();
  const point = resolveGridLocalPoint(event);
  selectionStart.value = point;
  selectionCurrent.value = point;
  if (!event.ctrlKey && !event.metaKey) {
    selectedPaths.value = new Set();
  }
  window.addEventListener("pointermove", handleSelectionPointerMove, { capture: true });
  window.addEventListener("pointerup", handleSelectionPointerUp, { capture: true, once: true });
  window.addEventListener("pointercancel", handleSelectionPointerUp, { capture: true, once: true });
}

/**
 * 框选移动阶段实时换算网格内坐标，滚动区域内也能得到稳定选择矩形。
 */
function handleSelectionPointerMove(event: PointerEvent): void {
  selectionCurrent.value = resolveGridLocalPoint(event);
  applySelectionRectangle();
}

/**
 * 鼠标释放时做最后一次命中计算，再清理全局 pointer 监听。
 */
function handleSelectionPointerUp(): void {
  applySelectionRectangle();
  stopSelectionRectangle();
}

/**
 * 框选监听挂在 window 上，清理必须成对执行，防止指针离开 Box 后残留选择状态。
 */
function stopSelectionRectangle(): void {
  window.removeEventListener("pointermove", handleSelectionPointerMove, { capture: true });
  window.removeEventListener("pointerup", handleSelectionPointerUp, { capture: true });
  window.removeEventListener("pointercancel", handleSelectionPointerUp, { capture: true });
  selectionStart.value = null;
  selectionCurrent.value = null;
}

/**
 * 将视口坐标换算为滚动容器内坐标，保证滚动后框选命中仍与视觉位置一致。
 */
function resolveGridLocalPoint(event: PointerEvent): { x: number; y: number } {
  const gridRect = boxGridRef.value?.getBoundingClientRect();
  if (!gridRect) {
    return { x: 0, y: 0 };
  }

  return {
    x: event.clientX - gridRect.left + (boxGridRef.value?.scrollLeft ?? 0),
    y: event.clientY - gridRect.top + (boxGridRef.value?.scrollTop ?? 0),
  };
}

/**
 * 使用 DOM 矩形做交集判断，避免根据网格列数推断位置时受字体和缩放影响。
 */
function applySelectionRectangle(): void {
  if (!boxGridRef.value || !selectionStart.value || !selectionCurrent.value) {
    return;
  }

  const selectionRect = normalizeRect(selectionStart.value, selectionCurrent.value);
  const gridRect = boxGridRef.value.getBoundingClientRect();
  const nextSelection = new Set(selectedPaths.value);
  for (const item of boxItems.value) {
    const element = boxGridRef.value.querySelector<HTMLElement>(
      `[data-box-item-path="${CSS.escape(item.path)}"]`,
    );
    if (!element) {
      continue;
    }

    const elementRect = element.getBoundingClientRect();
    const itemRect = {
      height: elementRect.height,
      left: elementRect.left - gridRect.left + boxGridRef.value.scrollLeft,
      top: elementRect.top - gridRect.top + boxGridRef.value.scrollTop,
      width: elementRect.width,
    };
    if (rectsIntersect(selectionRect, itemRect)) {
      nextSelection.add(item.path);
    }
  }
  selectedPaths.value = nextSelection;
}

/**
 * 统一归一化拖拽矩形，调用方不需要关心用户从哪个方向框选。
 */
function normalizeRect(
  start: { x: number; y: number },
  end: { x: number; y: number },
): { height: number; left: number; top: number; width: number } {
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
function rectsIntersect(
  leftRect: { height: number; left: number; top: number; width: number },
  rightRect: { height: number; left: number; top: number; width: number },
): boolean {
  return (
    leftRect.left < rightRect.left + rightRect.width &&
    leftRect.left + leftRect.width > rightRect.left &&
    leftRect.top < rightRect.top + rightRect.height &&
    leftRect.top + leftRect.height > rightRect.top
  );
}

/**
 * 文件区键盘快捷键覆盖高频整理操作，标题编辑中会跳过避免误删或误改文件。
 */
async function handleFileViewKeydown(event: KeyboardEvent): Promise<void> {
  if (isEditingTitle.value) {
    return;
  }

  const selectedItems = resolveSelectedItems();
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "a") {
    selectedPaths.value = new Set(boxItems.value.map((item) => item.path));
    event.preventDefault();
    return;
  }
  if (event.key === "F2") {
    startSelectedItemRename();
    event.preventDefault();
    return;
  }
  if (event.key === "Delete") {
    event.preventDefault();
    await deleteSelectedItems();
    return;
  }
  if (event.key === "Enter" && selectedItems[0]) {
    event.preventDefault();
    await openItem(selectedItems[0]);
    return;
  }
  if (event.key === "Escape") {
    editingPath.value = null;
    selectedPaths.value = new Set();
    event.preventDefault();
  }
}

/**
 * F2/Delete/Enter 不依赖文件网格是否正好拿到焦点，避免点击图标后快捷键被窗口吞掉。
 */
function handleGlobalFileViewKeydown(event: KeyboardEvent): void {
  if (event.defaultPrevented || shouldIgnoreGlobalFileShortcut()) {
    return;
  }

  void handleFileViewKeydown(event);
}

/**
 * 输入框和可编辑区域保留系统键盘行为，避免文件重命名或标题编辑时触发全局快捷键。
 */
function shouldIgnoreGlobalFileShortcut(): boolean {
  const activeElement = document.activeElement;
  if (!activeElement) {
    return false;
  }

  if (activeElement instanceof HTMLInputElement || activeElement instanceof HTMLTextAreaElement) {
    return true;
  }

  return activeElement instanceof HTMLElement && activeElement.isContentEditable;
}

/**
 * 选中项每次从最新扫描结果解析，避免文件刷新后继续操作已不存在的路径。
 */
function resolveSelectedItems(): DesktopItem[] {
  return boxItems.value.filter((item) => selectedPaths.value.has(item.path));
}

/**
 * 拖拽启动时若指针按在已选项上，就拖动当前全部选区；否则先把选区收敛到当前文件。
 */
function resolveDragItems(pointerItem: DesktopItem): DesktopItem[] {
  if (!selectedPaths.value.has(pointerItem.path)) {
    return [pointerItem];
  }

  const selectedItems = resolveSelectedItems();
  if (selectedItems.length === 0) {
    return [pointerItem];
  }

  return [
    pointerItem,
    ...selectedItems.filter((selectedItem) => selectedItem.path !== pointerItem.path),
  ];
}

/**
 * 拖影视觉参数来自当前 Box 设置，保证拖动中图标尺寸、文件名和快捷方式角标保持一致。
 */
function resolveFileDragPreviewOptions(): BoxFileDragPreviewOptions {
  return {
    iconSize: desktopStore.settings.boxIconSize,
    labelTextSize: desktopStore.settings.boxLabelTextSize,
    labelWidth: desktopStore.settings.boxFilenameWidth,
    nameDisplayMode: desktopStore.settings.nameDisplayMode,
    radiusSize: desktopStore.settings.boxCornerRadius,
    showItemLabels: desktopStore.settings.showItemLabels,
    showShortcutArrow: desktopStore.settings.showShortcutArrow,
  };
}

/**
 * F2 只编辑第一个选中项，符合 Explorer 多选时的基础重命名入口。
 */
function startSelectedItemRename(): void {
  const item = resolveSelectedItems()[0];
  if (!item) {
    return;
  }

  editingPath.value = item.path;
  renameDraft.value = item.name;
  void nextTick(() => {
    const input = boxGridRef.value?.querySelector<HTMLInputElement>("[data-rename-input='true']");
    input?.focus();
    input?.select();
  });
}

/**
 * 重命名只提交用户确认后的完整文件名，跨目录移动和冲突检查交给 Rust 层拦截。
 */
async function commitRename(): Promise<void> {
  const targetPath = editingPath.value;
  const nextName = renameDraft.value.trim();
  if (!targetPath || !nextName) {
    editingPath.value = null;
    return;
  }

  try {
    await renameDesktopItem(targetPath, nextName);
    editingPath.value = null;
    await refreshBoxFolderItems();
  } catch (error) {
    desktopStore.lastError = error instanceof Error ? error.message : String(error);
  }
}

/**
 * 取消重命名只恢复前端编辑态，真实文件名保持不变。
 */
function cancelRename(): void {
  editingPath.value = null;
  renameDraft.value = "";
}

/**
 * Delete 将选中文件送入回收站，成功后刷新文件夹并清空选区。
 */
async function deleteSelectedItems(): Promise<void> {
  const paths = resolveSelectedItems().map((item) => item.path);
  if (paths.length === 0) {
    return;
  }

  try {
    await deleteDesktopItems(paths);
    selectedPaths.value = new Set();
    await refreshBoxFolderItems();
  } catch (error) {
    desktopStore.lastError = error instanceof Error ? error.message : String(error);
  }
}
</script>

<template>
  <main
    class="h-screen w-screen overflow-hidden bg-transparent p-0"
    :class="box?.locked ? 'cursor-default' : ''"
    @click="closeContextMenu"
  >
    <article
      v-if="box"
      ref="boxSurfaceRef"
      class="dasktop-box-surface relative flex h-full w-full flex-col overflow-hidden text-slate-950 dark:text-white"
      :style="boxSurfaceStyle"
      @mouseenter="handleBoxMouseEnter"
      @mouseleave="handleBoxMouseLeave"
    >
      <template v-if="canResizeBox">
        <span
          v-for="handle in resizeHandles"
          :key="handle.direction"
          class="absolute z-40"
          :class="handle.className"
          @mousedown.stop.prevent="startResizing(handle.direction, $event)"
          @mouseenter="handleResizeHandleMouseEnter"
          @mouseleave="handleResizeHandleMouseLeave"
        />
      </template>

      <header
        class="relative z-30 flex h-10 shrink-0 select-none items-center justify-center px-3 transition-opacity duration-150 ease-out"
        :class="boxTitleOrderClass"
        :style="boxTitleAreaStyle"
        @mousedown.left="startDragging"
        @dblclick.stop.prevent
        @mouseenter="handleBoxTitleMouseEnter"
        @mouseleave="handleBoxTitleMouseLeave"
      >
        <input
          v-if="isEditingTitle"
          :ref="setTitleInputRef"
          v-model="titleDraft"
          aria-label="编辑 Box 名称"
          class="h-7 w-[68%] max-w-[220px] rounded-[6px] bg-white/60 px-2 text-center text-[13px] font-semibold text-slate-900 outline-none transition-colors placeholder:text-slate-400 focus:bg-white/85 dark:bg-white/10 dark:text-white dark:focus:bg-white/15"
          maxlength="32"
          type="text"
          @blur="commitTitleEditing"
          @keydown.enter.prevent="commitTitleEditing"
          @keydown.esc.prevent="cancelTitleEditing"
          @mousedown.stop
          @dblclick.stop
        />
        <span
          v-else
          class="h-7 min-w-12 max-w-[68%] truncate text-center text-[13px] font-semibold leading-7 text-slate-900 dark:text-white"
          title="双击编辑 Box 名称"
          @dblclick="startTitleEditing"
          @mousedown.stop
        >
          {{ box.title }}
        </span>
        <button
          aria-label="打开 Box 菜单"
          class="absolute right-2 top-1/2 grid size-7 -translate-y-1/2 place-items-center rounded-[6px] text-slate-600 transition-colors hover:bg-white/50 hover:text-slate-950 dark:text-slate-300 dark:hover:bg-white/10 dark:hover:text-white"
          type="button"
          @click.stop="toggleContextMenu"
          @mousedown.stop
        >
          <MoreHorizontal :size="18" />
        </button>
      </header>

      <div
        ref="boxGridRef"
        class="dasktop-scrollarea dasktop-box-scrollarea relative grid min-h-0 flex-1 p-2.5 transition-[opacity,transform] duration-180 ease-out outline-none"
        :class="[
          boxGridOverflowClass,
          boxItems.length === 0
            ? 'content-center place-items-center justify-center'
            : 'content-start items-start justify-start',
        ]"
        :style="[boxGridStyle, boxBodyStyle]"
        tabindex="0"
        @keydown="handleFileViewKeydown"
        @pointerdown="handleGridPointerDown"
      >
        <DesktopIcon
          v-for="item in boxItems"
          :key="item.path"
          :drag-interaction-disabled="isBoxFileDragActive"
          :dragging="isItemDragging(item)"
          :editing="editingPath === item.path"
          :icon-size="desktopStore.settings.boxIconSize"
          :item="item"
          :label-text-size="desktopStore.settings.boxLabelTextSize"
          :label-width="desktopStore.settings.boxFilenameWidth"
          :name-display-mode="desktopStore.settings.nameDisplayMode"
          :radius-size="desktopStore.settings.boxCornerRadius"
          :rename-draft="renameDraft"
          :selected="isItemSelected(item)"
          :show-label="desktopStore.settings.showItemLabels"
          :show-shortcut-arrow="desktopStore.settings.showShortcutArrow"
          @commit-rename="commitRename"
          @cancel-rename="cancelRename"
          @item-click="handleItemClick"
          @item-context-menu="handleItemContextMenu"
          @item-double-click="handleItemDoubleClick"
          @item-pointer-down="handleItemPointerDown"
          @rename-draft-change="renameDraft = $event"
        />

        <div
          v-if="boxItems.length === 0"
          class="col-span-full flex min-h-[120px] w-full flex-col items-center justify-center px-5 text-center"
        >
          <strong class="block text-center text-[13px] font-semibold text-slate-900 dark:text-white">这个 Box 还是空的</strong>
          <p class="mt-1 max-w-[180px] text-center text-[12px] leading-5 text-slate-600 dark:text-slate-300">把桌面文件拖进来就能开始整理</p>
        </div>

        <div
          v-if="isSelecting"
          class="pointer-events-none absolute z-20 border border-[#2f6bff]/70 bg-[#2f6bff]/15"
          :style="selectionRectStyle"
        />
      </div>
    </article>
  </main>
</template>
