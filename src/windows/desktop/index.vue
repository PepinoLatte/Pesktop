<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import type { CSSProperties } from "vue";
import { MoreHorizontal } from "@lucide/vue";
import { animate } from "motion";
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
import { useBoxContextMenu } from "./composables/useBoxContextMenu";
import { useBoxTitleEditing } from "./composables/useBoxTitleEditing";
import {
  resolveBoxGridDragInsertTarget,
  type BoxGridDragInsertTarget,
} from "./utils/dragGeometry";
import { useDesktopStore } from "@/entities/desktopBox/store";
import type { DesktopItem } from "@/entities/desktopItem/types";
import {
  getDesktopItemsByPaths,
  isPrimaryMouseButtonPressed,
  showNativeItemContextMenu,
} from "@/entities/desktopItem/api";
import {
  listenBoxItemDrag,
  listenBoxItemDragAccepted,
  notifyBoxItemDrag,
  notifyBoxItemDragAccepted,
  type BoxItemDragPayload,
  type BoxItemDragPreviewOptions,
} from "@/shared/ipc/boxItemDrag";
import { openDragPreviewWindow } from "@/windows/dragPreview/lifecycle";
import { preloadBoxContextMenuWindow } from "@/entities/desktopBox/windows";
import { listenBoxContextMenuState } from "@/shared/ipc/boxContextMenu";
import {
  BOX_COLLAPSE_INTERACTION,
  BOX_ITEM_DRAG_INTERACTION,
  BOX_TITLE_OPACITY,
  BOX_TITLE_VISIBILITY,
  BOX_WINDOW_INTERACTION_TIMING,
} from "@/entities/desktopBox/layout";

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
 * 收缩动画最终写回原生窗口时同时包含位置和尺寸，标题在下方时需要用它保持标题视觉锚点。
 */
interface LogicalWindowFrame {
  height: number;
  width: number;
  x: number;
  y: number;
}

/**
 * 跨窗口拖拽会话保存在来源 Box 中，用于轮询全局鼠标并在未被接收时执行拖出删除映射。
 */
interface BoxItemGlobalDragState {
  accepted: boolean;
  finishing: boolean;
  item: DesktopItem;
  preview: BoxItemDragPreviewOptions;
  sessionId: string;
  sourceBoxId: string;
}

/**
 * 外部桌面文件拖入 Box 时没有来源 Box，会话只负责拖影和插入线接力，不参与拖出删除。
 */
interface ExternalFileDragState {
  item: DesktopItem;
  preview: BoxItemDragPreviewOptions;
  sessionId: string;
  usesScreenPosition: boolean;
}

/**
 * 屏幕物理坐标换算到当前 Box WebView 的结果，目标窗口据此判断是否命中自身。
 */
interface BoxItemDragLocalPoint {
  inside: boolean;
  x: number;
  y: number;
}

/**
 * 外部 Windows 拖拽取消时可延迟清理 hover，保证鼠标仍按下时 Box 不会收缩导致原生拖放链路抖动。
 */
interface ExternalFileDragCancelOptions {
  deferHoverClearUntilRelease?: boolean;
}

let isApplyingWindowPosition = false;
let isApplyingCollapseWindowSize = false;
let windowPositionApplyVersion = 0;
/**
 * 收缩动画可能被快速 hover 切换打断，版本号用于阻止旧动画的异步收尾覆盖新状态。
 */
let collapseAnimationVersion = 0;
let collapseAnimationTween: ReturnType<typeof animate> | null = null;
let boxOpacityTween: ReturnType<typeof animate> | null = null;
let collapsePreviewCloseTimer: ReturnType<typeof window.setTimeout> | null = null;
let collapseSizeApplyLockTimer: ReturnType<typeof window.setTimeout> | null = null;
let windowResizableApplyVersion = 0;
let manualDragState: ManualDragState | null = null;
let manualDragCleanup: (() => void) | null = null;
let resizeReleaseCleanup: (() => void) | null = null;
let resizeInteractionReleaseProbeTimer: ReturnType<typeof window.setInterval> | null = null;
let resizePersistTimer: ReturnType<typeof window.setTimeout> | null = null;
let resizeStartedAt = 0;
let resizeReleasedStableTicks = 0;
let boxItemGlobalDragState: BoxItemGlobalDragState | null = null;
let boxItemDragPollTimer: ReturnType<typeof window.setInterval> | null = null;
let isBoxItemDragPollTickPending = false;
let externalFileDragState: ExternalFileDragState | null = null;
let externalFileDragReleaseProbeTimer: ReturnType<typeof window.setInterval> | null = null;
let externalFileDragReleaseProbeStartedAt = 0;
let externalFileDragReleaseSessionId = "";
const acceptedBoxItemDragSessions = new Set<string>();

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
  { direction: "North", className: "left-4 right-4 top-0 h-2 cursor-ns-resize" },
  { direction: "South", className: "bottom-0 left-4 right-4 h-2 cursor-ns-resize" },
  { direction: "West", className: "bottom-4 left-0 top-4 w-2 cursor-ew-resize" },
  { direction: "East", className: "bottom-4 right-0 top-4 w-2 cursor-ew-resize" },
  { direction: "NorthWest", className: "left-0 top-0 size-4 cursor-nwse-resize" },
  { direction: "NorthEast", className: "right-0 top-0 size-4 cursor-nesw-resize" },
  { direction: "SouthWest", className: "bottom-0 left-0 size-4 cursor-nesw-resize" },
  { direction: "SouthEast", className: "bottom-0 right-0 size-4 cursor-nwse-resize" },
];

const isCollapsedPreviewOpen = ref(false);
const isBoxHovered = ref(false);
const isDragHoveringBox = ref(false);
const isManualDraggingBox = ref(false);
const isResizeHandleHovered = ref(false);
const isResizingBox = ref(false);
const isTitleHovered = ref(false);
const isCollapseAnimating = ref(false);
const boxSurfaceVisualHeight = ref<number | null>(null);
const draggingBoxItemPath = ref<string | null>(null);
const draggingBoxItemSessionId = ref<string | null>(null);
const dragInsertLineStyle = ref<CSSProperties | null>(null);
const boxSurfaceRef = ref<HTMLElement | null>(null);
const box = computed(() => desktopStore.boxes.find((item) => item.id === props.boxId));
const boxItems = computed(() => (box.value ? desktopStore.getBoxItems(box.value.id) : []));
const boxGridRef = ref<HTMLElement | null>(null);
const isBoxItemDragActive = computed(() => Boolean(draggingBoxItemPath.value));
const isBoxCollapsedToTitle = computed(() =>
  Boolean(box.value?.collapsed && !isCollapsedPreviewOpen.value),
);
const collapsedWindowHeight = computed(() => BOX_TITLE_VISIBILITY.expandedHeight);
const boxIdleOpacity = computed(() =>
  isBoxHovered.value ||
  isDragHoveringBox.value ||
  isContextMenuOpen.value ||
  isEditingTitle.value ||
  isManualDraggingBox.value ||
  isResizeHandleHovered.value ||
  isResizingBox.value ||
  isCollapsedPreviewOpen.value
    ? 1
    : (box.value?.titleOpacity ?? BOX_TITLE_OPACITY.max) / 100,
);
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
const boxSurfaceStyle = computed(
  () =>
    ({
      "--dasktop-box-background-opacity": `${desktopStore.settings.boxBackgroundOpacity / 100}`,
      "--dasktop-box-radius": `${desktopStore.settings.boxCornerRadius}px`,
      borderRadius: "var(--dasktop-box-radius)",
      clipPath: "inset(0 round var(--dasktop-box-radius))",
      height: boxSurfaceVisualHeight.value === null ? "100%" : `${boxSurfaceVisualHeight.value}px`,
      position: "relative",
    }) as CSSProperties,
);
const boxTitleAreaStyle = computed(
  () =>
    ({
      height: `${BOX_TITLE_VISIBILITY.expandedHeight}px`,
    }) as CSSProperties,
);
const boxBodyStyle = computed(
  () => {
    const isBottomTitle = box.value?.titlePosition === "bottom";
    const animatedHeight = boxSurfaceVisualHeight.value;
    const isCollapsingBottomTitle =
      isBottomTitle && (isCollapseAnimating.value || isBoxCollapsedToTitle.value);
    const bodyHeight =
      !isCollapsingBottomTitle
        ? undefined
        : Math.max(
            (animatedHeight ?? collapsedWindowHeight.value) - BOX_TITLE_VISIBILITY.expandedHeight,
            0,
          );

    return {
      flex: isCollapsingBottomTitle ? "0 0 auto" : undefined,
      height: bodyHeight === undefined ? undefined : `${bodyHeight}px`,
      opacity: isBoxCollapsedToTitle.value ? "0" : "1",
      pointerEvents: isBoxCollapsedToTitle.value ? "none" : "auto",
      transform: isBoxCollapsedToTitle.value ? "translateY(-6px)" : "translateY(0)",
    } as CSSProperties;
  },
);
/**
 * 收缩窗口高度动画期间隐藏内部滚动条，避免 WebView 中间高度小于内容高度时闪出滚动条。
 */
const boxGridOverflowClass = computed(() =>
  isCollapseAnimating.value || isBoxCollapsedToTitle.value ? "overflow-hidden" : "overflow-auto",
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
  setLastError: (message) => {
    desktopStore.lastError = message;
  },
});
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
      acceptedBoxItemDragSessions.add(payload.sessionId);
      if (boxItemGlobalDragState?.sessionId === payload.sessionId) {
        boxItemGlobalDragState.accepted = true;
      }
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
 * 根据收缩展示状态调整真实窗口高度，避免透明空白窗口挡住桌面点击。
 */
async function applyCollapseWindowSize(shouldAnimate: boolean): Promise<void> {
  if (!box.value) {
    return;
  }

  cancelCollapseAnimationTween();
  collapseAnimationVersion += 1;
  const activeCollapseAnimationVersion = collapseAnimationVersion;

  const targetHeight = isBoxCollapsedToTitle.value ? collapsedWindowHeight.value : box.value.height;
  const targetWidth = box.value.width;
  const targetFrame = resolveCollapseWindowFrame(targetWidth, targetHeight);
  const animationMs = shouldAnimate ? desktopStore.getBoxCollapseAnimationMs() : 0;

  setCollapseSizeApplyLock(animationMs);

  if (!shouldAnimate) {
    boxSurfaceVisualHeight.value = null;
    await applyCollapseWindowFrame(targetFrame);
    return;
  }

  const [windowSize, scaleFactor] = await Promise.all([
    currentWindow.outerSize(),
    currentWindow.scaleFactor(),
  ]);
  const currentWindowHeight = windowSize.height / scaleFactor;
  const startHeight = boxSurfaceVisualHeight.value ?? currentWindowHeight;
  const heightDistance = targetHeight - startHeight;

  if (Math.abs(heightDistance) < 1) {
    boxSurfaceVisualHeight.value = null;
    await applyCollapseWindowFrame(targetFrame);
    return;
  }

  if (shouldReduceMotion()) {
    boxSurfaceVisualHeight.value = null;
    await applyCollapseWindowFrame(targetFrame);
    return;
  }

  isCollapseAnimating.value = true;
  boxSurfaceVisualHeight.value = startHeight;
  if (targetHeight > currentWindowHeight) {
    await applyCollapseWindowFrame(targetFrame);
  }

  const tweenState = {
    height: startHeight,
  };
  collapseAnimationTween = animate(
    tweenState,
    {
      height: targetHeight,
    },
    {
      duration: animationMs / 1000,
      ease: [0.22, 1, 0.36, 1],
      onComplete: () => {
        collapseAnimationTween = null;
        finishCollapseWindowResize(activeCollapseAnimationVersion, targetFrame);
      },
      onUpdate: () => {
        boxSurfaceVisualHeight.value = Math.round(tweenState.height);
      },
    },
  );
}

/**
 * motion 驱动 Box 闲置可见度，hover 进入时即使配置为 0 也能平滑恢复到完全可见。
 */
function animateBoxIdleOpacity(shouldAnimate: boolean): void {
  const surfaceElement = boxSurfaceRef.value;
  if (!surfaceElement) {
    return;
  }

  boxOpacityTween?.stop();
  boxOpacityTween = null;

  if (!shouldAnimate || shouldReduceMotion()) {
    surfaceElement.style.opacity = String(boxIdleOpacity.value);
    return;
  }

  boxOpacityTween = animate(
    surfaceElement,
    {
      opacity: boxIdleOpacity.value,
    },
    {
      delay: resolveBoxIdleOpacityAnimationDelay(),
      duration: 0.18,
      ease: [0.16, 1, 0.3, 1],
      onComplete: () => {
        boxOpacityTween = null;
      },
    },
  );
}

/**
 * 减少动态效果时直接应用终态，遵守系统辅助功能设置。
 */
function shouldReduceMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/**
 * 收缩触发闲置透明时延后淡出，让用户先感知 Box 收回动作，再看到透明度过渡。
 */
function resolveBoxIdleOpacityAnimationDelay(): number {
  if (boxIdleOpacity.value >= 1 || !isBoxCollapsedToTitle.value) {
    return 0;
  }

  return Math.min(desktopStore.getBoxCollapseAnimationMs() * 0.36, 140) / 1000;
}

/**
 * 收缩态统一保留完整 Box 顶部的标题高度，即使标题配置在下方也向上收缩，避免视觉方向反转。
 */
function resolveCollapseWindowFrame(width: number, height: number): LogicalWindowFrame {
  if (!box.value) {
    return {
      height,
      width,
      x: 0,
      y: 0,
    };
  }

  return {
    height,
    width,
    x: box.value.x,
    y: box.value.y,
  };
}

/**
 * 收缩和展开会主动调整窗口几何状态，需要加移动锁，防止 onMoved 把临时标题位置误写入数据库。
 */
async function applyCollapseWindowFrame(frame: LogicalWindowFrame): Promise<void> {
  const applyVersion = windowPositionApplyVersion + 1;

  windowPositionApplyVersion = applyVersion;
  isApplyingWindowPosition = true;
  try {
    await Promise.all([
      currentWindow.setPosition(new LogicalPosition(frame.x, frame.y)),
      currentWindow.setSize(new LogicalSize(frame.width, frame.height)),
    ]);
  } finally {
    window.setTimeout(() => {
      if (windowPositionApplyVersion === applyVersion) {
        isApplyingWindowPosition = false;
      }
    }, WINDOW_POSITION_APPLY_LOCK_MS);
  }
}

/**
 * 收缩动画完成后一次性同步真实窗口高度，避免动画过程中暴露 Windows 原生直角边界。
 */
function finishCollapseWindowResize(
  activeCollapseAnimationVersion: number,
  targetFrame: LogicalWindowFrame,
): void {
  boxSurfaceVisualHeight.value = targetFrame.height;
  void applyCollapseWindowFrame(targetFrame).finally(() => {
    if (activeCollapseAnimationVersion !== collapseAnimationVersion) {
      return;
    }

    boxSurfaceVisualHeight.value = null;
    isCollapseAnimating.value = false;
  });
}

/**
 * 收缩动画期间屏蔽 resize 落库，最终同步真实窗口高度时也不能覆盖用户保存的 Box 尺寸。
 */
function setCollapseSizeApplyLock(animationMs = 0): void {
  isApplyingCollapseWindowSize = true;
  if (collapseSizeApplyLockTimer) {
    window.clearTimeout(collapseSizeApplyLockTimer);
  }

  const lockMs = Math.max(
    BOX_COLLAPSE_INTERACTION.sizeApplyLockMs,
    animationMs + RESIZE_PERSIST_SETTLE_MS + 40,
  );
  collapseSizeApplyLockTimer = window.setTimeout(() => {
    isApplyingCollapseWindowSize = false;
    collapseSizeApplyLockTimer = null;
  }, lockMs);
}

/**
 * 取消未完成的 motion 收缩动画，用于快速 hover 切换或窗口卸载。
 */
function cancelCollapseAnimationTween(): void {
  if (!collapseAnimationTween) {
    return;
  }

  collapseAnimationTween.stop();
  collapseAnimationTween = null;
  isCollapseAnimating.value = false;
}

/**
 * 完整清理收缩动画和尺寸锁，避免窗口关闭后继续触发异步 setSize。
 */
function clearCollapseWindowAnimation(): void {
  collapseAnimationVersion += 1;
  cancelCollapseAnimationTween();
  boxOpacityTween?.stop();
  boxOpacityTween = null;
  clearCollapsedPreviewCloseTimer();
  boxSurfaceVisualHeight.value = null;
  if (collapseSizeApplyLockTimer) {
    window.clearTimeout(collapseSizeApplyLockTimer);
    collapseSizeApplyLockTimer = null;
  }
  isApplyingCollapseWindowSize = false;
  isCollapseAnimating.value = false;
}

/**
 * 清理延迟收起计时器，所有进入 Box、菜单、拖动和缩放的交互都应先取消旧的收起任务。
 */
function clearCollapsedPreviewCloseTimer(): void {
  if (!collapsePreviewCloseTimer) {
    return;
  }

  window.clearTimeout(collapsePreviewCloseTimer);
  collapsePreviewCloseTimer = null;
}

/**
 * 交互命中 Box 时使用同一套展开入口；拖拽命中也按普通鼠标进入处理，不再维护独立拖拽展开状态。
 */
function openCollapsedPreviewForActiveInteraction(): void {
  clearCollapsedPreviewCloseTimer();
  if (box.value?.collapsed) {
    isCollapsedPreviewOpen.value = true;
  }
}

/**
 * 判断是否存在需要保持 Box 展开的交互，避免菜单、缩放、拖动过程中被 mouseleave 抢先收起。
 */
function shouldKeepCollapsedPreviewOpen(): boolean {
  return (
    isBoxHovered.value ||
    isDragHoveringBox.value ||
    isTitleHovered.value ||
    isContextMenuOpen.value ||
    isEditingTitle.value ||
    isManualDraggingBox.value ||
    isResizeHandleHovered.value ||
    isResizingBox.value
  );
}

/**
 * 拖拽坐标命中独立于真实 mouseenter/mouseleave，避免拖拽结束后把普通 hover 状态卡住。
 */
function setDragHoveringBox(isHovering: boolean): void {
  isDragHoveringBox.value = isHovering;
  if (isHovering) {
    openCollapsedPreviewForActiveInteraction();
    return;
  }

  refreshCollapsedPreviewCloseSchedule();
}

/**
 * 拖拽结束后用最终屏幕坐标恢复普通 hover 状态，避免拖拽 hover 残留或误清真实鼠标停留。
 */
async function syncPointerHoverFromScreenPoint(screenX: number, screenY: number): Promise<void> {
  const localPoint = await resolveBoxItemDragLocalPoint(screenX, screenY);

  isBoxHovered.value = localPoint.inside;
  if (localPoint.inside) {
    openCollapsedPreviewForActiveInteraction();
    return;
  }

  refreshCollapsedPreviewCloseSchedule();
}

/**
 * 鼠标离开后延迟收起，给用户从标题移动到边缘缩放或菜单窗口留出缓冲时间。
 */
function scheduleCollapsedPreviewClose(): void {
  clearCollapsedPreviewCloseTimer();
  if (!box.value?.collapsed || shouldKeepCollapsedPreviewOpen()) {
    return;
  }

  collapsePreviewCloseTimer = window.setTimeout(() => {
    collapsePreviewCloseTimer = null;
    if (!shouldKeepCollapsedPreviewOpen()) {
      isCollapsedPreviewOpen.value = false;
    }
  }, BOX_WINDOW_INTERACTION_TIMING.collapsePreviewCloseDelayMs);
}

/**
 * 状态退出时统一刷新收起计划，保证透明度淡出只会在收缩动画真正启动后再延迟发生。
 */
function refreshCollapsedPreviewCloseSchedule(): void {
  if (shouldKeepCollapsedPreviewOpen()) {
    clearCollapsedPreviewCloseTimer();
    return;
  }

  scheduleCollapsedPreviewClose();
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
 * 锁定布局和收缩态不仅隐藏自定义热区，也要关闭原生 resizable，避免窗口边缘仍可被系统缩放。
 */
function syncNativeWindowResizable(canResize: boolean): void {
  const applyVersion = windowResizableApplyVersion + 1;

  windowResizableApplyVersion = applyVersion;
  void currentWindow
    .setResizable(canResize)
    .catch((error) => {
      if (windowResizableApplyVersion === applyVersion) {
        desktopStore.lastError = error instanceof Error ? error.message : String(error);
      }
    });
}

/**
 * 拖动只从标题栏触发，避免图标区域的拖拽和窗口移动互相抢事件。
 */
function startDragging(event: MouseEvent): void {
  if (event.button !== 0 || event.detail > 1 || isEditingTitle.value || box.value?.locked) {
    return;
  }

  closeContextMenu();
  void startManualDragging(event);
}

/**
 * Box 区域 hover 进入时取消延迟收起；收缩态窗口只剩标题高度，因此进入可见区域等同于进入标题入口。
 */
function handleBoxMouseEnter(): void {
  isBoxHovered.value = true;
  openCollapsedPreviewForActiveInteraction();
}

/**
 * 标题 hover 是收缩 Box 的展开入口，内容是否展开仍由标题区域单独控制。
 */
function handleBoxTitleMouseEnter(): void {
  isTitleHovered.value = true;
  openCollapsedPreviewForActiveInteraction();
}

/**
 * 离开标题后只取消 roll-up 展开入口，内容收回仍等鼠标离开整个 Box。
 */
function handleBoxTitleMouseLeave(): void {
  isTitleHovered.value = false;
  refreshCollapsedPreviewCloseSchedule();
}

/**
 * 鼠标离开整个 Box 后延迟收回临时展开内容，避免移动到菜单或缩放边缘时立刻收缩。
 */
function handleBoxMouseLeave(): void {
  isBoxHovered.value = false;
  isTitleHovered.value = false;
  refreshCollapsedPreviewCloseSchedule();
}

/**
 * 鼠标命中缩放热区时保持展开，防止刚出现 resize 光标就被自动收起打断。
 */
function handleResizeHandleMouseEnter(): void {
  isResizeHandleHovered.value = true;
  openCollapsedPreviewForActiveInteraction();
}

/**
 * 离开缩放热区后回到统一延迟收起调度，避免 resize 边缘和标题区之间闪收。
 */
function handleResizeHandleMouseLeave(): void {
  isResizeHandleHovered.value = false;
  refreshCollapsedPreviewCloseSchedule();
}

/**
 * 缩放从窗口边缘热区触发，保持 Box 没有最小化、最大化、关闭按钮的桌面组件形态。
 */
function startResizing(direction: ResizeDirection, event: MouseEvent): void {
  if (event.button !== 0 || box.value?.locked || isBoxCollapsedToTitle.value) {
    return;
  }

  isResizingBox.value = true;
  resizeStartedAt = performance.now();
  resizeReleasedStableTicks = 0;
  openCollapsedPreviewForActiveInteraction();
  stopManualDragging(false);
  closeContextMenu();
  bindResizeReleaseEvents();
  void currentWindow.startResizeDragging(direction);
}

/**
 * Box 图标 pointer 拖拽开始时创建跨窗口会话，并启动全局鼠标轮询和拖影窗口。
 */
function startBoxItemPointerDrag(event: PointerEvent, itemPath: string): void {
  draggingBoxItemPath.value = itemPath;
  clearBoxItemDragIndicator();
  closeContextMenu();
  void beginBoxItemGlobalDrag(event, itemPath);
}

/**
 * 外部文件拖入 Box 时声明当前区域可接收文件；内部排序已改用 pointer 拖拽。
 */
function handleBoxGridDragOver(event: DragEvent): void {
  setDragHoveringBox(true);
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = "copy";
  }
}

/**
 * DOM drop 只处理外部文件路径；Box 内部排序由 pointer up 提交，避免浏览器 DnD 禁用光标。
 */
async function handleBoxGridDrop(event: DragEvent): Promise<void> {
  if (!box.value || externalFileDragState) {
    return;
  }

  await desktopStore.handleItemDrop(event, box.value.id);
}

/**
 * pointer 释放时只通知全局拖拽会话结束；实际排序由命中的 Box 处理 Drop 事件。
 */
async function finishBoxItemPointerDrag(event: PointerEvent, itemPath: string): Promise<void> {
  if (draggingBoxItemPath.value !== itemPath) {
    return;
  }

  await finishBoxItemGlobalDrag(event);
}

/**
 * 清理排序提示线，避免拖拽离开窗口或落到空白区后保留旧位置。
 */
function clearBoxItemDragIndicator(): void {
  dragInsertLineStyle.value = null;
}

/**
 * 拖拽来源状态必须和路径一起清理，避免旧来源 Box 让后续外部拖入误隐藏同名项目。
 */
function clearDraggingBoxItemState(): void {
  draggingBoxItemPath.value = null;
  draggingBoxItemSessionId.value = null;
}

/**
 * 清理拖拽视觉状态时校验 session，避免旧窗口的 cancel/drop 事件覆盖新的同路径拖拽。
 */
function isCurrentBoxItemDragPayload(payload: BoxItemDragPayload): boolean {
  return (
    draggingBoxItemPath.value === payload.item.path &&
    (!draggingBoxItemSessionId.value || draggingBoxItemSessionId.value === payload.sessionId)
  );
}

/**
 * 应用当前拖拽插入目标；目标为空时保留 Drop 到末尾的语义但隐藏竖线。
 */
function applyBoxItemDragIndicator(insertTarget: BoxGridDragInsertTarget | null): void {
  dragInsertLineStyle.value = insertTarget?.indicatorStyle ?? null;
}

/**
 * 外部文件进入 Box 时先解析第一项作为拖影代表，其余路径在 Drop 时一起收纳。
 */
async function beginExternalFileDrag(
  paths: string[],
  x: number,
  y: number,
  isScreenPosition = false,
): Promise<void> {
  const acceptedPaths = paths.filter(Boolean);
  if (acceptedPaths.length === 0) {
    return;
  }

  clearExternalFileDragReleaseProbe();
  const previewItems = await resolveExternalDragPreviewItems(acceptedPaths);
  const firstItem = previewItems[0];
  if (!firstItem) {
    return;
  }

  const screenPoint = isScreenPosition
    ? { x, y }
    : await resolveWindowClientPhysicalPointToScreen(x, y);
  setDragHoveringBox(true);
  const sessionId = crypto.randomUUID();
  externalFileDragState = {
    item: firstItem,
    preview: resolveCurrentDragPreviewOptions(),
    sessionId,
    usesScreenPosition: isScreenPosition,
  };
  void openDragPreviewWindow()
    .then(() => emitExternalFileDragPhase("move", screenPoint.x, screenPoint.y, sessionId))
    .catch((error) => {
      desktopStore.lastError = error instanceof Error ? error.message : String(error);
    });
  await emitExternalFileDragPhase("start", screenPoint.x, screenPoint.y, sessionId);
}

/**
 * 外部文件悬停时持续广播坐标，目标 Box 用同一套逻辑绘制插入线。
 */
async function moveExternalFileDrag(x: number, y: number): Promise<void> {
  const dragState = externalFileDragState;
  if (!dragState) {
    return;
  }

  const screenPoint = dragState.usesScreenPosition
    ? { x, y }
    : await resolveWindowClientPhysicalPointToScreen(x, y);
  await emitExternalFileDragPhase("move", screenPoint.x, screenPoint.y, dragState.sessionId);
}

/**
 * 外部文件 Drop 后按当前插入点写入 Box，随后关闭拖影。
 */
async function finishExternalFileDrag(paths: string[], x: number, y: number): Promise<void> {
  if (!box.value) {
    cancelExternalFileDrag();
    return;
  }

  const acceptedPaths = paths.filter(Boolean);
  if (acceptedPaths.length === 0) {
    cancelExternalFileDrag();
    return;
  }

  const dragState =
    externalFileDragState ??
    (await createExternalFileDragStateForDrop(acceptedPaths));

  if (!dragState) {
    return;
  }

  externalFileDragState = dragState;
  const screenPoint = dragState.usesScreenPosition
    ? { x, y }
    : await resolveWindowClientPhysicalPointToScreen(x, y);
  const localPoint = await resolveBoxItemDragLocalPoint(screenPoint.x, screenPoint.y);
  const insertTarget =
    localPoint.inside && boxGridRef.value
      ? resolveBoxGridDragInsertTarget(
          localPoint.x,
          localPoint.y,
          boxGridRef.value,
          dragState.item.path,
        )
      : null;

  await desktopStore.assignDroppedPathsToBox(acceptedPaths, box.value.id);
  if (insertTarget) {
    await desktopStore.reorderBoxItem(
      box.value.id,
      dragState.item.path,
      insertTarget.path,
      insertTarget.placement,
    );
  }
  await emitExternalFileDragPhase("drop", screenPoint.x, screenPoint.y, dragState.sessionId);
  externalFileDragState = null;
  clearExternalFileDragReleaseProbe();
  setDragHoveringBox(false);
  await syncPointerHoverFromScreenPoint(screenPoint.x, screenPoint.y);
  refreshCollapsedPreviewCloseSchedule();
  clearBoxItemDragIndicator();
}

/**
 * 外部拖拽离开窗口时关闭拖影和插入线，真实文件不做任何处理。
 */
function cancelExternalFileDrag(options: ExternalFileDragCancelOptions = {}): void {
  const dragState = externalFileDragState;
  externalFileDragState = null;
  if (options.deferHoverClearUntilRelease && dragState) {
    startExternalFileDragReleaseProbe(dragState.sessionId);
  } else {
    clearExternalFileDragReleaseProbe();
    setDragHoveringBox(false);
    refreshCollapsedPreviewCloseSchedule();
  }
  clearBoxItemDragIndicator();

  if (!dragState) {
    return;
  }

  void notifyBoxItemDrag({
    item: dragState.item,
    phase: "cancel",
    preview: dragState.preview,
    screenX: 0,
    screenY: 0,
    sessionId: dragState.sessionId,
    sourceBoxId: "",
  });
}

/**
 * 原生拖拽 leave 后鼠标仍处于按下态，保持临时展开直到释放，避免收缩 setSize 让 WebView 拖放状态卡死。
 */
function startExternalFileDragReleaseProbe(sessionId: string): void {
  externalFileDragReleaseSessionId = sessionId;
  externalFileDragReleaseProbeStartedAt = performance.now();
  clearExternalFileDragReleaseProbe(false);
  setDragHoveringBox(true);
  externalFileDragReleaseProbeTimer = window.setInterval(() => {
    void isPrimaryMouseButtonPressed()
      .then((isPressed) => {
        const elapsedMs = performance.now() - externalFileDragReleaseProbeStartedAt;
        if (elapsedMs < BOX_ITEM_DRAG_INTERACTION.externalReleaseMinHoldMs) {
          return;
        }

        const hasTimedOut =
          elapsedMs >= BOX_ITEM_DRAG_INTERACTION.externalReleaseFallbackMs;
        if (isPressed && !hasTimedOut) {
          return;
        }

        finishExternalFileDragReleaseProbe(sessionId);
      })
      .catch(() => {
        finishExternalFileDragReleaseProbe(sessionId);
      });
  }, BOX_ITEM_DRAG_INTERACTION.externalReleasePollIntervalMs);
}

/**
 * 外部拖拽释放兜底只清理自己的会话，防止旧 leave 的轮询把新拖拽 hover 状态误关闭。
 */
function finishExternalFileDragReleaseProbe(sessionId: string): void {
  if (externalFileDragReleaseSessionId !== sessionId) {
    return;
  }

  clearExternalFileDragReleaseProbe();
  setDragHoveringBox(false);
  refreshCollapsedPreviewCloseSchedule();
}

/**
 * 清理外部拖拽释放轮询；保留会话时用于先停旧 timer 再启动新 timer。
 */
function clearExternalFileDragReleaseProbe(shouldClearSession = true): void {
  if (externalFileDragReleaseProbeTimer) {
    window.clearInterval(externalFileDragReleaseProbeTimer);
    externalFileDragReleaseProbeTimer = null;
  }

  if (shouldClearSession) {
    externalFileDragReleaseSessionId = "";
    externalFileDragReleaseProbeStartedAt = 0;
  }
}

/**
 * Drop 事件可能先于 enter 状态初始化到达，此时临时创建一次会话以复用排序流程。
 */
async function createExternalFileDragStateForDrop(
  paths: string[],
): Promise<ExternalFileDragState | null> {
  const previewItems = await resolveExternalDragPreviewItems(paths);
  const firstItem = previewItems[0];
  if (!firstItem) {
    return null;
  }

  return {
    item: firstItem,
    preview: resolveCurrentDragPreviewOptions(),
    sessionId: crypto.randomUUID(),
    usesScreenPosition: false,
  };
}

/**
 * 外部拖入只解析第一条路径作为拖影代表，避免为了预览提前写入 Box 映射。
 */
async function resolveExternalDragPreviewItems(paths: string[]): Promise<DesktopItem[]> {
  const firstPath = paths[0];
  if (!firstPath) {
    return [];
  }

  const existingItem = desktopStore.findItem(firstPath);
  if (existingItem) {
    return [existingItem];
  }

  return getDesktopItemsByPaths([firstPath]);
}

/**
 * 外部拖放 over/drop 在 Windows WebView2 下是窗口客户区物理坐标，广播前需要转成屏幕物理坐标。
 */
async function resolveWindowClientPhysicalPointToScreen(
  x: number,
  y: number,
): Promise<PhysicalWindowPoint> {
  const position = await currentWindow.innerPosition();

  return {
    x: position.x + x,
    y: position.y + y,
  };
}

/**
 * 外部文件拖影复用 Box 内部拖拽事件，sourceBoxId 为空表示不会触发拖出删除。
 */
async function emitExternalFileDragPhase(
  phase: BoxItemDragPayload["phase"],
  screenX: number,
  screenY: number,
  sessionId: string,
): Promise<void> {
  const dragState = externalFileDragState;
  if (!dragState || dragState.sessionId !== sessionId) {
    return;
  }

  await notifyBoxItemDrag({
    item: dragState.item,
    phase,
    preview: dragState.preview,
    screenX,
    screenY,
    sessionId: dragState.sessionId,
    sourceBoxId: "",
  });
}

/**
 * 拖影沿用当前 Box 的展示配置，保持拖动中的图标大小、文字和圆角与目标工作区一致。
 */
function resolveCurrentDragPreviewOptions(): BoxItemDragPreviewOptions {
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
 * 来源 Box 创建拖拽会话后持续广播屏幕坐标，目标 Box 不需要依赖浏览器原生 DnD。
 */
async function beginBoxItemGlobalDrag(event: PointerEvent, itemPath: string): Promise<void> {
  if (!box.value || boxItemGlobalDragState) {
    clearDraggingBoxItemState();
    return;
  }

  const item = desktopStore.findItem(itemPath);
  if (!item) {
    clearDraggingBoxItemState();
    return;
  }

  const sessionId = crypto.randomUUID();
  draggingBoxItemSessionId.value = sessionId;
  const cursor = await cursorPosition().catch(() => ({
    x: Math.round(event.screenX),
    y: Math.round(event.screenY),
  }));

  boxItemGlobalDragState = {
    accepted: false,
    finishing: false,
    item,
    preview: resolveCurrentDragPreviewOptions(),
    sessionId,
    sourceBoxId: box.value.id,
  };
  acceptedBoxItemDragSessions.delete(sessionId);
  void openDragPreviewWindow()
    .then(() => emitBoxItemDragPhase("move", cursor.x, cursor.y))
    .catch((error) => {
      desktopStore.lastError = error instanceof Error ? error.message : String(error);
    });
  await emitBoxItemDragPhase("start", cursor.x, cursor.y);
  startBoxItemDragPolling();
}

/**
 * 全局鼠标轮询让拖拽在离开当前 WebView 后仍能更新拖影和目标 Box 插入线。
 */
function startBoxItemDragPolling(): void {
  clearBoxItemDragPolling();
  boxItemDragPollTimer = window.setInterval(() => {
    void pollBoxItemDragCursor();
  }, BOX_ITEM_DRAG_INTERACTION.pollIntervalMs);
}

/**
 * 每一帧读取鼠标位置和左键状态；左键释放时统一派发 Drop。
 */
async function pollBoxItemDragCursor(): Promise<void> {
  const dragState = boxItemGlobalDragState;
  if (!dragState || dragState.finishing || isBoxItemDragPollTickPending) {
    return;
  }

  isBoxItemDragPollTickPending = true;
  try {
    const [cursor, isPressed] = await Promise.all([
      cursorPosition(),
      isPrimaryMouseButtonPressed(),
    ]);
    if (!boxItemGlobalDragState || boxItemGlobalDragState.sessionId !== dragState.sessionId) {
      return;
    }

    if (!isPressed) {
      await finishBoxItemGlobalDragAt(cursor.x, cursor.y);
      return;
    }

    await emitBoxItemDragPhase("move", cursor.x, cursor.y);
  } finally {
    isBoxItemDragPollTickPending = false;
  }
}

/**
 * 清理拖拽轮询定时器，避免 Drop 结束后继续发送旧坐标。
 */
function clearBoxItemDragPolling(): void {
  if (!boxItemDragPollTimer) {
    return;
  }

  window.clearInterval(boxItemDragPollTimer);
  boxItemDragPollTimer = null;
}

/**
 * pointerup 仍在当前窗口内时使用实时鼠标坐标结束会话，避免等待下一次轮询。
 */
async function finishBoxItemGlobalDrag(event: PointerEvent): Promise<void> {
  const cursor = await cursorPosition().catch(() => ({
    x: Math.round(event.screenX),
    y: Math.round(event.screenY),
  }));

  await finishBoxItemGlobalDragAt(cursor.x, cursor.y);
}

/**
 * 结束拖拽时先广播 Drop，再给目标窗口一个短暂提交窗口；无人接收才按拖出 Box 删除映射。
 */
async function finishBoxItemGlobalDragAt(screenX: number, screenY: number): Promise<void> {
  const dragState = boxItemGlobalDragState;
  if (!dragState || dragState.finishing) {
    return;
  }

  dragState.finishing = true;
  clearBoxItemDragPolling();
  await emitBoxItemDragPhase("drop", screenX, screenY);
  window.setTimeout(() => {
    if (
      draggingBoxItemPath.value === dragState.item.path &&
      draggingBoxItemSessionId.value === dragState.sessionId
    ) {
      clearDraggingBoxItemState();
      clearBoxItemDragIndicator();
    }
  }, BOX_ITEM_DRAG_INTERACTION.sourceLayoutReleaseDelayMs);
  scheduleUnacceptedBoxItemDragRemoval(dragState);
  boxItemGlobalDragState = null;
}

/**
 * 来源窗口销毁或异常结束时取消拖影，避免残留一个始终置顶的小透明窗口。
 */
function cancelActiveBoxItemDrag(): void {
  const dragState = boxItemGlobalDragState;
  clearBoxItemDragPolling();
  clearDraggingBoxItemState();
  clearBoxItemDragIndicator();
  boxItemGlobalDragState = null;
  refreshCollapsedPreviewCloseSchedule();

  if (dragState) {
    void notifyBoxItemDrag({
      item: dragState.item,
      phase: "cancel",
      preview: dragState.preview,
      screenX: 0,
      screenY: 0,
      sessionId: dragState.sessionId,
      sourceBoxId: dragState.sourceBoxId,
    });
  }
}

/**
 * 广播拖拽阶段时复用当前会话数据，保证预览窗和所有 Box 看到同一个 sessionId。
 */
async function emitBoxItemDragPhase(
  phase: BoxItemDragPayload["phase"],
  screenX: number,
  screenY: number,
): Promise<void> {
  const dragState = boxItemGlobalDragState;
  if (!dragState) {
    return;
  }

  await notifyBoxItemDrag({
    item: dragState.item,
    phase,
    preview: dragState.preview,
    screenX,
    screenY,
    sessionId: dragState.sessionId,
    sourceBoxId: dragState.sourceBoxId,
  });
}

/**
 * 如果没有任何 Box 回执接收 Drop，就按用户拖出 Box 处理，只删除映射不动真实文件。
 */
function scheduleUnacceptedBoxItemDragRemoval(dragState: BoxItemGlobalDragState): void {
  window.setTimeout(() => {
    const hasAccepted =
      dragState.accepted || acceptedBoxItemDragSessions.has(dragState.sessionId);
    acceptedBoxItemDragSessions.delete(dragState.sessionId);
    if (hasAccepted) {
      return;
    }

    void desktopStore.removeItemFromBox(dragState.sourceBoxId, dragState.item.path);
  }, BOX_ITEM_DRAG_INTERACTION.acceptFallbackDelayMs);
}

/**
 * 所有 Box 都监听拖拽事件，但只有鼠标落入当前窗口时才展示插入线或接收 Drop。
 */
async function handleBoxItemDragPayload(payload: BoxItemDragPayload): Promise<void> {
  if (!box.value || payload.phase === "cancel") {
    if (isCurrentBoxItemDragPayload(payload)) {
      clearDraggingBoxItemState();
    }
    const isWaitingForExternalRelease =
      !payload.sourceBoxId && externalFileDragReleaseSessionId === payload.sessionId;
    if (!isWaitingForExternalRelease) {
      setDragHoveringBox(false);
    }
    clearBoxItemDragIndicator();
    if (payload.phase === "cancel" && !isWaitingForExternalRelease) {
      refreshCollapsedPreviewCloseSchedule();
    }
    return;
  }

  const localPoint = await resolveBoxItemDragLocalPoint(payload.screenX, payload.screenY);
  if (!localPoint.inside) {
    if (isCurrentBoxItemDragPayload(payload)) {
      clearDraggingBoxItemState();
    }
    setDragHoveringBox(false);
    clearBoxItemDragIndicator();
    refreshCollapsedPreviewCloseSchedule();
    return;
  }

  if (payload.phase !== "drop") {
    setDragHoveringBox(true);
  }
  const insertTarget = boxGridRef.value
    ? resolveBoxGridDragInsertTarget(
        localPoint.x,
        localPoint.y,
        boxGridRef.value,
        payload.item.path,
      )
    : null;

  applyBoxItemDragIndicator(insertTarget);
  if (payload.phase !== "drop") {
    draggingBoxItemPath.value = payload.item.path;
    draggingBoxItemSessionId.value = payload.sessionId;
    return;
  }

  if (!payload.sourceBoxId) {
    if (isCurrentBoxItemDragPayload(payload)) {
      clearDraggingBoxItemState();
    }
    clearBoxItemDragIndicator();
    setDragHoveringBox(false);
    await syncPointerHoverFromScreenPoint(payload.screenX, payload.screenY);
    refreshCollapsedPreviewCloseSchedule();
    return;
  }

  await commitBoxItemDragDrop(payload, insertTarget);
  clearDraggingBoxItemState();
  clearBoxItemDragIndicator();
  await notifyBoxItemDragAccepted({
    sessionId: payload.sessionId,
    targetBoxId: box.value.id,
  });
  setDragHoveringBox(false);
  await syncPointerHoverFromScreenPoint(payload.screenX, payload.screenY);
  refreshCollapsedPreviewCloseSchedule();
}

/**
 * 将屏幕物理坐标换算到当前无边框窗口的逻辑坐标，高 DPI 下插入线不会偏移。
 */
async function resolveBoxItemDragLocalPoint(
  screenX: number,
  screenY: number,
): Promise<BoxItemDragLocalPoint> {
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
 * Drop 命中当前 Box 时按目标决定是本 Box 排序、移动到末尾，还是跨 Box 重新收纳。
 */
async function commitBoxItemDragDrop(
  payload: BoxItemDragPayload,
  insertTarget: BoxGridDragInsertTarget | null,
): Promise<void> {
  if (!box.value) {
    return;
  }

  if (payload.sourceBoxId === box.value.id) {
    if (insertTarget) {
      await desktopStore.reorderBoxItem(
        box.value.id,
        payload.item.path,
        insertTarget.path,
        insertTarget.placement,
      );
      return;
    }

    await desktopStore.moveBoxItemToEnd(box.value.id, payload.item.path);
    return;
  }

  await desktopStore.assignDroppedPathsToBox([payload.item.path], box.value.id);
  if (insertTarget) {
    await desktopStore.reorderBoxItem(
      box.value.id,
      payload.item.path,
      insertTarget.path,
      insertTarget.placement,
    );
  }
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
  if (isApplyingCollapseWindowSize) {
    return;
  }

  clearResizePersistTimer();
  resizePersistTimer = window.setTimeout(() => {
    resizePersistTimer = null;
    persistResizeBounds();
  }, RESIZE_PERSIST_SETTLE_MS);
}

/**
 * 缩放静止或释放时保存最终边界，并同步给设置页与其他 Box 窗口。
 */
function persistResizeBounds(): void {
  clearResizePersistTimer();
  void persistCurrentWindowBounds();
}

/**
 * 鼠标释放才结束 resize 交互；窗口 resize 静止保存不会提前触发自动收缩。
 */
function finishResizeInteraction(): void {
  const wasResizing = isResizingBox.value;

  isResizingBox.value = false;
  resizeReleasedStableTicks = 0;
  clearResizeReleaseEvents();
  clearResizeInteractionReleaseProbe();
  persistResizeBounds();
  if (wasResizing) {
    refreshCollapsedPreviewCloseSchedule();
  }
}

/**
 * 监听缩放释放事件；系统原生拖拽吞掉释放事件时，resize 静止兜底仍会保存。
 */
function bindResizeReleaseEvents(): void {
  clearResizeReleaseEvents();

  window.addEventListener("mouseup", finishResizeInteraction, { capture: true, once: true });
  window.addEventListener("pointerup", finishResizeInteraction, { capture: true, once: true });
  document.addEventListener("mouseup", finishResizeInteraction, { capture: true, once: true });
  document.addEventListener("pointerup", finishResizeInteraction, { capture: true, once: true });
  resizeReleaseCleanup = () => {
    window.removeEventListener("mouseup", finishResizeInteraction, { capture: true });
    window.removeEventListener("pointerup", finishResizeInteraction, { capture: true });
    document.removeEventListener("mouseup", finishResizeInteraction, { capture: true });
    document.removeEventListener("pointerup", finishResizeInteraction, { capture: true });
  };
  startResizeInteractionReleaseProbe();
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
 * WebView 在原生 resize 期间可能收不到 mouseup，短轮询左键状态作为结束交互的兜底。
 */
function startResizeInteractionReleaseProbe(): void {
  clearResizeInteractionReleaseProbe();
  resizeInteractionReleaseProbeTimer = window.setInterval(() => {
    void isPrimaryMouseButtonPressed()
      .then((isPressed) => {
        if (isPressed) {
          resizeReleasedStableTicks = 0;
          return;
        }

        const hasMinimumResizeTimeElapsed =
          performance.now() - resizeStartedAt >=
          BOX_WINDOW_INTERACTION_TIMING.resizeReleaseProbeMinMs;
        if (!hasMinimumResizeTimeElapsed) {
          return;
        }

        resizeReleasedStableTicks += 1;
        if (
          resizeReleasedStableTicks >= BOX_WINDOW_INTERACTION_TIMING.resizeReleaseProbeStableTicks
        ) {
          finishResizeInteraction();
        }
      })
      .catch(() => undefined);
  }, BOX_ITEM_DRAG_INTERACTION.pollIntervalMs);
}

/**
 * 清理 resize 释放兜底轮询，避免窗口卸载或缩放结束后继续读取全局鼠标状态。
 */
function clearResizeInteractionReleaseProbe(): void {
  if (!resizeInteractionReleaseProbeTimer) {
    return;
  }

  window.clearInterval(resizeInteractionReleaseProbeTimer);
  resizeInteractionReleaseProbeTimer = null;
}

/**
 * 同时清理缩放相关的监听和计时器，用于窗口卸载时释放异步回调。
 */
function clearResizePersistState(): void {
  clearResizeReleaseEvents();
  clearResizePersistTimer();
  clearResizeInteractionReleaseProbe();
  isResizingBox.value = false;
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

  isManualDraggingBox.value = true;
  openCollapsedPreviewForActiveInteraction();
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
  isManualDraggingBox.value = false;
  manualDragCleanup?.();
  manualDragCleanup = null;
  refreshCollapsedPreviewCloseSchedule();

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

      <header
        class="relative flex h-10 shrink-0 select-none items-center justify-center px-3 transition-opacity duration-150 ease-out"
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
          :dragging="draggingBoxItemPath === item.path"
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
