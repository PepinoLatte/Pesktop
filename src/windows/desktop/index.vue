<script setup lang="ts">
import { computed, ref } from "vue";
import type { ComponentPublicInstance, CSSProperties } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
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
import { useDesktopStore } from "@/entities/desktopBox/store";
import { BOX_WINDOW_INTERACTION_TIMING } from "@/entities/desktopBox/layout";
import type { DesktopItem } from "@/entities/desktopItem/types";

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
);
readEditingTitle = () => isEditingTitle.value;

const {
  boxItems,
  refreshBoxFolderItems,
  startFolderRefreshPolling,
  stopFolderRefreshPolling,
} = useBoxFileItems({
  box,
  setLastError,
});

let openItemHandler = async (_item: DesktopItem): Promise<void> => undefined;
let deleteSelectedItemsHandler = async (): Promise<void> => undefined;
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
  deleteSelectedItems: () => deleteSelectedItemsHandler(),
  getDoubleClickOpenItems: () => desktopStore.settings.doubleClickOpenItems,
  isEditingTitle: () => isEditingTitle.value,
  openItem: (item) => openItemHandler(item),
  startSelectedItemRename: () => startSelectedItemRenameHandler(),
});

const {
  cancelRename,
  commitRename,
  deleteSelectedItems,
  editingPath,
  handleItemContextMenu,
  handleItemDoubleClick,
  openItem,
  renameDraft,
  startSelectedItemRename,
} = useBoxFileActions({
  boxGridRef,
  closeContextMenu,
  getDoubleClickOpenItems: () => desktopStore.settings.doubleClickOpenItems,
  refreshBoxFolderItems,
  resolveSelectedItems,
  selectedPaths,
  setLastError,
});
openItemHandler = openItem;
deleteSelectedItemsHandler = deleteSelectedItems;
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
  refreshBoxFolderItems,
  refreshCollapsedPreviewCloseSchedule,
  refreshDesktopSnapshot: () => desktopStore.refreshSnapshot(false),
  resolveSelectedItems,
  selectedPaths,
  setDragHoveringBox,
  setLastError,
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
    </article>
  </main>
</template>
