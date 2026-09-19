<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import type { ComponentPublicInstance, CSSProperties } from "vue";
import { cursorPosition, Effect, getCurrentWindow } from "@tauri-apps/api/window";
import { emit, listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { Folder } from "@lucide/vue";
import BoxFileGrid from "@/windows/desktop/components/BoxFileGrid.vue";
import BoxHeader from "@/windows/desktop/components/BoxHeader.vue";
import BoxResizeHandles from "@/windows/desktop/components/BoxResizeHandles.vue";
import { DESKTOP_ICON_VIEW } from "@/windows/desktop/config/desktopIcon";
import { useBoxCollapsePreview } from "@/windows/desktop/composables/useBoxCollapsePreview";
import { useBoxContextMenu } from "@/windows/desktop/composables/useBoxContextMenu";
import { useBoxFileActions } from "@/windows/desktop/composables/useBoxFileActions";
import { useBoxFileDrag } from "@/windows/desktop/composables/useBoxFileDrag";
import { useBoxFileItems } from "@/windows/desktop/composables/useBoxFileItems";
import { useBoxFileSelection } from "@/windows/desktop/composables/useBoxFileSelection";
import { useBoxTitleEditing } from "@/windows/desktop/composables/useBoxTitleEditing";
import { useBoxWindowFrame } from "@/windows/desktop/composables/useBoxWindowFrame";
import { useBoxWindowLifecycle } from "@/windows/desktop/composables/useBoxWindowLifecycle";
import { resolveBoxResizeGridRowHeight } from "@/windows/desktop/utils/boxResizeGrid";
import { useDesktopStore } from "@/entities/desktopBox/store";
import { BOX_ICON_STATE_SIZE, BOX_WINDOW_INTERACTION_TIMING } from "@/entities/desktopBox/layout";
import { BOX_HOVER_HANDOFF_EVENT, type BoxHoverHandoffPayload } from "@/shared/ipc/desktop";
import { BUILTIN_COVER_ICONS, parseBuiltinCoverId } from "@/windows/boxContextMenu/model/builtinCovers";
import type { DesktopItem } from "@/entities/desktopItem/types";
import type {
  BoxScreenPoint,
  BoxSortInsertionPreview,
} from "@/windows/desktop/model/fileDrag";

const props = defineProps<{
  boxId: string;
}>();

const desktopStore = useDesktopStore();
const currentWindow = getCurrentWindow();
const boxSurfaceRef = ref<HTMLElement | null>(null);
const boxGridRef = ref<HTMLElement | null>(null);
const box = computed(() => desktopStore.boxes.find((item) => item.id === props.boxId));

let readContextMenuOpen = (): boolean => false;
let readEditingTitle = (): boolean => false;
let readBoxCollapsedToTitle = (): boolean => false;
let readCollapseWindowSizeApplying = (): boolean => false;
let closeActiveContextMenu = (): void => undefined;
let openCollapsedPreviewForActiveInteractionHandler = (): void => undefined;
let refreshCollapsedPreviewCloseScheduleHandler = (): void => undefined;

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
  currentWindow: {
    clearWindowEffects: () => currentWindow.setEffects({ effects: [] }),
    outerPosition: () => currentWindow.outerPosition(),
    outerSize: () => currentWindow.outerSize(),
    scaleFactor: () => currentWindow.scaleFactor(),
    setEffects: (effects) =>
      currentWindow.setEffects({ effects: effects.effects as Effect[] }),
    setPosition: (position) => currentWindow.setPosition(position),
    setResizable: (resizable) => currentWindow.setResizable(resizable),
    setSize: (size) => currentWindow.setSize(size),
    startDragging: () => currentWindow.startDragging(),
  },
  getBoxes: () => desktopStore.boxes,
  getBoxAcrylicEnabled: () => desktopStore.settings.boxAcrylicEnabled,
  getResizeGridSettings: () => desktopStore.settings,
  getSnapThreshold: () => desktopStore.settings.snapThreshold,
  getSnapToEdges: () => desktopStore.settings.snapToEdges,
  isBoxCollapsedToTitle: () => readBoxCollapsedToTitle(),
  isCollapseWindowSizeApplying: () => readCollapseWindowSizeApplying(),
  isEditingTitle: () => readEditingTitle(),
  openCollapsedPreviewForActiveInteraction: () => openCollapsedPreviewForActiveInteractionHandler(),
  refreshCollapsedPreviewCloseSchedule: () => refreshCollapsedPreviewCloseScheduleHandler(),
  setLastError,
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
  isBoxInIconState,
  isCollapseAnimating,
  openCollapsedPreviewForActiveInteraction,
  refreshCollapsedPreviewCloseSchedule,
  setDragHoveringBox,
  setNativeItemContextMenuOpen,
} = useBoxCollapsePreview({
  applyWindowFrame: (frame) => applyCollapseWindowFrame(frame),
  box,
  boxSurfaceRef,
  getBoxBackgroundOpacity: () => desktopStore.settings.boxBackgroundOpacity,
  getBoxCollapseAnimationMs: () => desktopStore.getBoxCollapseAnimationMs(),
  getBoxCollapseDelayMs: () => desktopStore.getBoxCollapseDelayMs(),
  getBoxCollapseMode: () => box.value?.collapseMode ?? "icon",
  getBoxCornerRadius: () => desktopStore.settings.boxCornerRadius,
  getBoxExpandHoverDelayMs: () => desktopStore.settings.boxExpandHoverDelayMs,
  getBoxIconFadeInMs: () => desktopStore.settings.boxIconFadeInMs,
  getBoxIdleOpacityHideAnimationMs: () => desktopStore.getBoxIdleOpacityHideAnimationMs(),
  getBoxIdleOpacityShowAnimationMs: () => desktopStore.getBoxIdleOpacityShowAnimationMs(),
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
  resolveCurrentWindowWidth: async () => {
    const [windowSize, scaleFactor] = await Promise.all([
      currentWindow.outerSize(),
      currentWindow.scaleFactor(),
    ]);

    return windowSize.width / scaleFactor;
  },
  resolvePointerLocalPoint: async () => ({ inside: false }),
  resizePersistSettleMs: BOX_WINDOW_INTERACTION_TIMING.resizePersistSettleMs,
});

readBoxCollapsedToTitle = () => isBoxCollapsedToTitle.value;
readCollapseWindowSizeApplying = () => isApplyingCollapseWindowSize();
openCollapsedPreviewForActiveInteractionHandler = openCollapsedPreviewForActiveInteraction;
refreshCollapsedPreviewCloseScheduleHandler = refreshCollapsedPreviewCloseSchedule;

const boxItemWidth = computed(() =>
  Math.max(
    desktopStore.settings.boxFilenameWidth,
    desktopStore.settings.boxIconSize + DESKTOP_ICON_VIEW.itemInlinePadding * 2,
  ),
);
const boxTitleOrderClass = computed(() =>
  box.value?.titlePosition === "bottom" ? "order-2" : "order-0",
);
/**
 * 图标态入口按既定优先级取图：自定义图片封面 > 内置图标库 > Box 内第一个
 * 非快捷方式文件的 Shell 图标 > 默认文件夹图标；封面由更多菜单「设置封面」维护。
 * 快捷方式的 Shell 位图自带白底角标，不作为图标态封面源
 */
const firstNonShortcutItem = computed(
  () => boxItems.value.find((item) => item.kind !== "shortcut") ?? null,
);
const iconStateImageSrc = computed(() => {
  const cover = box.value?.coverIcon ?? null;
  if (cover?.startsWith("data:image/")) {
    return cover;
  }

  return firstNonShortcutItem.value?.iconDataUrl ?? null;
});
const iconStateBuiltinComponent = computed(() => {
  const builtinId = parseBuiltinCoverId(box.value?.coverIcon ?? null);
  if (!builtinId) {
    return null;
  }

  return BUILTIN_COVER_ICONS.find((cover) => cover.id === builtinId)?.component ?? null;
});

/**
 * 过界判定容差：鼠标落点落在相邻 Box 逻辑边界向外扩展该值的范围内即视为越界进入
 */
const HOVER_HANDOFF_TOLERANCE_PX = 24;
let unlistenHoverHandoff: (() => void) | null = null;
let hoverHandoffTimer: ReturnType<typeof window.setTimeout> | null = null;

/**
 * 鼠标离开当前 Box 后判定落点：落在吸附相邻且处于收缩形态的 Box 边界内时广播过界事件，
 * 让目标 Box 无需被精确命中小图标即可承接展开态
 */
async function broadcastHoverHandoffOnLeave(): Promise<void> {
  const currentBox = box.value;
  if (!currentBox) {
    return;
  }

  const [cursor, scaleFactor] = await Promise.all([
    cursorPosition(),
    currentWindow.scaleFactor(),
  ]);
  const logicalX = cursor.x / scaleFactor;
  const logicalY = cursor.y / scaleFactor;
  const tolerance = desktopStore.settings.snapThreshold;
  const target = desktopStore.boxes.find((candidate) => {
    if (candidate.id === currentBox.id || !candidate.collapsed) {
      return false;
    }

    // 判定视点用目标收缩后的图标方块范围，而不是其展开尺寸，
    // 避免相邻 Box 展开体覆盖到鼠标路径时造成误触发
    return (
      logicalX >= candidate.x - tolerance &&
      logicalX <= candidate.x + BOX_ICON_STATE_SIZE + tolerance &&
      logicalY >= candidate.y - tolerance &&
      logicalY <= candidate.y + BOX_ICON_STATE_SIZE + tolerance
    );
  });

  if (!target) {
    return;
  }

  await emit(BOX_HOVER_HANDOFF_EVENT, { fromBoxId: currentBox.id, toBoxId: target.id });
}

function handleBoxMouseLeaveWithHandoff(): void {
  handleBoxMouseLeave();
  void broadcastHoverHandoffOnLeave().catch(() => undefined);
}

/**
 * 接收相邻 Box 的过界切换：防误触延迟后确认鼠标确实落在自己边界内才临时展开，
 * 鼠标快速划过桌面时不会反复拉起沿途的收缩 Box
 */
async function acceptHoverHandoff(): Promise<void> {
  const currentBox = box.value;
  if (!currentBox?.collapsed) {
    return;
  }

  const [cursor, scaleFactor] = await Promise.all([
    cursorPosition(),
    currentWindow.scaleFactor(),
  ]);
  const logicalX = cursor.x / scaleFactor;
  const logicalY = cursor.y / scaleFactor;
  // 接受判定同样以自己的图标方块为视点：鼠标确实停在图标附近才算承接，
  // 鼠标仍停在来源 Box 的展开区域内时不切换
  const inside =
    logicalX >= currentBox.x - HOVER_HANDOFF_TOLERANCE_PX &&
    logicalX <= currentBox.x + BOX_ICON_STATE_SIZE + HOVER_HANDOFF_TOLERANCE_PX &&
    logicalY >= currentBox.y - HOVER_HANDOFF_TOLERANCE_PX &&
    logicalY <= currentBox.y + BOX_ICON_STATE_SIZE + HOVER_HANDOFF_TOLERANCE_PX;
  if (inside) {
    openCollapsedPreviewForActiveInteraction();
  }
}

onMounted(() => {
  // Box 是常驻桌面挂件：标记为工具窗口后不进 Alt+Tab，Win+D 显示桌面时保持原位
  void invoke("apply_desktop_toolbox").catch((error) => {
    setLastError(error instanceof Error ? error.message : String(error));
  });
  void listen<BoxHoverHandoffPayload>(BOX_HOVER_HANDOFF_EVENT, ({ payload }) => {
    if (payload.toBoxId !== props.boxId || !box.value?.collapsed) {
      return;
    }

    if (hoverHandoffTimer) {
      window.clearTimeout(hoverHandoffTimer);
    }

    hoverHandoffTimer = window.setTimeout(() => {
      hoverHandoffTimer = null;
      void acceptHoverHandoff().catch(() => undefined);
    }, BOX_WINDOW_INTERACTION_TIMING.hoverSwitchDelayMs);
  })
    .then((unlisten) => {
      unlistenHoverHandoff = unlisten;
    })
    .catch(() => undefined);
});

/**
 * 系统毛玻璃（DWM Acrylic）按设置应用或清除：开启后背景模糊由系统合成，
 * 前端 surface 只保留色调层；拖动期间的效果暂停由窗口框架逻辑单独管理
 */
async function applyWindowEffectsFromSettings(): Promise<void> {
  try {
    if (desktopStore.settings.boxAcrylicEnabled) {
      await currentWindow.setEffects({ effects: [Effect.Acrylic] });
    } else {
      await currentWindow.setEffects({ effects: [] });
    }
  } catch (error) {
    setLastError(error instanceof Error ? error.message : String(error));
  }
}

watch(
  () => desktopStore.settings.boxAcrylicEnabled,
  () => {
    void applyWindowEffectsFromSettings();
  },
  { immediate: true },
);
const boxGridStyle = computed(
  () =>
    ({
      columnGap: `${desktopStore.settings.boxIconGapX}px`,
      gridAutoRows: `${resolveBoxResizeGridRowHeight(desktopStore.settings)}px`,
      gridTemplateColumns: `repeat(auto-fill, minmax(min(100%, ${boxItemWidth.value}px), ${boxItemWidth.value}px))`,
      rowGap: `${desktopStore.settings.boxIconGapY}px`,
    }) as CSSProperties,
);
const boxSortInsertionPreview = ref<BoxSortInsertionPreview | null>(null);
let sortInsertionPreviewExpireTimer: ReturnType<typeof window.setTimeout> | null = null;
const SORT_INSERTION_PREVIEW_STALE_MS = 200;

/**
 * 排序插入线是拖拽 over 的瞬时命中结果；null 只表示不续期，真正消失统一交给心跳过期。
 */
function setBoxSortInsertionPreview(preview: BoxSortInsertionPreview | null): void {
  if (!preview) {
    return;
  }

  clearSortInsertionPreviewExpireTimer();
  boxSortInsertionPreview.value = preview;
  sortInsertionPreviewExpireTimer = window.setTimeout(() => {
    sortInsertionPreviewExpireTimer = null;
    boxSortInsertionPreview.value = null;
  }, SORT_INSERTION_PREVIEW_STALE_MS);
}

/**
 * 组件销毁时取消延迟任务，防止旧窗口的异步计时器写回新状态。
 */
function clearSortInsertionPreviewExpireTimer(): void {
  if (!sortInsertionPreviewExpireTimer) {
    return;
  }

  window.clearTimeout(sortInsertionPreviewExpireTimer);
  sortInsertionPreviewExpireTimer = null;
}

onUnmounted(() => {
  clearSortInsertionPreviewExpireTimer();
  unlistenHoverHandoff?.();
  unlistenHoverHandoff = null;
  if (hoverHandoffTimer) {
    window.clearTimeout(hoverHandoffTimer);
    hoverHandoffTimer = null;
  }
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
  setLastError,
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
  () => refreshCollapsedPreviewCloseSchedule(),
);
readEditingTitle = () => isEditingTitle.value;

const {
  boxItems,
  appendBoxShellItems,
  moveBoxItemsInOrder,
  removeBoxItemOrderPaths,
  removeBoxShellItems,
  replaceBoxItemOrderPath,
  refreshBoxFolderItems,
  startFolderRefreshPolling,
  stopFolderRefreshPolling,
} = useBoxFileItems({
  box,
  getAutoHideNativeShellIcons: () => desktopStore.settings.autoHideNativeShellIcons,
  setLastError,
});

let openItemHandler = async (_item: DesktopItem): Promise<void> => undefined;
let copySelectedItemsHandler = async (): Promise<void> => undefined;
let cutSelectedItemsHandler = async (): Promise<void> => undefined;
let deleteSelectedItemsHandler = async (): Promise<void> => undefined;
let pasteClipboardItemsHandler = async (): Promise<void> => undefined;
let startSelectedItemRenameHandler = (): void => undefined;
let cancelRenameHandler = (): void => undefined;
let consumeSuppressedItemClickHandler = (): boolean => false;

const {
  handleFileViewKeydown,
  handleGlobalFileViewKeydown,
  handleGridPointerDown,
  handleItemClick,
  isItemSelected,
  isSelecting,
  resolveSelectedItems,
  selectedPaths,
  selectionRectStyle,
  stopSelectionRectangle,
} = useBoxFileSelection({
  boxGridRef,
  boxItems,
  cancelFileRename: () => cancelRenameHandler(),
  closeContextMenu,
  consumeSuppressedItemClick: () => consumeSuppressedItemClickHandler(),
  copySelectedItems: () => copySelectedItemsHandler(),
  cutSelectedItems: () => cutSelectedItemsHandler(),
  deleteSelectedItems: () => deleteSelectedItemsHandler(),
  getDoubleClickOpenItems: () => desktopStore.settings.doubleClickOpenItems,
  isEditingTitle: () => isEditingTitle.value,
  openItem: (item) => openItemHandler(item),
  pasteClipboardItems: () => pasteClipboardItemsHandler(),
  startSelectedItemRename: () => startSelectedItemRenameHandler(),
});

const {
  cancelRename,
  commitRename,
  copySelectedItems,
  cutSelectedItems,
  deleteSelectedItems,
  editingPath,
  handleItemContextMenu,
  handleItemDoubleClick,
  openItem,
  pasteClipboardItems,
  renameDraft,
  startSelectedItemRename,
} = useBoxFileActions({
  boxGridRef,
  closeContextMenu,
  getBoxConflictPolicy: () => desktopStore.settings.boxConflictPolicy,
  getBoxFolderPath: () => box.value?.folderPath ?? "",
  getDoubleClickOpenItems: () => desktopStore.settings.doubleClickOpenItems,
  setNativeItemContextMenuOpen,
  removeBoxItemOrderPaths,
  removeBoxShellItems,
  refreshBoxFolderItems,
  replaceBoxItemOrderPath,
  resolveSelectedItems,
  selectedPaths,
  setLastError,
});
openItemHandler = openItem;
copySelectedItemsHandler = copySelectedItems;
cutSelectedItemsHandler = cutSelectedItems;
deleteSelectedItemsHandler = deleteSelectedItems;
pasteClipboardItemsHandler = pasteClipboardItems;
startSelectedItemRenameHandler = startSelectedItemRename;
cancelRenameHandler = cancelRename;

const {
  cancelFileDragSession,
  clearAcceptedFileDragWaiters,
  consumeSuppressedItemClick,
  handleBoxFileDragPayload,
  handleItemPointerDown,
  handleNativeDragDropEvent,
  isBoxFileDragActive,
  isItemDragging,
  markFileDragAccepted,
} = useBoxFileDrag({
  box,
  currentWindow,
  getDesktopPath: () => desktopStore.desktopPath,
  getSettings: () => desktopStore.settings,
  isFileRenaming: () => Boolean(editingPath.value),
  appendBoxShellItems,
  removeBoxShellItems,
  refreshBoxFolderItems,
  refreshCollapsedPreviewCloseSchedule,
  refreshDesktopSnapshot: () => desktopStore.refreshSnapshot(false),
  resolveSortInsertionPreview: resolveBoxSortInsertionPreview,
  resolveSelectedItems,
  selectedPaths,
  setDragHoveringBox,
  setSortInsertionPreview: setBoxSortInsertionPreview,
  setLastError,
  placeIncomingItemsInCurrentBox: placeIncomingItemsAtSortPreview,
  showSortInsertionPreview: showDraggedItemsSortPreview,
  sortItemsInSourceBox: sortDraggedItemsInCurrentBox,
});
consumeSuppressedItemClickHandler = consumeSuppressedItemClick;

const canResizeBox = computed(
  () =>
    Boolean(box.value) &&
    !isBoxFileDragActive.value &&
    !isBoxCollapsedToTitle.value &&
    !box.value?.locked,
);

useBoxWindowLifecycle({
  animateBoxIdleOpacity,
  applyCollapseWindowSize,
  box,
  boxId: props.boxId,
  boxIdleOpacity,
  canResizeBox,
  cancelFileDragSession,
  clearAcceptedFileDragWaiters,
  clearCollapseWindowAnimation,
  clearResizePersistState,
  closeContextMenu,
  currentWindow,
  ensureWindowInsideMonitor,
  handleBoxContextMenuState,
  handleBoxFileDragPayload,
  handleGlobalFileViewKeydown,
  handleNativeDragDropEvent,
  handleWindowMoved,
  initializeStore: () => desktopStore.initialize(),
  initializeStoreFromSnapshot: (token) => desktopStore.initializeFromStartupSnapshot(token),
  isBoxCollapsedToTitle,
  markFileDragAccepted,
  refreshBoxFolderItems,
  scheduleResizePersist,
  setLastError,
  startFolderRefreshPolling,
  stopFolderRefreshPolling,
  stopManualDragging,
  stopSelectionRectangle,
  syncNativeWindowResizable,
  syncWindowBoundsFromStore,
});

/**
 * 组件函数 ref 将文件网格 DOM 交给选择、重命名和框选逻辑使用。
 */
function setBoxGridRef(element: Element | ComponentPublicInstance | null): void {
  boxGridRef.value = element instanceof HTMLElement ? element : null;
}

/**
 * Store 错误统一以文本形式落到 lastError，避免各组合式逻辑重复 unknown 处理。
 */
function setLastError(message: string): void {
  desktopStore.lastError = message;
}

/**
 * 排序插入候选项记录真实 DOM 位置，拖拽中图标禁用 pointer-events 时仍可按几何位置计算落点。
 */
interface BoxSortInsertionCandidate {
  path: string;
  rect: DOMRect;
}

/**
 * 拖拽过程中的排序预览和最终排序共用同一套落点计算，保证视觉提示与落库顺序一致。
 */
function showDraggedItemsSortPreview(paths: string[], point: BoxScreenPoint): void {
  setBoxSortInsertionPreview(resolveBoxSortInsertionPreview(paths, point));
}

/**
 * 来源 Box 内释放拖拽时只调整展示顺序，释放到其他 Box 或桌面仍由拖拽逻辑执行真实文件移动。
 */
async function sortDraggedItemsInCurrentBox(
  paths: string[],
  point: BoxScreenPoint,
): Promise<void> {
  const insertionPreview = resolveBoxSortInsertionPreview(paths, point);
  if (!insertionPreview) {
    return;
  }

  await moveBoxItemsInOrder(
    paths,
    insertionPreview.targetPath,
    insertionPreview.placement,
  );
}

/**
 * 外部拖入和跨 Box 拖入都先完成真实文件/虚拟项写入，再按释放前锁定的插入预览保存展示顺序。
 */
async function placeIncomingItemsAtSortPreview(
  paths: string[],
  preview: BoxSortInsertionPreview | null,
): Promise<void> {
  if (!preview || paths.length === 0) {
    return;
  }

  await moveBoxItemsInOrder(paths, preview.targetPath, preview.placement);
}

/**
 * 网格排序落点按未拖动项的几何位置推断，避免拖动项本身或图标间隙导致提示跳动。
 */
function resolveBoxSortInsertionPreview(
  paths: string[],
  point: BoxScreenPoint,
): BoxSortInsertionPreview | null {
  if (!point.inside || !boxGridRef.value) {
    return null;
  }

  const draggedPathSet = new Set(paths);
  const candidates = Array.from(
    boxGridRef.value.querySelectorAll<HTMLElement>("[data-box-item-path]"),
  )
    .map((element) => ({
      path: element.dataset.boxItemPath ?? "",
      rect: element.getBoundingClientRect(),
    }))
    .filter(
      (candidate): candidate is BoxSortInsertionCandidate =>
        Boolean(candidate.path) && !draggedPathSet.has(candidate.path),
    );
  if (candidates.length === 0) {
    return null;
  }

  return resolveInsertionPreviewInRow(
    resolveInsertionCandidateRow(candidates, point.y),
    point.x,
  );
}

/**
 * 按 CSS Grid 的视觉行挑选候选项，指针处在行间距中时归到更接近的上一行或下一行。
 */
function resolveInsertionCandidateRow(
  candidates: BoxSortInsertionCandidate[],
  pointerY: number,
): BoxSortInsertionCandidate[] {
  const sortedCandidates = [...candidates].sort(
    (left, right) => left.rect.top - right.rect.top || left.rect.left - right.rect.left,
  );
  const rows: BoxSortInsertionCandidate[][] = [];
  const firstCandidateHeight = sortedCandidates[0]?.rect.height ?? 0;
  const rowMergeThreshold = Math.max(4, firstCandidateHeight * 0.25);

  for (const candidate of sortedCandidates) {
    const currentRow = rows[rows.length - 1];
    if (currentRow && Math.abs(currentRow[0].rect.top - candidate.rect.top) <= rowMergeThreshold) {
      currentRow.push(candidate);
      continue;
    }

    rows.push([candidate]);
  }

  const rowGapTolerance = Math.max(8, desktopStore.settings.boxIconGapY / 2);
  const targetRow =
    rows.find((row) => pointerY <= resolveRowBottom(row) + rowGapTolerance) ??
    rows[rows.length - 1];

  return [...targetRow].sort((left, right) => left.rect.left - right.rect.left);
}

/**
 * 同一行内以每个图标横向中线作为前后分界，确保插入线不会依赖颜色或 hover 才可辨认。
 */
function resolveInsertionPreviewInRow(
  row: BoxSortInsertionCandidate[],
  pointerX: number,
): BoxSortInsertionPreview {
  let lastCandidate = row[0];
  for (const candidate of row) {
    if (pointerX <= candidate.rect.left + candidate.rect.width / 2) {
      return {
        placement: "before",
        targetPath: candidate.path,
      };
    }

    lastCandidate = candidate;
  }

  return {
    placement: "after",
    targetPath: lastCandidate.path,
  };
}

/**
 * 行底部取该行最高图标的真实边界，兼容重命名输入框或标签展开导致的单项高度变化。
 */
function resolveRowBottom(row: BoxSortInsertionCandidate[]): number {
  return Math.max(...row.map((candidate) => candidate.rect.bottom));
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
      @mouseleave="handleBoxMouseLeaveWithHandoff"
    >
      <!--
        图标态：图标模式收缩闲置的形态，整面只渲染一个图标入口。
        悬停/点击交给现有展开调度回到完整态，右键直接唤出 Box 菜单，
        按下即可拖动（原生拖动循环），与完整态标题栏的手感一致。
        收缩动画进行中仍渲染完整内容（已淡出），完成后才切入图标，
        避免窗口还在缩小时内容形态提前跳变。
      -->
      <button
        v-if="isBoxInIconState && !isCollapseAnimating"
        aria-label="展开 Box"
        class="dasktop-icon-state-button grid h-full w-full place-items-center"
        :style="{ '--dasktop-icon-fade-ms': `${desktopStore.settings.boxIconFadeInMs}ms` }"
        type="button"
        @click="openCollapsedPreviewForActiveInteraction()"
        @contextmenu.prevent.stop="toggleContextMenu"
        @mousedown.left.stop="startDragging($event)"
      >
        <img
          v-if="iconStateImageSrc"
          :alt="box.title"
          class="h-10 w-10 rounded-[var(--dasktop-box-radius)] object-contain"
          draggable="false"
          :src="iconStateImageSrc"
        />
        <component
          :is="iconStateBuiltinComponent"
          v-else-if="iconStateBuiltinComponent"
          aria-hidden="true"
          class="text-slate-400 dark:text-slate-500"
          :size="36"
        />
        <Folder v-else aria-hidden="true" class="text-slate-400 dark:text-slate-500" :size="32" />
      </button>

      <template v-else>
        <BoxResizeHandles
          :can-resize-box="canResizeBox"
          :resize-handles="resizeHandles"
          @resize-handle-mouse-enter="handleResizeHandleMouseEnter"
          @resize-handle-mouse-leave="handleResizeHandleMouseLeave"
          @start-resizing="startResizing"
        />

        <BoxHeader
          v-model:title-draft="titleDraft"
          :box="box"
          :box-title-area-style="boxTitleAreaStyle"
          :box-title-order-class="boxTitleOrderClass"
          :is-editing-title="isEditingTitle"
          :set-title-input-ref="setTitleInputRef"
          :title-text-size="desktopStore.settings.boxTitleTextSize"
          @cancel-title-editing="cancelTitleEditing"
          @commit-title-editing="commitTitleEditing"
          @start-dragging="startDragging"
          @start-title-editing="startTitleEditing"
          @title-mouse-enter="handleBoxTitleMouseEnter"
          @title-mouse-leave="handleBoxTitleMouseLeave"
          @toggle-context-menu="toggleContextMenu"
        />

        <BoxFileGrid
          :box-body-style="boxBodyStyle"
          :box-grid-overflow-class="boxGridOverflowClass"
          :box-grid-style="boxGridStyle"
          :box-items="boxItems"
          :editing-path="editingPath"
          :is-box-file-drag-active="isBoxFileDragActive"
          :is-item-dragging="isItemDragging"
          :is-item-selected="isItemSelected"
          :is-selecting="isSelecting"
          :rename-draft="renameDraft"
          :selection-rect-style="selectionRectStyle"
          :set-grid-ref="setBoxGridRef"
          :sort-insertion-preview="boxSortInsertionPreview"
          :settings="desktopStore.settings"
          @cancel-rename="cancelRename"
          @commit-rename="commitRename"
          @file-view-keydown="handleFileViewKeydown"
          @grid-pointer-down="handleGridPointerDown"
          @item-click="handleItemClick"
          @item-context-menu="handleItemContextMenu"
          @item-double-click="handleItemDoubleClick"
          @item-pointer-down="handleItemPointerDown"
          @rename-draft-change="renameDraft = $event"
        />
      </template>
    </article>
  </main>
</template>
