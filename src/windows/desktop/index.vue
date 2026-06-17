<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import type { CSSProperties } from "vue";
import { MoreHorizontal } from "@lucide/vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import DesktopIcon from "./components/DesktopIcon.vue";
import { DESKTOP_ICON_VIEW } from "./config/desktopIcon";
import { useBoxCollapsePreview } from "./composables/useBoxCollapsePreview";
import { useBoxContextMenu } from "./composables/useBoxContextMenu";
import { useBoxItemDragSession } from "./composables/useBoxItemDragSession";
import { useBoxTitleEditing } from "./composables/useBoxTitleEditing";
import { useBoxWindowFrame } from "./composables/useBoxWindowFrame";
import { useDesktopStore } from "@/entities/desktopBox/store";
import type { DesktopItem } from "@/entities/desktopItem/types";
import { showNativeItemContextMenu } from "@/entities/desktopItem/api";
import { listenBoxItemDrag, listenBoxItemDragAccepted } from "@/shared/ipc/boxItemDrag";
import { preloadBoxContextMenuWindow } from "@/entities/desktopBox/windows";
import { listenBoxContextMenuState } from "@/shared/ipc/boxContextMenu";
import { BOX_WINDOW_INTERACTION_TIMING } from "@/entities/desktopBox/layout";

const props = defineProps<{
  boxId: string;
}>();

const desktopStore = useDesktopStore();
const currentWindow = getCurrentWindow();
const unlistenFns: UnlistenFn[] = [];

/**
 * 桌面 Box 窗口只保留 DOM 锚点和 Store 派生数据，具体交互状态交给下方组合式逻辑维护。
 */
const boxSurfaceRef = ref<HTMLElement | null>(null);
const box = computed(() => desktopStore.boxes.find((item) => item.id === props.boxId));
const boxItems = computed(() => (box.value ? desktopStore.getBoxItems(box.value.id) : []));
const boxGridRef = ref<HTMLElement | null>(null);

/**
 * 组合式逻辑之间需要互相读取状态；使用延迟赋值的读取函数避免 setup 阶段出现循环初始化。
 */
let readContextMenuOpen = (): boolean => false;
let readEditingTitle = (): boolean => false;
let readBoxCollapsedToTitle = (): boolean => false;
let readCollapseWindowSizeApplying = (): boolean => false;
let closeActiveContextMenu = (): void => undefined;
let openCollapsedPreviewForActiveInteractionHandler = (): void => undefined;
let refreshCollapsedPreviewCloseScheduleHandler = (): void => undefined;

/**
 * 窗口框架层封装 Tauri 位置、尺寸、手动拖动、吸附和 resize 落库，组件只关心事件入口。
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
  resolveBoxItemDragLocalPoint,
  resolveWindowClientPhysicalPointToScreen,
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
 * 收缩预览层统一处理 roll-up 展开、闲置透明度和收起动画，避免这些状态散落在模板事件里。
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
  syncPointerHoverFromScreenPoint,
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
  resolvePointerLocalPoint: (screenX, screenY) =>
    resolveBoxItemDragLocalPoint(screenX, screenY),
  resizePersistSettleMs: BOX_WINDOW_INTERACTION_TIMING.resizePersistSettleMs,
});

/**
 * 回填跨组合式逻辑读取入口，后续事件触发时能读取到真实的收缩预览状态。
 */
readBoxCollapsedToTitle = () => isBoxCollapsedToTitle.value;
readCollapseWindowSizeApplying = () => isApplyingCollapseWindowSize();
openCollapsedPreviewForActiveInteractionHandler = openCollapsedPreviewForActiveInteraction;
refreshCollapsedPreviewCloseScheduleHandler = refreshCollapsedPreviewCloseSchedule;

/**
 * 拖拽会话层统一处理 Box 内排序、跨 Box 移动、外部文件拖入和拖影 IPC。
 */
const {
  acceptBoxItemDragSession,
  beginExternalFileDrag,
  cancelActiveBoxItemDrag,
  cancelExternalFileDrag,
  clearExternalFileDragReleaseProbe,
  dragInsertLineStyle,
  draggingBoxItemPath,
  finishBoxItemPointerDrag,
  finishExternalFileDrag,
  handleBoxGridDragOver,
  handleBoxGridDrop,
  handleBoxItemDragPayload,
  isBoxItemDragActive,
  moveExternalFileDrag,
  startBoxItemPointerDrag,
} = useBoxItemDragSession({
  box,
  boxGridRef,
  closeContextMenu: () => closeActiveContextMenu(),
  refreshCollapsedPreviewCloseSchedule: () => refreshCollapsedPreviewCloseSchedule(),
  resolveBoxItemDragLocalPoint: (screenX, screenY) =>
    resolveBoxItemDragLocalPoint(screenX, screenY),
  resolveWindowClientPhysicalPointToScreen: (x, y) =>
    resolveWindowClientPhysicalPointToScreen(x, y),
  setDragHoveringBox: (isHovering) => setDragHoveringBox(isHovering),
  setLastError: (message) => {
    desktopStore.lastError = message;
  },
  syncPointerHoverFromScreenPoint: (screenX, screenY) =>
    syncPointerHoverFromScreenPoint(screenX, screenY),
});

/**
 * resize 能力同时受锁定、收缩态和 Box 图标拖拽状态影响；外部文件拖入只展示落点，不锁住缩放热区。
 */
const canResizeBox = computed(
  () =>
    Boolean(box.value) &&
    !isBoxItemDragActive.value &&
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

/**
 * 网格列宽由图标尺寸和文件名宽度共同决定，保证标签变宽时图标列不会互相覆盖。
 */
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
  setLastError: (message) => {
    desktopStore.lastError = message;
  },
});
readContextMenuOpen = () => isContextMenuOpen.value;
closeActiveContextMenu = closeContextMenu;

/**
 * 标题编辑只修改 Box 展示名，提交前会先关闭独立菜单窗口避免焦点和 blur 顺序互相干扰。
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

watch(boxIdleOpacity, () => {
  animateBoxIdleOpacity(true);
});

watch(
  canResizeBox,
  (canResize) => {
    syncNativeWindowResizable(canResize);
  },
  { immediate: true },
);

/**
 * 挂载时先恢复窗口几何和收缩尺寸，再注册跨窗口 IPC 与原生拖放监听，避免早到事件读到未初始化状态。
 */
onMounted(async () => {
  await desktopStore.initialize();
  await syncWindowBoundsFromStore();
  await applyCollapseWindowSize(false);
  await nextTick();
  animateBoxIdleOpacity(false);
  syncNativeWindowResizable(canResizeBox.value);
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

  unlistenFns.push(
    await listenBoxItemDrag(async ({ payload }) => {
      await handleBoxItemDragPayload(payload);
    }),
  );

  unlistenFns.push(
    await listenBoxItemDragAccepted(({ payload }) => {
      acceptBoxItemDragSession(payload.sessionId);
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
      if (!box.value) {
        return;
      }

      if (payload.type === "enter") {
        await beginExternalFileDrag(payload.paths, payload.position.x, payload.position.y);
        return;
      }

      if (payload.type === "over") {
        await moveExternalFileDrag(payload.position.x, payload.position.y);
        return;
      }

      if (payload.type === "drop") {
        await finishExternalFileDrag(payload.paths, payload.position.x, payload.position.y);
        return;
      }

      cancelExternalFileDrag({ deferHoverClearUntilRelease: true });
    }),
  );
});

/**
 * 卸载时按拖拽、窗口、菜单、动画、监听的顺序清理，防止异步回调在窗口关闭后继续写状态。
 */
onUnmounted(() => {
  cancelActiveBoxItemDrag();
  cancelExternalFileDrag();
  clearExternalFileDragReleaseProbe();
  stopManualDragging(false);
  closeContextMenu();
  clearCollapseWindowAnimation();
  clearResizePersistState();
  for (const unlisten of unlistenFns) {
    unlisten();
  }
});

/**
 * Windows Shell 右键菜单由后端直接接管，前端只负责传递屏幕坐标和目标路径。
 */
function openNativeItemContextMenu(event: MouseEvent, item: DesktopItem): void {
  closeContextMenu();
  void showNativeItemContextMenu(item.path, event.screenX, event.screenY).catch((error) => {
    desktopStore.lastError = error instanceof Error ? error.message : String(error);
  });
}

</script>

<template>
  <main
    class="h-screen w-screen overflow-hidden bg-transparent p-0"
    :class="isBoxItemDragActive || box?.locked ? 'cursor-default' : ''"
    @click="closeContextMenu"
  >
    <article
      v-if="box"
      ref="boxSurfaceRef"
      class="dasktop-box-surface relative flex h-full w-full flex-col overflow-hidden text-slate-950 dark:text-white"
      :style="boxSurfaceStyle"
      @dragover.prevent="handleBoxGridDragOver"
      @drop.prevent="handleBoxGridDrop"
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

      <!-- 标题层级高于内容区，避免底部收缩时图标网格的过渡帧遮住标题文字。 -->
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
        class="dasktop-scrollarea dasktop-box-scrollarea relative grid min-h-0 flex-1 p-2.5 transition-[opacity,transform] duration-180 ease-out"
        :class="
          [
            boxGridOverflowClass,
            boxItems.length === 0
              ? 'content-center place-items-center justify-center'
              : 'content-start items-start justify-start',
          ]
        "
        :style="[boxGridStyle, boxBodyStyle]"
      >
        <DesktopIcon
          v-for="item in boxItems"
          :key="item.path"
          :double-click-open="desktopStore.settings.doubleClickOpenItems"
          :drag-interaction-disabled="isBoxItemDragActive"
          :dragging="isBoxItemDragActive && draggingBoxItemPath === item.path"
          :icon-size="desktopStore.settings.boxIconSize"
          :item="item"
          :label-text-size="desktopStore.settings.boxLabelTextSize"
          :label-width="desktopStore.settings.boxFilenameWidth"
          :name-display-mode="desktopStore.settings.nameDisplayMode"
          :radius-size="desktopStore.settings.boxCornerRadius"
          :show-label="desktopStore.settings.showItemLabels"
          :show-shortcut-arrow="desktopStore.settings.showShortcutArrow"
          @box-pointer-drag-end="finishBoxItemPointerDrag"
          @box-pointer-drag-start="startBoxItemPointerDrag"
          @native-context-menu="openNativeItemContextMenu"
        />

        <div
          v-if="dragInsertLineStyle"
          aria-hidden="true"
          class="dasktop-box-drag-insert-line pointer-events-none absolute z-20 w-[2px] rounded-full bg-[#2f6bff]"
          :style="dragInsertLineStyle"
        />

        <div
          v-if="boxItems.length === 0"
          class="col-span-full flex min-h-[120px] w-full flex-col items-center justify-center px-5 text-center"
        >
          <strong class="block text-center text-[13px] font-semibold text-slate-900 dark:text-white">这个 Box 还是空的</strong>
          <p class="mt-1 max-w-[180px] text-center text-[12px] leading-5 text-slate-600 dark:text-slate-300">把桌面文件拖进来就能开始整理。</p>
        </div>
      </div>
    </article>
  </main>
</template>
