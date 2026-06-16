<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { MoreHorizontal, RefreshCw, Settings, Trash2 } from "@lucide/vue";
import {
  LogicalPosition,
  LogicalSize,
  PhysicalPosition,
  cursorPosition,
  currentMonitor,
  getCurrentWindow,
  primaryMonitor,
} from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import DesktopIcon from "./components/DesktopIcon.vue";
import { useDesktopStore } from "./store/desktopStore";
import { openSettingsWindow } from "../../shared/window/boxWindows";

const props = defineProps<{
  boxId: string;
}>();

const desktopStore = useDesktopStore();
const currentWindow = getCurrentWindow();
const unlistenFns: UnlistenFn[] = [];
/**
 * 程序主动定位后的短锁用于过滤 setPosition 自己触发的移动事件。
 */
const WINDOW_POSITION_APPLY_LOCK_MS = 80;
/**
 * 系统缩放拖拽可能吞掉 WebView 的 mouseup，短暂静止后保存最终边界作为兜底。
 */
const RESIZE_PERSIST_SETTLE_MS = 180;

/**
 * 物理坐标用于和 Tauri 窗口移动事件保持同一坐标体系，高 DPI 下再单独换算逻辑坐标。
 */
interface PhysicalWindowPoint {
  x: number;
  y: number;
}

/**
 * 屏幕工作区使用物理坐标保存，拖动时可直接和窗口物理坐标比较，避免高 DPI 下贴边偏移。
 */
interface PhysicalWorkArea {
  height: number;
  width: number;
  x: number;
  y: number;
}

let isApplyingWindowPosition = false;
let windowPositionApplyVersion = 0;
let manualDragState: ManualDragState | null = null;
let manualDragCleanup: (() => void) | null = null;
let resizeReleaseCleanup: (() => void) | null = null;
let resizePersistTimer: ReturnType<typeof window.setTimeout> | null = null;

/**
 * 手写拖动状态保存鼠标与窗口左上角的偏移，实时移动时复用窗口尺寸和缩放系数。
 */
interface ManualDragState {
  cursorStartX: number;
  cursorStartY: number;
  height: number;
  lastPosition: PhysicalWindowPoint;
  offsetX: number;
  offsetY: number;
  scaleFactor: number;
  screenStartX: number;
  screenStartY: number;
  width: number;
  workArea?: PhysicalWorkArea;
}

type ResizeDirection =
  | "East"
  | "North"
  | "NorthEast"
  | "NorthWest"
  | "South"
  | "SouthEast"
  | "SouthWest"
  | "West";

/**
 * 无边框 Box 需要显式提供缩放热区，否则透明窗口在 Windows 上不一定有稳定边缘命中。
 */
const resizeHandles: Array<{
  direction: ResizeDirection;
  className: string;
}> = [
  { direction: "North", className: "left-4 right-4 top-[-3px] h-2 cursor-ns-resize" },
  { direction: "South", className: "bottom-[-3px] left-4 right-4 h-2 cursor-ns-resize" },
  { direction: "West", className: "bottom-4 left-[-3px] top-4 w-2 cursor-ew-resize" },
  { direction: "East", className: "bottom-4 right-[-3px] top-4 w-2 cursor-ew-resize" },
  { direction: "NorthWest", className: "left-[-4px] top-[-4px] size-4 cursor-nwse-resize" },
  { direction: "NorthEast", className: "right-[-4px] top-[-4px] size-4 cursor-nesw-resize" },
  { direction: "SouthWest", className: "bottom-[-4px] left-[-4px] size-4 cursor-nesw-resize" },
  { direction: "SouthEast", className: "bottom-[-4px] right-[-4px] size-4 cursor-nwse-resize" },
];

const contextMenu = ref({ open: false, x: 0, y: 0 });
const isEditingTitle = ref(false);
const titleDraft = ref("");
const titleInputRef = ref<HTMLInputElement | null>(null);
const box = computed(() => desktopStore.boxes.find((item) => item.id === props.boxId));
const boxItems = computed(() =>
  box.value
    ? box.value.itemPaths
        .map((path) => desktopStore.findItem(path))
        .filter((item) => item !== undefined)
    : [],
);

onMounted(async () => {
  await desktopStore.initialize();
  await syncWindowBoundsFromStore();

  unlistenFns.push(
    await currentWindow.onMoved(async ({ payload }) => {
      await handleWindowMoved(payload.x, payload.y);
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
      if (payload.type !== "drop" || !box.value) {
        return;
      }

      await desktopStore.assignDroppedPathsToBox(payload.paths, box.value.id);
    }),
  );
});

onUnmounted(() => {
  stopManualDragging(false);
  clearResizePersistState();
  for (const unlisten of unlistenFns) {
    unlisten();
  }
});

/**
 * 初始化时用持久化数据校准窗口尺寸，防止分辨率变化后窗口状态与数据库脱节。
 */
async function syncWindowBoundsFromStore(): Promise<void> {
  if (!box.value) {
    return;
  }

  await currentWindow.setSize(new LogicalSize(box.value.width, box.value.height));
  await currentWindow.setPosition(new LogicalPosition(box.value.x, box.value.y));
  await ensureWindowInsideMonitor();
}

/**
 * 外部窗口移动只负责持久化，手写拖动期间的位置由拖动循环统一保存。
 */
async function handleWindowMoved(x: number, y: number): Promise<void> {
  if (isApplyingWindowPosition || manualDragState) {
    return;
  }

  await persistWindowPositionFromPhysical(x, y);
}

/**
 * 拖动只从标题栏触发，避免图标区域的拖拽和窗口移动互相抢事件。
 */
function startDragging(event: MouseEvent): void {
  if (event.button !== 0 || event.detail > 1 || isEditingTitle.value) {
    return;
  }

  closeContextMenu();
  void startManualDragging(event);
}

/**
 * Box 标题双击进入编辑态，只修改 Dasktop 的分组名称，不重命名真实桌面文件。
 */
function startTitleEditing(event: MouseEvent): void {
  event.stopPropagation();

  if (!box.value) {
    return;
  }

  closeContextMenu();
  titleDraft.value = box.value.title;
  isEditingTitle.value = true;
  void nextTick(() => {
    titleInputRef.value?.focus();
    titleInputRef.value?.select();
  });
}

/**
 * 保存标题时保留非空约束，避免用户误删名称后 Box 在设置页中不可识别。
 */
async function commitTitleEditing(): Promise<void> {
  if (!box.value || !isEditingTitle.value) {
    return;
  }

  const nextTitle = titleDraft.value.trim() || box.value.title;
  isEditingTitle.value = false;

  if (nextTitle === box.value.title) {
    return;
  }

  await desktopStore.updateBox({
    ...box.value,
    title: nextTitle,
  });
}

/**
 * 取消编辑只还原标题草稿，不触发数据库写入。
 */
function cancelTitleEditing(): void {
  titleDraft.value = box.value?.title ?? "";
  isEditingTitle.value = false;
}

/**
 * 缩放从窗口边缘热区触发，保持 Box 没有最小化、最大化、关闭按钮的桌面组件形态。
 */
function startResizing(direction: ResizeDirection, event: MouseEvent): void {
  if (event.button !== 0) {
    return;
  }

  stopManualDragging(false);
  closeContextMenu();
  bindResizeReleaseEvents();
  void currentWindow.startResizeDragging(direction);
}

/**
 * 更多菜单限制在当前 Box 窗口内，避免菜单跑出透明窗口区域后不可点击。
 */
function openContextMenu(event: MouseEvent): void {
  const menuWidth = 190;
  const menuHeight = 128;

  contextMenu.value = {
    open: true,
    x: Math.min(Math.max(event.clientX, 8), Math.max(window.innerWidth - menuWidth, 8)),
    y: Math.min(Math.max(event.clientY, 8), Math.max(window.innerHeight - menuHeight, 8)),
  };
}

/**
 * 菜单只由右上角更多按钮触发，Box 区域右键交还系统默认处理。
 */
function closeContextMenu(): void {
  contextMenu.value.open = false;
}

/**
 * Box 菜单只负责唤起设置页，具体设置仍由主窗口统一承载。
 */
async function openSettingsFromMenu(): Promise<void> {
  closeContextMenu();
  await openSettingsWindow();
}

/**
 * 右键刷新只重新读取桌面目录，不改变任何 Box 布局。
 */
async function refreshDesktopFromMenu(): Promise<void> {
  closeContextMenu();
  await desktopStore.refreshSnapshot();
}

/**
 * 删除 Box 只删除当前分组窗口和映射，真实桌面文件不会被删除。
 */
async function deleteCurrentBox(): Promise<void> {
  if (!box.value) {
    return;
  }

  closeContextMenu();
  await desktopStore.deleteBox(box.value.id);
  await currentWindow.close();
}

/**
 * 持久化窗口位置用于下次启动恢复 Box，并广播给其他独立 Box 作为后续吸附参照。
 */
async function persistWindowPosition(x: number, y: number): Promise<void> {
  if (!box.value) {
    return;
  }

  await desktopStore.updateBox(
    {
      ...box.value,
      x: Math.round(x),
      y: Math.round(y),
    },
    {
      sanitize: false,
    },
  );
}

/**
 * Tauri 移动事件返回物理坐标，持久化前转回逻辑坐标，保证高 DPI 下重启位置不漂移。
 */
async function persistWindowPositionFromPhysical(x: number, y: number): Promise<void> {
  const scaleFactor = await currentWindow.scaleFactor();
  const logicalPosition = new PhysicalPosition(x, y).toLogical(scaleFactor);

  await persistWindowPosition(logicalPosition.x, logicalPosition.y);
}

/**
 * 持久化窗口完整边界，用于缩放结束后一次性保存位置和尺寸。
 */
async function persistWindowBounds(
  x: number,
  y: number,
  width: number,
  height: number,
): Promise<void> {
  if (!box.value) {
    return;
  }

  await desktopStore.updateBox(
    {
      ...box.value,
      x: Math.round(x),
      y: Math.round(y),
      width: Math.round(width),
      height: Math.round(height),
    },
  );
}

/**
 * 读取当前真实窗口边界后落库，避免 resize payload 只包含尺寸而漏掉左上方向缩放的位置变化。
 */
async function persistCurrentWindowBounds(): Promise<void> {
  const [position, size, scaleFactor] = await Promise.all([
    currentWindow.outerPosition(),
    currentWindow.outerSize(),
    currentWindow.scaleFactor(),
  ]);
  const logicalPosition = new PhysicalPosition(position.x, position.y).toLogical(scaleFactor);

  await persistWindowBounds(
    logicalPosition.x,
    logicalPosition.y,
    size.width / scaleFactor,
    size.height / scaleFactor,
  );
}

/**
 * 缩放事件只安排最终保存，不在拖动过程中写 SQLite。
 */
function scheduleResizePersist(): void {
  clearResizePersistTimer();
  resizePersistTimer = window.setTimeout(() => {
    resizePersistTimer = null;
    finishResizePersist();
  }, RESIZE_PERSIST_SETTLE_MS);
}

/**
 * 缩放结束后保存最终边界，并同步给设置页与其他 Box 窗口。
 */
function finishResizePersist(): void {
  clearResizeReleaseEvents();
  clearResizePersistTimer();
  void persistCurrentWindowBounds();
}

/**
 * 监听缩放释放事件；系统原生拖拽吞掉释放事件时，resize 静止兜底仍会保存。
 */
function bindResizeReleaseEvents(): void {
  clearResizeReleaseEvents();

  window.addEventListener("mouseup", finishResizePersist, { capture: true, once: true });
  window.addEventListener("pointerup", finishResizePersist, { capture: true, once: true });
  document.addEventListener("mouseup", finishResizePersist, { capture: true, once: true });
  document.addEventListener("pointerup", finishResizePersist, { capture: true, once: true });
  resizeReleaseCleanup = () => {
    window.removeEventListener("mouseup", finishResizePersist, { capture: true });
    window.removeEventListener("pointerup", finishResizePersist, { capture: true });
    document.removeEventListener("mouseup", finishResizePersist, { capture: true });
    document.removeEventListener("pointerup", finishResizePersist, { capture: true });
  };
}

/**
 * 清理缩放释放监听，避免重复缩放时多次写入最终边界。
 */
function clearResizeReleaseEvents(): void {
  resizeReleaseCleanup?.();
  resizeReleaseCleanup = null;
}

/**
 * 清理缩放兜底计时器，窗口销毁或已经松手保存时不再重复落库。
 */
function clearResizePersistTimer(): void {
  if (!resizePersistTimer) {
    return;
  }

  window.clearTimeout(resizePersistTimer);
  resizePersistTimer = null;
}

/**
 * 同时清理缩放相关的监听和计时器，用于窗口卸载时释放异步回调。
 */
function clearResizePersistState(): void {
  clearResizeReleaseEvents();
  clearResizePersistTimer();
}

/**
 * 分辨率或 DPI 变化后，把 Box 拉回当前显示器工作区，避免窗口跑到屏幕外。
 */
async function ensureWindowInsideMonitor(): Promise<void> {
  if (!box.value || isApplyingWindowPosition) {
    return;
  }

  const monitor = (await currentMonitor()) ?? (await primaryMonitor());
  if (!monitor) {
    return;
  }

  const position = await currentWindow.outerPosition();
  const size = await currentWindow.outerSize();
  const workX = monitor.workArea.position.x;
  const workY = monitor.workArea.position.y;
  const maxX = workX + monitor.workArea.size.width - size.width;
  const maxY = workY + monitor.workArea.size.height - size.height;
  const nextX = Math.min(Math.max(position.x, workX), Math.max(maxX, workX));
  const nextY = Math.min(Math.max(position.y, workY), Math.max(maxY, workY));

  if (nextX === position.x && nextY === position.y) {
    return;
  }

  await applyWindowPhysicalPosition(nextX, nextY);
}

/**
 * 手写拖动从全局鼠标坐标开始，避免 Tauri 原生拖动在松手时回写旧位置。
 */
async function startManualDragging(event: MouseEvent): Promise<void> {
  if (!box.value || manualDragState) {
    return;
  }

  const [windowPosition, windowSize, cursor, scaleFactor, monitor] = await Promise.all([
    currentWindow.outerPosition(),
    currentWindow.outerSize(),
    cursorPosition(),
    currentWindow.scaleFactor(),
    currentMonitor(),
  ]);
  const activeMonitor = monitor ?? (await primaryMonitor());

  manualDragState = {
    cursorStartX: cursor.x,
    cursorStartY: cursor.y,
    height: windowSize.height,
    lastPosition: { x: windowPosition.x, y: windowPosition.y },
    offsetX: cursor.x - windowPosition.x,
    offsetY: cursor.y - windowPosition.y,
    scaleFactor,
    screenStartX: event.screenX,
    screenStartY: event.screenY,
    width: windowSize.width,
    workArea: activeMonitor
      ? {
          height: activeMonitor.workArea.size.height,
          width: activeMonitor.workArea.size.width,
          x: activeMonitor.workArea.position.x,
          y: activeMonitor.workArea.position.y,
        }
      : undefined,
  };

  bindManualDragReleaseEvents();
}

/**
 * 鼠标释放时停止拖动循环，并把最终物理坐标转换成逻辑坐标写入数据库。
 */
function stopManualDragging(shouldPersist: boolean): void {
  const dragState = manualDragState;

  manualDragState = null;
  manualDragCleanup?.();
  manualDragCleanup = null;

  if (shouldPersist && dragState) {
    void persistManualDragPosition(dragState.lastPosition);
  }
}

/**
 * 松手时用最后一次计算出的吸附坐标落库，移动过程中只改变窗口位置不写 SQLite。
 */
async function persistManualDragPosition(position: PhysicalWindowPoint): Promise<void> {
  await applyWindowPhysicalPosition(position.x, position.y, false);
  await persistWindowPositionFromPhysical(position.x, position.y);
}

/**
 * 释放监听同时挂在 window 和 document，确保窗口跟随鼠标移动时仍能收到 mouseup。
 */
function bindManualDragReleaseEvents(): void {
  manualDragCleanup?.();

  const stopDragging = (): void => {
    stopManualDragging(true);
  };
  const updateDragging = (event: MouseEvent): void => {
    updateManualDragCursor(event);
  };

  window.addEventListener("mousemove", updateDragging, { capture: true });
  window.addEventListener("mouseup", stopDragging, { capture: true, once: true });
  window.addEventListener("pointerup", stopDragging, { capture: true, once: true });
  document.addEventListener("mousemove", updateDragging, { capture: true });
  document.addEventListener("mouseup", stopDragging, { capture: true, once: true });
  document.addEventListener("pointerup", stopDragging, { capture: true, once: true });
  manualDragCleanup = () => {
    window.removeEventListener("mousemove", updateDragging, { capture: true });
    window.removeEventListener("mouseup", stopDragging, { capture: true });
    window.removeEventListener("pointerup", stopDragging, { capture: true });
    document.removeEventListener("mousemove", updateDragging, { capture: true });
    document.removeEventListener("mouseup", stopDragging, { capture: true });
    document.removeEventListener("pointerup", stopDragging, { capture: true });
  };
}

/**
 * 鼠标移动时按用户给出的 DOM 示例实时计算位置，只移动窗口不持久化数据库。
 */
function updateManualDragCursor(event: MouseEvent): void {
  const dragState = manualDragState;
  if (!dragState) {
    return;
  }

  const cursor = {
    x: dragState.cursorStartX + (event.screenX - dragState.screenStartX) * dragState.scaleFactor,
    y: dragState.cursorStartY + (event.screenY - dragState.screenStartY) * dragState.scaleFactor,
  };
  const rawPosition = {
    x: cursor.x - dragState.offsetX,
    y: cursor.y - dragState.offsetY,
  };
  const nextPosition = resolveManualDragPosition(rawPosition, dragState);

  dragState.lastPosition = nextPosition;
  void applyWindowPhysicalPosition(nextPosition.x, nextPosition.y, false);
}

/**
 * 程序主动移动窗口时统一加锁，避免 setPosition 自己触发的 onMoved 被误判成外部移动。
 */
async function applyWindowPhysicalPosition(
  x: number,
  y: number,
  shouldPersist = true,
): Promise<void> {
  const applyVersion = windowPositionApplyVersion + 1;

  windowPositionApplyVersion = applyVersion;
  isApplyingWindowPosition = true;
  try {
    await currentWindow.setPosition(new PhysicalPosition(x, y));
    if (shouldPersist) {
      await persistWindowPositionFromPhysical(x, y);
    }
  } finally {
    window.setTimeout(() => {
      if (windowPositionApplyVersion === applyVersion) {
        isApplyingWindowPosition = false;
      }
    }, WINDOW_POSITION_APPLY_LOCK_MS);
  }
}

/**
 * 拖动吸附实时参考其他 Box 的相邻边和屏幕工作区边缘，不做延迟二次定位。
 */
function resolveManualDragPosition(
  rawPosition: PhysicalWindowPoint,
  dragState: ManualDragState,
): PhysicalWindowPoint {
  const threshold = desktopStore.settings.snapThreshold;
  let nextX = rawPosition.x;
  let nextY = rawPosition.y;

  if (desktopStore.settings.snapToEdges) {
    for (const otherBox of desktopStore.boxes) {
      if (otherBox.id === props.boxId) {
        continue;
      }

      const otherX = Math.round(otherBox.x * dragState.scaleFactor);
      const otherY = Math.round(otherBox.y * dragState.scaleFactor);
      const otherWidth = Math.round(otherBox.width * dragState.scaleFactor);
      const otherHeight = Math.round(otherBox.height * dragState.scaleFactor);
      const otherRight = otherX + otherWidth;
      const otherBottom = otherY + otherHeight;

      if (Math.abs(nextX - otherRight) < threshold) {
        nextX = otherRight;
      }
      if (Math.abs(nextX + dragState.width - otherX) < threshold) {
        nextX = otherX - dragState.width;
      }
      if (Math.abs(nextY - otherBottom) < threshold) {
        nextY = otherBottom;
      }
      if (Math.abs(nextY + dragState.height - otherY) < threshold) {
        nextY = otherY - dragState.height;
      }
    }

    if (dragState.workArea) {
      const workX = dragState.workArea.x;
      const workY = dragState.workArea.y;
      const workRight = workX + dragState.workArea.width;
      const workBottom = workY + dragState.workArea.height;

      if (Math.abs(nextX - workX) < threshold) {
        nextX = workX;
      }
      if (Math.abs(nextX + dragState.width - workRight) < threshold) {
        nextX = workRight - dragState.width;
      }
      if (Math.abs(nextY - workY) < threshold) {
        nextY = workY;
      }
      if (Math.abs(nextY + dragState.height - workBottom) < threshold) {
        nextY = workBottom - dragState.height;
      }
    }
  }

  return {
    x: nextX,
    y: nextY,
  };
}
</script>

<template>
  <main class="h-screen w-screen overflow-hidden bg-transparent p-0">
    <article
      v-if="box"
      class="relative flex h-full w-full flex-col overflow-hidden rounded-[8px] border border-white/45 bg-white/10 text-slate-950 dark:border-[#3b3f49] dark:bg-[#101114]/30 dark:text-white"
      @click="closeContextMenu"
      @dragover.prevent
      @drop="desktopStore.handleItemDrop($event, box.id)"
    >
      <span
        v-for="handle in resizeHandles"
        :key="handle.direction"
        class="absolute z-40"
        :class="handle.className"
        @mousedown.stop.prevent="startResizing(handle.direction, $event)"
      />

      <header
        class="relative flex h-10 shrink-0 select-none items-center justify-center border-b border-white/35 bg-white/10 px-3 dark:border-[#30333c] dark:bg-[#1a1c22]/40"
        @mousedown.left="startDragging"
        @dblclick.stop.prevent
      >
        <input
          v-if="isEditingTitle"
          ref="titleInputRef"
          v-model="titleDraft"
          aria-label="编辑 Box 名称"
          class="h-7 max-w-[68%] rounded-[6px] border border-[#d7dae2] bg-white px-2 text-center text-[13px] font-semibold text-slate-900 outline-none ring-0 transition focus:border-[#ff5c5c] focus:shadow-[0_0_0_3px_rgba(255,92,92,0.16)] dark:border-[#3a3d46] dark:bg-[#202228] dark:text-white"
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
          class="max-w-[68%] truncate text-center text-[13px] font-semibold text-slate-900 dark:text-white"
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
          @click.stop="openContextMenu"
          @mousedown.stop
        >
          <MoreHorizontal :size="18" />
        </button>
      </header>

      <div class="grid min-h-0 flex-1 auto-rows-[82px] grid-cols-[repeat(auto-fill,minmax(72px,1fr))] gap-1 overflow-auto p-2.5">
        <DesktopIcon
          v-for="item in boxItems"
          :key="item.path"
          :item="item"
          :show-label="desktopStore.settings.showItemLabels"
        />

        <div
          v-if="boxItems.length === 0"
          class="col-span-full grid min-h-[120px] place-items-center px-5 text-center"
        >
          <div>
            <strong class="text-[13px] font-semibold text-slate-900 dark:text-white">这个 Box 还是空的</strong>
            <p class="mt-1 text-[12px] leading-5 text-slate-600 dark:text-slate-300">把桌面文件拖进来就能开始整理。</p>
          </div>
        </div>
      </div>

      <nav
        v-if="contextMenu.open"
        class="fixed z-50 grid min-w-[176px] overflow-hidden rounded-[10px] border border-[#d9dce3] bg-[#fbfbfd] p-1 text-slate-800 shadow-[0_18px_45px_rgba(15,23,42,0.24)] dark:border-[#30333c] dark:bg-[#202228] dark:text-slate-100"
        :style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }"
        @click.stop
      >
        <button
          class="flex h-8 items-center rounded-[7px] px-3 text-left text-[12px] transition-colors hover:bg-[#eceef3] dark:hover:bg-[#2b2e37]"
          type="button"
          @click="openSettingsFromMenu"
        >
          <Settings class="mr-2 text-slate-500 dark:text-slate-400" :size="14" />
          打开设置
        </button>
        <button
          class="flex h-8 items-center rounded-[7px] px-3 text-left text-[12px] transition-colors hover:bg-[#eceef3] dark:hover:bg-[#2b2e37]"
          type="button"
          @click="refreshDesktopFromMenu"
        >
          <RefreshCw class="mr-2 text-slate-500 dark:text-slate-400" :size="14" />
          刷新桌面
        </button>
        <span class="my-1 h-px bg-[#e4e6eb] dark:bg-[#30333c]" />
        <button
          class="flex h-8 items-center rounded-[7px] px-3 text-left text-[12px] text-red-600 transition-colors hover:bg-[#fff0f0] dark:text-red-400 dark:hover:bg-[#3a2528]"
          type="button"
          @click="deleteCurrentBox"
        >
          <Trash2 class="mr-2" :size="14" />
          删除 Box
        </button>
      </nav>
    </article>
  </main>
</template>
