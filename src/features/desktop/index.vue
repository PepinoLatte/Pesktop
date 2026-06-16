<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import type { CSSProperties } from "vue";
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
import { DESKTOP_ICON_VIEW } from "./config/desktopIcon";
import { useDesktopStore } from "./store/desktopStore";
import SegmentedControl from "../../shared/components/SegmentedControl.vue";
import type {
  DesktopBoxItemDropPlacement,
  DesktopBoxTitlePosition,
  DesktopItem,
} from "../../shared/types/desktop";
import { showNativeItemContextMenu } from "../../shared/api/desktop";
import { openSettingsWindow } from "../../shared/window/boxWindows";
import {
  BOX_CONTEXT_MENU_LAYOUT,
  BOX_WINDOW_INTERACTION_TIMING,
} from "../../shared/config/desktopLayout";

const props = defineProps<{
  boxId: string;
}>();

const desktopStore = useDesktopStore();
const currentWindow = getCurrentWindow();
const unlistenFns: UnlistenFn[] = [];
/**
 * 程序主动定位后的短锁用于过滤 setPosition 自己触发的移动事件。
 */
const WINDOW_POSITION_APPLY_LOCK_MS = BOX_WINDOW_INTERACTION_TIMING.positionApplyLockMs;
/**
 * 系统缩放拖拽可能吞掉 WebView 的 mouseup，短暂静止后保存最终边界作为兜底。
 */
const RESIZE_PERSIST_SETTLE_MS = BOX_WINDOW_INTERACTION_TIMING.resizePersistSettleMs;

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

/**
 * Box 内空隙拖拽命中结果，父级用它在最近图标左右两侧绘制插入线。
 */
interface BoxGridDragInsertTarget {
  path: string;
  placement: DesktopBoxItemDropPlacement;
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

/**
 * Box 菜单里的标题位置使用紧凑分段控件，既保留动画反馈，也避免菜单出现两行重复按钮。
 */
const BOX_TITLE_POSITION_OPTIONS: Array<{
  label: string;
  value: DesktopBoxTitlePosition;
}> = [
  { label: "上方", value: "top" },
  { label: "下方", value: "bottom" },
];
const BOX_TITLE_POSITION_OPTION_WIDTH = 74;
const contextMenu = ref({ open: false, x: 0, y: 0 });
const isEditingTitle = ref(false);
const draggingBoxItemPath = ref<string | null>(null);
const dragInsertTargetPath = ref<string | null>(null);
const dragInsertPlacement = ref<DesktopBoxItemDropPlacement | null>(null);
const titleDraft = ref("");
const titleInputRef = ref<HTMLInputElement | null>(null);
const box = computed(() => desktopStore.boxes.find((item) => item.id === props.boxId));
const boxItems = computed(() => (box.value ? desktopStore.getBoxItems(box.value.id) : []));
const boxGridRef = ref<HTMLElement | null>(null);
const boxItemWidth = computed(() =>
  Math.max(
    desktopStore.settings.boxFilenameWidth,
    desktopStore.settings.boxIconSize + DESKTOP_ICON_VIEW.itemInlinePadding * 2,
  ),
);
const boxSurfaceStyle = computed(
  () =>
    ({
      "--dasktop-box-background-opacity": `${desktopStore.settings.boxBackgroundOpacity / 100}`,
      "--dasktop-box-radius": `${desktopStore.settings.boxCornerRadius}px`,
      borderRadius: "var(--dasktop-box-radius)",
    }) as CSSProperties,
);
const boxGridStyle = computed(
  () =>
    ({
      columnGap: `${desktopStore.settings.boxIconGapX}px`,
      gridTemplateColumns: `repeat(auto-fill, minmax(min(100%, ${boxItemWidth.value}px), ${boxItemWidth.value}px))`,
      rowGap: `${desktopStore.settings.boxIconGapY}px`,
    }) as CSSProperties,
);

onMounted(async () => {
  await desktopStore.initialize();
  await syncWindowBoundsFromStore();

  unlistenFns.push(
    await currentWindow.onMoved(async ({ payload }) => {
      await handleWindowMoved(payload.x, payload.y);
    }),
  );

  unlistenFns.push(
    await currentWindow.onFocusChanged(({ payload }) => {
      if (!payload) {
        handleWindowBlur();
      }
    }),
  );
  window.addEventListener("blur", handleWindowBlur);

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
  window.removeEventListener("blur", handleWindowBlur);
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
 * 窗口失焦时关闭菜单；如果正拖着 Box 图标离开窗口，则按拖出 Box 删除映射处理。
 */
function handleWindowBlur(): void {
  closeContextMenu();
  void removeDraggingBoxItemAfterWindowBlur();
}

/**
 * pointer 拖出 WebView 后可能收不到 pointerup，用窗口失焦作为删除映射的兜底信号。
 */
async function removeDraggingBoxItemAfterWindowBlur(): Promise<void> {
  const itemPath = draggingBoxItemPath.value;
  if (!box.value || !itemPath) {
    return;
  }

  draggingBoxItemPath.value = null;
  clearBoxItemDragIndicator();
  await desktopStore.removeItemFromBox(box.value.id, itemPath);
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
 * 保存标题时允许空文本，设置页会用兜底名称识别该 Box，不强迫用户显示标题。
 */
async function commitTitleEditing(): Promise<void> {
  if (!box.value || !isEditingTitle.value) {
    return;
  }

  const nextTitle = titleDraft.value.trim();
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
 * 更多菜单相对按钮向左下方错开，并限制在当前窗口内，避免贴着点击点或被窗口边缘裁切。
 */
function openContextMenu(event: MouseEvent): void {
  if (contextMenu.value.open) {
    return;
  }

  const { height, triggerGap, viewportPadding, width } = BOX_CONTEXT_MENU_LAYOUT;
  const preferredX = event.clientX - width + triggerGap * 2;
  const preferredY =
    event.clientY + triggerGap + height > window.innerHeight - viewportPadding
      ? event.clientY - height - triggerGap
      : event.clientY + triggerGap;

  contextMenu.value = {
    open: true,
    x: Math.min(Math.max(preferredX, viewportPadding), window.innerWidth - width - viewportPadding),
    y: Math.min(
      Math.max(preferredY, viewportPadding),
      window.innerHeight - height - viewportPadding,
    ),
  };
}

/**
 * Box 图标 pointer 拖拽开始时关闭菜单，并记录当前正在排序的项目路径。
 */
function startBoxItemPointerDrag(itemPath: string): void {
  draggingBoxItemPath.value = itemPath;
  clearBoxItemDragIndicator();
  closeContextMenu();
}

/**
 * pointer 拖过 Box 内容区时展示插入竖线，让用户在释放前知道排序位置。
 */
function moveBoxItemPointerDrag(event: PointerEvent, itemPath: string): void {
  if (draggingBoxItemPath.value !== itemPath || !boxGridRef.value) {
    clearBoxItemDragIndicator();
    return;
  }

  const insertTarget = resolveBoxGridDragInsertTarget(
    event.clientX,
    event.clientY,
    boxGridRef.value,
  );
  if (!insertTarget) {
    clearBoxItemDragIndicator();
    return;
  }

  dragInsertTargetPath.value = insertTarget.path;
  dragInsertPlacement.value = insertTarget.placement;
}

/**
 * 外部文件拖入 Box 时声明当前区域可接收文件；内部排序已改用 pointer 拖拽。
 */
function handleBoxGridDragOver(event: DragEvent): void {
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = "copy";
  }
}

/**
 * DOM drop 只处理外部文件路径；Box 内部排序由 pointer up 提交，避免浏览器 DnD 禁用光标。
 */
async function handleBoxGridDrop(event: DragEvent): Promise<void> {
  if (!box.value) {
    return;
  }

  await desktopStore.handleItemDrop(event, box.value.id);
}

/**
 * pointer 拖拽结束时提交排序；如果释放点已离开 Box 窗口，则删除映射让图标回到桌面。
 */
async function finishBoxItemPointerDrag(event: PointerEvent, itemPath: string): Promise<void> {
  if (!box.value || draggingBoxItemPath.value !== itemPath) {
    draggingBoxItemPath.value = null;
    clearBoxItemDragIndicator();
    return;
  }

  const shouldRemove = !isPointerInsideCurrentWindow(event);
  const insertTargetPath = dragInsertTargetPath.value;
  const insertPlacement = dragInsertPlacement.value;

  draggingBoxItemPath.value = null;
  clearBoxItemDragIndicator();
  if (shouldRemove) {
    await desktopStore.removeItemFromBox(box.value.id, itemPath);
    return;
  }

  if (insertTargetPath && insertPlacement) {
    await desktopStore.reorderBoxItem(box.value.id, itemPath, insertTargetPath, insertPlacement);
    return;
  }

  await desktopStore.moveBoxItemToEnd(box.value.id, itemPath);
}

/**
 * 清理排序提示线，避免拖拽离开窗口或落到空白区后保留旧位置。
 */
function clearBoxItemDragIndicator(): void {
  dragInsertTargetPath.value = null;
  dragInsertPlacement.value = null;
}

/**
 * 指针位于图标间隙时，根据同一行最近的图标推导插入方向，避免排序提示线在空隙中消失。
 */
function resolveBoxGridDragInsertTarget(
  clientX: number,
  clientY: number,
  container: HTMLElement,
): BoxGridDragInsertTarget | null {
  const iconElements = Array.from(
    container.querySelectorAll<HTMLElement>("[data-box-item-path]"),
  );
  let nearestTarget: (BoxGridDragInsertTarget & { distance: number }) | null = null;

  for (const iconElement of iconElements) {
    const targetPath = iconElement.dataset.boxItemPath;
    if (!targetPath || targetPath === draggingBoxItemPath.value) {
      continue;
    }

    const rect = iconElement.getBoundingClientRect();
    const verticalDistance =
      clientY < rect.top ? rect.top - clientY : Math.max(clientY - rect.bottom, 0);
    if (verticalDistance > DESKTOP_ICON_VIEW.dragInsertRowTolerance) {
      continue;
    }

    const centerX = rect.left + rect.width / 2;
    const distance = Math.abs(clientX - centerX) + verticalDistance * 3;
    if (nearestTarget && nearestTarget.distance <= distance) {
      continue;
    }

    nearestTarget = {
      distance,
      path: targetPath,
      placement: clientX > centerX ? "after" : "before",
    };
  }

  return nearestTarget
    ? {
        path: nearestTarget.path,
        placement: nearestTarget.placement,
      }
    : null;
}

/**
 * 浏览器拖拽事件的 client 坐标可以判断是否仍在当前透明 Box 窗口内。
 */
function isPointerInsideCurrentWindow(event: PointerEvent): boolean {
  return (
    event.clientX > 0 &&
    event.clientY > 0 &&
    event.clientX < window.innerWidth &&
    event.clientY < window.innerHeight
  );
}

/**
 * Windows Shell 右键菜单由后端直接接管，前端只负责传递屏幕坐标和目标路径。
 */
function openNativeItemContextMenu(event: MouseEvent, item: DesktopItem): void {
  closeContextMenu();
  void showNativeItemContextMenu(item.path, event.screenX, event.screenY).catch((error) => {
    desktopStore.lastError = error instanceof Error ? error.message : String(error);
  });
}

/**
 * Box 标题位置从更多菜单直接切换，适合用户在整理时即时调整窗口布局。
 */
async function updateTitlePositionFromMenu(position: DesktopBoxTitlePosition): Promise<void> {
  if (!box.value) {
    return;
  }

  await desktopStore.updateBoxTitlePosition(box.value.id, position);
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
  <main class="h-screen w-screen overflow-hidden bg-transparent p-0" @click="closeContextMenu">
    <article
      v-if="box"
      class="dasktop-box-surface relative flex h-full w-full flex-col overflow-hidden text-slate-950 shadow-[0_10px_28px_rgba(15,23,42,0.10)] dark:text-white dark:shadow-[0_10px_28px_rgba(0,0,0,0.24)]"
      :style="boxSurfaceStyle"
      @dragover.prevent="handleBoxGridDragOver"
      @drop.prevent="handleBoxGridDrop"
    >
      <span
        v-for="handle in resizeHandles"
        :key="handle.direction"
        class="absolute z-40"
        :class="handle.className"
        @mousedown.stop.prevent="startResizing(handle.direction, $event)"
      />

      <header
        class="relative flex h-10 shrink-0 select-none items-center justify-center px-3"
        :class="box.titlePosition === 'bottom' ? 'order-2' : 'order-0'"
        @mousedown.left="startDragging"
        @dblclick.stop.prevent
      >
        <input
          v-if="isEditingTitle"
          ref="titleInputRef"
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
          @click.stop="openContextMenu"
          @mousedown.stop
        >
          <MoreHorizontal :size="18" />
        </button>
      </header>

      <div
        ref="boxGridRef"
        class="dasktop-scrollarea dasktop-box-scrollarea grid min-h-0 flex-1 overflow-auto p-2.5"
        :class="
          boxItems.length === 0
            ? 'content-center place-items-center justify-center'
            : 'content-start items-start justify-start'
        "
        :style="boxGridStyle"
      >
        <DesktopIcon
          v-for="item in boxItems"
          :key="item.path"
          :double-click-open="desktopStore.settings.doubleClickOpenItems"
          :drag-insert-position="dragInsertTargetPath === item.path ? dragInsertPlacement : null"
          :icon-size="desktopStore.settings.boxIconSize"
          :item="item"
          :label-text-size="desktopStore.settings.boxLabelTextSize"
          :label-width="desktopStore.settings.boxFilenameWidth"
          :name-display-mode="desktopStore.settings.nameDisplayMode"
          :radius-size="desktopStore.settings.boxCornerRadius"
          :show-label="desktopStore.settings.showItemLabels"
          :show-shortcut-arrow="desktopStore.settings.showShortcutArrow"
          @box-pointer-drag-end="finishBoxItemPointerDrag"
          @box-pointer-drag-move="moveBoxItemPointerDrag"
          @box-pointer-drag-start="startBoxItemPointerDrag"
          @native-context-menu="openNativeItemContextMenu"
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

    <nav
      v-if="box && contextMenu.open"
      class="dasktop-box-menu fixed z-[100] grid min-w-[176px] overflow-hidden rounded-[10px] border border-[#d9dce3] bg-[#fbfbfd] p-1 text-slate-800 shadow-[0_18px_45px_rgba(15,23,42,0.24)] dark:border-[#30333c] dark:bg-[#202228] dark:text-slate-100"
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
      <div class="grid gap-2 px-2 py-1.5">
        <span class="text-[11px] font-medium text-slate-500 dark:text-slate-400">标题位置</span>
        <SegmentedControl
          :model-value="box.titlePosition"
          :option-width-px="BOX_TITLE_POSITION_OPTION_WIDTH"
          :options="BOX_TITLE_POSITION_OPTIONS"
          @change="updateTitlePositionFromMenu"
        />
      </div>
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
  </main>
</template>
