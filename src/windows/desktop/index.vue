<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import type { ComponentPublicInstance, CSSProperties } from "vue";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import BoxFileGrid from "@/windows/desktop/components/BoxFileGrid.vue";
import BoxHeader from "@/windows/desktop/components/BoxHeader.vue";
import BoxIconGlyph from "@/windows/desktop/components/BoxIconGlyph.vue";
import BoxResizeHandles from "@/windows/desktop/components/BoxResizeHandles.vue";
import { openDesktopItem } from "@/entities/desktopItem/api";
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
import { BOX_WINDOW_INTERACTION_TIMING } from "@/entities/desktopBox/layout";
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
  confirmBoxActiveState,
  handleBoxMouseEnter,
  handleBoxMouseLeave,
  handleBoxTitleMouseEnter,
  handleBoxTitleMouseLeave,
  isApplyingCollapseWindowSize,
  isBoxCollapsedToTitle,
  openCollapsedPreviewForActiveInteraction,
  refreshCollapsedPreviewCloseSchedule,
  setDragHoveringBox,
  setNativeItemContextMenuOpen,
} = useBoxCollapsePreview({
  applyWindowFrame: (frame) => applyCollapseWindowFrame(frame),
  box,
  boxSurfaceRef,
  getBoxBackgroundOpacity: () => desktopStore.settings.boxBackgroundOpacity,
  getBoxBlurStrength: () => desktopStore.settings.boxBlurStrength,
  getBoxCollapseAnimationMs: () => desktopStore.getBoxCollapseAnimationMs(),
  getBoxCollapseDelayMs: () => desktopStore.getBoxCollapseDelayMs(),
  getBoxCornerRadius: () => desktopStore.settings.boxCornerRadius,
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
  resolvePointerLocalPoint: async () => ({ inside: false }),
  resizePersistSettleMs: BOX_WINDOW_INTERACTION_TIMING.resizePersistSettleMs,
});

readBoxCollapsedToTitle = () => isBoxCollapsedToTitle.value;
readCollapseWindowSizeApplying = () => isApplyingCollapseWindowSize();
openCollapsedPreviewForActiveInteractionHandler = openCollapsedPreviewForActiveInteraction;
refreshCollapsedPreviewCloseScheduleHandler = refreshCollapsedPreviewCloseSchedule;

/**
 * 当 Box 尺寸缩至极小（宽或高 <= 110px）时，自动切换为超简约的桌面小图标小部件模式
 */
const isCompactIconMode = computed(() =>
  Boolean(box.value && (box.value.width <= 110 || box.value.height <= 110)),
);

/**
 * 双击小图标时在系统资源管理器中打开对应 Box 绑定的物理存储文件夹
 */
function openBoxFolder(): void {
  if (box.value?.folderPath) {
    void openDesktopItem(box.value.folderPath);
  }
}

/**
 * 小图标模式下鼠标悬停临时展开为大面板的展示状态
 */
const isCompactHoverExpanded = ref(false);
let compactHoverExpandTimer: ReturnType<typeof window.setTimeout> | null = null;
let compactHoverCloseTimer: ReturnType<typeof window.setTimeout> | null = null;
let originalCompactBounds: { height: number; width: number } | null = null;

const EXPANDED_PANEL_WIDTH = 320;
const EXPANDED_PANEL_HEIGHT = 360;

/**
 * 清理小图标展开延时计时器，避免快速掠过时误触发大面板展开
 */
function clearCompactExpandTimer(): void {
  if (compactHoverExpandTimer) {
    window.clearTimeout(compactHoverExpandTimer);
    compactHoverExpandTimer = null;
  }
}

/**
 * 取消小图标收回计时器，鼠标重新移入大面板时保持展开
 */
function clearCompactCloseTimer(): void {
  if (compactHoverCloseTimer) {
    window.clearTimeout(compactHoverCloseTimer);
    compactHoverCloseTimer = null;
  }
}

/**
 * 小图标模式下鼠标移入，延时 120ms 平滑展开为完整大面板供用户直接浏览与交互。
 * 严禁调用 openCollapsedPreviewForActiveInteractionHandler，防止触发普通标题收缩的尺寸冲突。
 */
function handleCompactMouseEnter(): void {
  clearCompactCloseTimer();
  if (!isCompactIconMode.value || isCompactHoverExpanded.value) {
    return;
  }

  clearCompactExpandTimer();
  compactHoverExpandTimer = window.setTimeout(async () => {
    compactHoverExpandTimer = null;
    if (!box.value) {
      return;
    }

    originalCompactBounds = {
      height: box.value.height,
      width: box.value.width,
    };

    await currentWindow.setSize(new LogicalSize(EXPANDED_PANEL_WIDTH, EXPANDED_PANEL_HEIGHT));
    isCompactHoverExpanded.value = true;
  }, 120);
}

/**
 * 鼠标离开大面板后延时收起恢复为小图标尺寸。
 * 在菜单打开、标题或文件编辑、正在拖拽或缩放时严禁自动收回。
 */
function handleCompactMouseLeave(): void {
  clearCompactExpandTimer();

  if (!isCompactHoverExpanded.value) {
    return;
  }

  clearCompactCloseTimer();
  compactHoverCloseTimer = window.setTimeout(async () => {
    compactHoverCloseTimer = null;
    if (
      readContextMenuOpen() ||
      readEditingTitle() ||
      Boolean(editingPath.value) ||
      isResizingBox.value ||
      isManualDraggingBox.value ||
      isBoxFileDragActive.value
    ) {
      return;
    }

    if (originalCompactBounds && box.value) {
      await currentWindow.setSize(
        new LogicalSize(originalCompactBounds.width, originalCompactBounds.height),
      );
    }
    isCompactHoverExpanded.value = false;
    originalCompactBounds = null;
  }, Math.max(desktopStore.getBoxCollapseDelayMs(), 260));
}

/**
 * Box 根容器鼠标进入事件：根据当前是否处于小图标模式，智能分发至小图标展开或常规展开
 */
function handleSurfaceMouseEnter(): void {
  clearCompactCloseTimer();
  if (isCompactIconMode.value) {
    if (!isCompactHoverExpanded.value) {
      handleCompactMouseEnter();
    }
    return;
  }

  handleBoxMouseEnter();
}

/**
 * Box 根容器鼠标离开事件：根据当前形态分发收起动作
 */
function handleSurfaceMouseLeave(): void {
  if (isCompactIconMode.value) {
    clearCompactExpandTimer();
    if (isCompactHoverExpanded.value) {
      handleCompactMouseLeave();
    }
    return;
  }

  handleBoxMouseLeave();
}

/**
 * 监听小图标模式切换，当用户从菜单恢复为常规窗口时，立刻清理展开临时态
 */
watch(isCompactIconMode, (compact) => {
  if (!compact) {
    clearCompactExpandTimer();
    clearCompactCloseTimer();
    isCompactHoverExpanded.value = false;
    originalCompactBounds = null;
  }
});

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
  clearCompactExpandTimer();
  clearCompactCloseTimer();
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

/**
 * 是否允许通过四方边缘把手缩放 Box：
 * 在小图标模式、折叠至标题状态、拖动文件或 Box 被锁定时完全隐藏并禁用。
 */
const canResizeBox = computed(
  () =>
    Boolean(box.value) &&
    !isBoxFileDragActive.value &&
    !isBoxCollapsedToTitle.value &&
    !box.value?.locked &&
    !isCompactIconMode.value,
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
      class="dasktop-box-surface group/box relative flex h-full w-full flex-col overflow-hidden text-slate-950 dark:text-white"
      :style="boxSurfaceStyle"
      @click="confirmBoxActiveState"
      @mouseenter="handleSurfaceMouseEnter"
      @mouseleave="handleSurfaceMouseLeave"
    >
      <BoxResizeHandles
        :can-resize-box="canResizeBox"
        :resize-handles="resizeHandles"
        @resize-handle-mouse-enter="handleResizeHandleMouseEnter"
        @resize-handle-mouse-leave="handleResizeHandleMouseLeave"
        @start-resizing="startResizing"
      />

      <!-- 小图标简约模式：仅展示纯净图标部件，隐藏文件名与把手，避免内部销毁触发额外 mouseleave -->
      <div
        v-if="isCompactIconMode && !isCompactHoverExpanded"
        class="flex h-full w-full items-center justify-center p-1 select-none cursor-default"
        :title="`${box.title} (悬停展开大面板，双击打开文件夹，右键打开菜单)`"
        @click="confirmBoxActiveState"
        @contextmenu.prevent.stop="toggleContextMenu"
        @dblclick.stop="openBoxFolder"
        @mousedown.left="startDragging($event)"
      >
        <div class="relative grid place-items-center size-11 rounded-[13px] bg-[#2f6bff]/15 text-[#2f6bff] shadow-sm transition-transform duration-150 hover:scale-105 active:scale-95 dark:bg-[#2f6bff]/25 dark:text-white">
          <BoxIconGlyph :icon="box.icon" :size="26" />
        </div>
      </div>

      <!-- 常规完整模式：展示标题栏与文件网格 -->
      <template v-else>
        <BoxHeader
          v-model:title-draft="titleDraft"
          :box="box"
          :box-title-area-style="boxTitleAreaStyle"
          :box-title-order-class="boxTitleOrderClass"
          :is-editing-title="isEditingTitle"
          :set-title-input-ref="setTitleInputRef"
          :title-font-size="desktopStore.settings.boxTitleTextSize"
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
