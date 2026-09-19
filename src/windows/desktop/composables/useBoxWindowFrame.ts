import { ref, type ComputedRef } from "vue";
import {
  LogicalPosition,
  LogicalSize,
  PhysicalPosition,
  cursorPosition,
  currentMonitor,
  primaryMonitor,
} from "@tauri-apps/api/window";
import { isPrimaryMouseButtonPressed } from "@/entities/desktopItem/api";
import type { DesktopBox } from "@/entities/desktopBox/types";
import {
  BOX_ITEM_DRAG_INTERACTION,
  BOX_WINDOW_SIZE,
  BOX_WINDOW_INTERACTION_TIMING,
} from "@/entities/desktopBox/layout";
import type { AppSettings } from "@/entities/appSettings/types";
import { resolveBoxResizeGridSnappedBounds } from "../utils/boxResizeGrid";

/**
 * 物理坐标用于和 Tauri 窗口移动事件保持同一坐标体系，高 DPI 下再单独换算逻辑坐标
 */
export interface PhysicalWindowPoint {
  x: number;
  y: number;
}

/**
 * 收缩动画最终写回原生窗口时同时包含位置和尺寸，标题在下方时需要用它保持标题视觉锚点
 */
export interface LogicalWindowFrame {
  height: number;
  width: number;
  x: number;
  y: number;
}

/**
 * 屏幕工作区使用物理坐标保存，拖动时可直接和窗口物理坐标比较，避免高 DPI 下贴边偏移
 */
interface PhysicalWorkArea {
  height: number;
  width: number;
  x: number;
  y: number;
}

/**
 * 原生拖动期间只需记住窗口尺寸与工作区，供松手后的吸附校正使用；
 * 拖动过程本身由操作系统移动循环驱动，JS 不参与每一帧定位
 */
interface NativeDragState {
  height: number;
  scaleFactor: number;
  width: number;
  workArea?: PhysicalWorkArea;
}

/**
 * 手动 resize 用逻辑坐标保存起始窗口边界，鼠标轮询使用物理坐标再按 DPI 换算
 */
interface ManualResizeState {
  cursorStartX: number;
  cursorStartY: number;
  direction: ResizeDirection;
  lastFrame: LogicalWindowFrame;
  scaleFactor: number;
  startFrame: LogicalWindowFrame;
}

export type ResizeDirection =
  | "East"
  | "North"
  | "NorthEast"
  | "NorthWest"
  | "South"
  | "SouthEast"
  | "SouthWest"
  | "West";

/**
 * 无边框 Box 需要显式提供缩放热区：热区比视觉边界更宽便于命中，
 * 平时完全透明，悬停时显示圆角高亮提示边界可抓取（边界判定提示）
 */
const resizeHandles: Array<{
  direction: ResizeDirection;
  className: string;
}> = [
  { direction: "North", className: "left-3 right-3 top-0 h-3 cursor-ns-resize" },
  { direction: "South", className: "bottom-0 left-3 right-3 h-3 cursor-ns-resize" },
  { direction: "West", className: "bottom-3 left-0 top-3 w-3 cursor-ew-resize" },
  { direction: "East", className: "bottom-3 right-0 top-3 w-3 cursor-ew-resize" },
  { direction: "NorthWest", className: "left-0 top-0 size-5 cursor-nwse-resize" },
  { direction: "NorthEast", className: "right-0 top-0 size-5 cursor-nesw-resize" },
  { direction: "SouthWest", className: "bottom-0 left-0 size-5 cursor-nesw-resize" },
  { direction: "SouthEast", className: "bottom-0 right-0 size-5 cursor-nwse-resize" },
];

/**
 * 原生拖动的释放探测间隔：模态移动循环内 WebView 收不到 mouseup，
 * 只能轮询全局左键状态判断松手，32ms 足够跟手且开销可忽略
 */
const NATIVE_DRAG_RELEASE_PROBE_MS = 32;

/**
 * 只声明当前桌面窗口用到的 Tauri 能力，降低组合式逻辑对具体窗口类的类型耦合；
 * 窗口效果参数保持结构化透传，效果清单由调用方按设置组装
 */
interface DesktopWindowHandle {
  clearWindowEffects: () => Promise<void>;
  outerPosition: () => Promise<PhysicalWindowPoint>;
  outerSize: () => Promise<{ height: number; width: number }>;
  scaleFactor: () => Promise<number>;
  setEffects: (effects: { effects: string[] }) => Promise<void>;
  setPosition: (position: LogicalPosition | PhysicalPosition) => Promise<void>;
  setResizable: (resizable: boolean) => Promise<void>;
  setSize: (size: LogicalSize) => Promise<void>;
  startDragging: () => Promise<void>;
}

/**
 * Store 更新选项只暴露本模块需要的 sanitize 标记，避免把整个 Store 类型带进窗口几何层
 */
interface UpdateBoxOptions {
  sanitize?: boolean;
}

/**
 * Box 窗口框架组合式逻辑负责原生窗口位置、尺寸、拖动、缩放和边界落库
 */
export function useBoxWindowFrame(options: {
  beforeNativeDragStart?: () => Promise<void>;
  box: ComputedRef<DesktopBox | undefined>;
  clearExpandHoverTimer?: () => void;
  closeContextMenu: () => void;
  currentWindow: DesktopWindowHandle;
  getBoxes: () => DesktopBox[];
  getBoxAcrylicEnabled: () => boolean;
  getResizeGridSettings: () => AppSettings;
  getSnapThreshold: () => number;
  getSnapToEdges: () => boolean;
  isBoxCollapsedToTitle: () => boolean;
  isCollapseWindowSizeApplying: () => boolean;
  isEditingTitle: () => boolean;
  openCollapsedPreviewForActiveInteraction: () => void;
  refreshCollapsedPreviewCloseSchedule: () => void;
  setLastError: (message: string) => void;
  updateBox: (box: DesktopBox, options?: UpdateBoxOptions) => Promise<void>;
}) {
  const isManualDraggingBox = ref(false);
  const isResizeHandleHovered = ref(false);
  const isResizingBox = ref(false);
  let isApplyingWindowPosition = false;
  let isApplyingProgrammaticResize = false;
  let windowPositionApplyVersion = 0;
  let windowResizableApplyVersion = 0;
  let programmaticResizeApplyVersion = 0;
  let nativeDragState: NativeDragState | null = null;
  let nativeDragReleaseProbeTimer: ReturnType<typeof window.setInterval> | null = null;
  let dragStartPhysicalPosition: PhysicalWindowPoint = { x: 0, y: 0 };
  let manualResizeState: ManualResizeState | null = null;
  let manualResizeFrameTimer: ReturnType<typeof window.setInterval> | null = null;
  let manualResizeApplyPending = false;
  let pendingManualResizeFrame: LogicalWindowFrame | null = null;
  let resizeReleaseCleanup: (() => void) | null = null;
  let resizeInteractionReleaseProbeTimer: ReturnType<typeof window.setInterval> | null = null;
  let resizePersistTimer: ReturnType<typeof window.setTimeout> | null = null;
  let activeResizeDirection: ResizeDirection | null = null;
  let resizeStartedAt = 0;
  let resizeReleasedStableTicks = 0;

  /**
   * 初始化时用持久化数据校准窗口尺寸，防止分辨率变化后窗口状态与数据库脱节
   */
  async function syncWindowBoundsFromStore(): Promise<void> {
    if (!options.box.value) {
      return;
    }

    const resizeApplyVersion = programmaticResizeApplyVersion + 1;

    programmaticResizeApplyVersion = resizeApplyVersion;
    isApplyingProgrammaticResize = true;
    try {
      await options.currentWindow.setSize(
        new LogicalSize(options.box.value.width, options.box.value.height),
      );
      await options.currentWindow.setPosition(
        new LogicalPosition(options.box.value.x, options.box.value.y),
      );
    } finally {
      window.setTimeout(() => {
        if (programmaticResizeApplyVersion === resizeApplyVersion) {
          isApplyingProgrammaticResize = false;
        }
      }, BOX_WINDOW_INTERACTION_TIMING.positionApplyLockMs);
    }
    await ensureWindowInsideMonitor();
  }

  /**
   * 收缩和展开会主动调整窗口几何状态，需要加移动锁，防止 onMoved 把临时标题位置误写入数据库
   */
  async function applyCollapseWindowFrame(frame: LogicalWindowFrame): Promise<void> {
    const applyVersion = windowPositionApplyVersion + 1;

    windowPositionApplyVersion = applyVersion;
    isApplyingWindowPosition = true;
    try {
      await Promise.all([
        options.currentWindow.setPosition(new LogicalPosition(frame.x, frame.y)),
        options.currentWindow.setSize(new LogicalSize(frame.width, frame.height)),
      ]);
    } finally {
      window.setTimeout(() => {
        if (windowPositionApplyVersion === applyVersion) {
          isApplyingWindowPosition = false;
        }
      }, BOX_WINDOW_INTERACTION_TIMING.positionApplyLockMs);
    }
  }

  /**
   * 外部窗口移动只负责持久化；原生拖动期间、收缩预览展开、动画或程序性位移期间绝对不覆盖数据库中的锚点坐标，
   * 避免展开临时坐标污染数据库导致 Box 漂移走位
   */
  async function handleWindowMoved(x: number, y: number): Promise<void> {
    if (
      nativeDragState ||
      isApplyingWindowPosition ||
      isApplyingProgrammaticResize ||
      options.isCollapseWindowSizeApplying() ||
      options.isBoxCollapsedToTitle() ||
      options.box.value?.collapsed
    ) {
      return;
    }

    await persistWindowPositionFromPhysical(x, y);
  }

  /**
   * resize 由自定义热区接管，原生边框缩放保持关闭，避免绕过网格化尺寸计算
   */
  function syncNativeWindowResizable(): void {
    const applyVersion = windowResizableApplyVersion + 1;

    windowResizableApplyVersion = applyVersion;
    void options.currentWindow
      .setResizable(false)
      .catch((error) => {
        if (windowResizableApplyVersion === applyVersion) {
          options.setLastError(error instanceof Error ? error.message : String(error));
        }
      });
  }

  /**
   * 拖动只从标题栏触发，避免图标区域的拖拽和窗口移动互相抢事件
   */
  function startDragging(event: MouseEvent): void {
    if (
      event.button !== 0 ||
      event.detail > 1 ||
      options.isEditingTitle() ||
      options.box.value?.locked
    ) {
      return;
    }

    options.closeContextMenu();
    void startNativeDragging();
  }

  /**
   * 鼠标命中缩放热区时保持展开，防止刚出现 resize 光标就被自动收起打断
   */
  function handleResizeHandleMouseEnter(): void {
    isResizeHandleHovered.value = true;
    options.openCollapsedPreviewForActiveInteraction();
  }

  /**
   * 离开缩放热区后回到统一延迟收起调度，避免 resize 边缘和标题区之间闪收
   */
  function handleResizeHandleMouseLeave(): void {
    isResizeHandleHovered.value = false;
    options.refreshCollapsedPreviewCloseSchedule();
  }

  /**
   * 缩放从窗口边缘热区触发，保持 Box 没有最小化、最大化、关闭按钮的桌面组件形态
   */
  function startResizing(direction: ResizeDirection, event: MouseEvent): void {
    if (event.button !== 0 || options.box.value?.locked || options.isBoxCollapsedToTitle()) {
      return;
    }

    void startManualResizing(direction).catch((error) => {
      clearResizePersistState();
      options.setLastError(error instanceof Error ? error.message : String(error));
    });
  }

  /**
   * 手动 resize 由全局鼠标坐标驱动，便于在拖动过程中实时按网格步进改变窗口尺寸
   */
  async function startManualResizing(direction: ResizeDirection): Promise<void> {
    if (!options.box.value || manualResizeState) {
      return;
    }

    const [windowPosition, windowSize, cursor, scaleFactor] = await Promise.all([
      options.currentWindow.outerPosition(),
      options.currentWindow.outerSize(),
      cursorPosition(),
      options.currentWindow.scaleFactor(),
    ]);
    const logicalPosition = new PhysicalPosition(windowPosition.x, windowPosition.y).toLogical(
      scaleFactor,
    );
    const startFrame = {
      height: windowSize.height / scaleFactor,
      width: windowSize.width / scaleFactor,
      x: logicalPosition.x,
      y: logicalPosition.y,
    };

    isResizingBox.value = true;
    activeResizeDirection = direction;
    resizeStartedAt = performance.now();
    resizeReleasedStableTicks = 0;
    manualResizeState = {
      cursorStartX: cursor.x,
      cursorStartY: cursor.y,
      direction,
      lastFrame: startFrame,
      scaleFactor,
      startFrame,
    };
    options.openCollapsedPreviewForActiveInteraction();
    stopManualDragging(false);
    options.closeContextMenu();
    bindResizeReleaseEvents();
    startManualResizeFrameLoop();
  }

  /**
   * resize 期间轮询系统鼠标位置，即使窗口边界移动导致指针离开 WebView 也能继续计算尺寸
   */
  function startManualResizeFrameLoop(): void {
    clearManualResizeFrameLoop();
    manualResizeFrameTimer = window.setInterval(() => {
      void updateManualResizeFrame();
    }, BOX_ITEM_DRAG_INTERACTION.pollIntervalMs);
  }

  /**
   * 每一帧都基于起始边界和鼠标位移重新计算，避免连续取整导致尺寸误差累积
   */
  async function updateManualResizeFrame(): Promise<void> {
    const resizeState = manualResizeState;
    if (!resizeState) {
      return;
    }

    const cursor = await cursorPosition();
    const nextFrame = resolveManualResizeFrame(cursor, resizeState);
    if (areWindowFramesEqual(nextFrame, resizeState.lastFrame)) {
      return;
    }

    resizeState.lastFrame = nextFrame;
    requestManualResizeFrameApply(nextFrame);
  }

  /**
   * 原生窗口写入可能慢于鼠标轮询，始终只保留最新一帧，避免排队应用过期尺寸
   */
  function requestManualResizeFrameApply(frame: LogicalWindowFrame): void {
    pendingManualResizeFrame = frame;
    if (manualResizeApplyPending) {
      return;
    }

    manualResizeApplyPending = true;
    void flushManualResizeFrameApply();
  }

  /**
   * 顺序应用最新窗口边界，保证 Tauri setSize/setPosition 不被并发写入打乱
   */
  async function flushManualResizeFrameApply(): Promise<void> {
    try {
      while (pendingManualResizeFrame) {
        const frame = pendingManualResizeFrame;

        pendingManualResizeFrame = null;
        await applyResizeSnappedWindowBounds(frame);
      }
    } catch (error) {
      options.setLastError(error instanceof Error ? error.message : String(error));
    } finally {
      manualResizeApplyPending = false;
      if (pendingManualResizeFrame) {
        requestManualResizeFrameApply(pendingManualResizeFrame);
      }
    }
  }

  /**
   * 根据拖拽方向更新对应边；开启网格时再换算成完整图标行列。
   */
  function resolveManualResizeFrame(
    cursor: PhysicalWindowPoint,
    resizeState: ManualResizeState,
  ): LogicalWindowFrame {
    const deltaX = (cursor.x - resizeState.cursorStartX) / resizeState.scaleFactor;
    const deltaY = (cursor.y - resizeState.cursorStartY) / resizeState.scaleFactor;
    const rawFrame = resolveRawResizeFrame(resizeState.startFrame, resizeState.direction, {
      x: deltaX,
      y: deltaY,
    });

    return resolveResizeFrameForSettings(rawFrame, resizeState.direction);
  }

  /**
   * 未开启网格调整时仍保持最小尺寸约束，避免手动写入小于 Tauri 最小窗口的值。
   */
  function resolveResizeFrameForSettings(
    frame: LogicalWindowFrame,
    direction: ResizeDirection | null,
  ): LogicalWindowFrame {
    const settings = options.getResizeGridSettings();
    if (settings.boxResizeGridEnabled) {
      return resolveBoxResizeGridSnappedBounds(frame, settings, direction);
    }

    return resolveMinimumResizeFrame(frame, direction);
  }

  /**
   * 连续缩放模式只做最小尺寸夹取，左/上边拖动时保持右/下边不漂移
   */
  function resolveMinimumResizeFrame(
    frame: LogicalWindowFrame,
    direction: ResizeDirection | null,
  ): LogicalWindowFrame {
    const width = Math.max(frame.width, BOX_WINDOW_SIZE.min.width);
    const height = Math.max(frame.height, BOX_WINDOW_SIZE.min.height);

    return {
      height,
      width,
      x: shouldResizeAnchorRightEdge(direction) ? frame.x + frame.width - width : frame.x,
      y: shouldResizeAnchorBottomEdge(direction) ? frame.y + frame.height - height : frame.y,
    };
  }

  /**
   * 从原始起点按拖拽方向计算连续尺寸，后续再由网格或最小尺寸规则收口
   */
  function resolveRawResizeFrame(
    startFrame: LogicalWindowFrame,
    direction: ResizeDirection,
    delta: { x: number; y: number },
  ): LogicalWindowFrame {
    const resizesWest = shouldResizeAnchorRightEdge(direction);
    const resizesNorth = shouldResizeAnchorBottomEdge(direction);
    const resizesEast =
      direction === "East" || direction === "NorthEast" || direction === "SouthEast";
    const resizesSouth =
      direction === "South" || direction === "SouthEast" || direction === "SouthWest";
    const width = startFrame.width + (resizesEast ? delta.x : 0) - (resizesWest ? delta.x : 0);
    const height = startFrame.height + (resizesSouth ? delta.y : 0) - (resizesNorth ? delta.y : 0);

    return {
      height,
      width,
      x: resizesWest ? startFrame.x + delta.x : startFrame.x,
      y: resizesNorth ? startFrame.y + delta.y : startFrame.y,
    };
  }

  /**
   * 窗口边界只按整数像素比较，避免浮点微差导致重复 setSize
   */
  function areWindowFramesEqual(left: LogicalWindowFrame, right: LogicalWindowFrame): boolean {
    return (
      Math.round(left.x) === Math.round(right.x) &&
      Math.round(left.y) === Math.round(right.y) &&
      Math.round(left.width) === Math.round(right.width) &&
      Math.round(left.height) === Math.round(right.height)
    );
  }

  /**
   * 从左侧缩放时右边缘是用户眼中的固定锚点，网格和连续模式都遵循这一点
   */
  function shouldResizeAnchorRightEdge(direction: ResizeDirection | null): boolean {
    return direction === "West" || direction === "NorthWest" || direction === "SouthWest";
  }

  /**
   * 从上方缩放时底边缘是用户眼中的固定锚点，避免尺寸夹取后窗口向下漂移
   */
  function shouldResizeAnchorBottomEdge(direction: ResizeDirection | null): boolean {
    return direction === "North" || direction === "NorthEast" || direction === "NorthWest";
  }

  /**
   * 清理手动 resize 的鼠标轮询，窗口卸载或松手后不再继续写入尺寸
   */
  function clearManualResizeFrameLoop(): void {
    if (!manualResizeFrameTimer) {
      return;
    }

    window.clearInterval(manualResizeFrameTimer);
    manualResizeFrameTimer = null;
  }

  /**
   * 持久化窗口位置用于下次启动恢复 Box，并广播给其他独立 Box 作为后续吸附参照
   */
  async function persistWindowPosition(x: number, y: number): Promise<void> {
    if (!options.box.value) {
      return;
    }

    await options.updateBox(
      {
        ...options.box.value,
        x: Math.round(x),
        y: Math.round(y),
      },
      {
        sanitize: false,
      },
    );
  }

  /**
   * Tauri 移动事件返回物理坐标，持久化前转回逻辑坐标，保证高 DPI 下重启位置不漂移
   */
  async function persistWindowPositionFromPhysical(x: number, y: number): Promise<void> {
    const scaleFactor = await options.currentWindow.scaleFactor();
    const logicalPosition = new PhysicalPosition(x, y).toLogical(scaleFactor);

    await persistWindowPosition(logicalPosition.x, logicalPosition.y);
  }

  /**
   * 持久化窗口完整边界，用于缩放结束后一次性保存位置和尺寸
   */
  async function persistWindowBounds(
    x: number,
    y: number,
    width: number,
    height: number,
  ): Promise<void> {
    if (!options.box.value) {
      return;
    }

    await options.updateBox({
      ...options.box.value,
      x: Math.round(x),
      y: Math.round(y),
      width: Math.round(width),
      height: Math.round(height),
    });
  }

  /**
   * 读取当前真实窗口边界后落库，避免 resize payload 只包含尺寸而漏掉左上方向缩放的位置变化
   */
  async function persistCurrentWindowBounds(
    resizeDirection: ResizeDirection | null = activeResizeDirection,
  ): Promise<void> {
    const [position, size, scaleFactor] = await Promise.all([
      options.currentWindow.outerPosition(),
      options.currentWindow.outerSize(),
      options.currentWindow.scaleFactor(),
    ]);
    const logicalPosition = new PhysicalPosition(position.x, position.y).toLogical(scaleFactor);
    const snappedBounds = resolveResizeFrameForSettings(
      {
        height: size.height / scaleFactor,
        width: size.width / scaleFactor,
        x: logicalPosition.x,
        y: logicalPosition.y,
      },
      resizeDirection,
    );

    await applyResizeSnappedWindowBounds(snappedBounds);
    await persistWindowBounds(
      snappedBounds.x,
      snappedBounds.y,
      snappedBounds.width,
      snappedBounds.height,
    );
  }

  /**
   * 只有用户从缩放热区发起的 resize 才允许安排尺寸落库；自动收起、启动恢复和 DPI 校准也会触发
   * Tauri 的 onResized，必须把这些展示态尺寸排除在正式 Box 布局之外。
   */
  function scheduleResizePersist(): void {
    if (
      activeResizeDirection === null ||
      options.isCollapseWindowSizeApplying() ||
      isApplyingProgrammaticResize
    ) {
      return;
    }

    clearResizePersistTimer();
    resizePersistTimer = window.setTimeout(() => {
      resizePersistTimer = null;
      persistResizeBounds();
    }, BOX_WINDOW_INTERACTION_TIMING.resizePersistSettleMs);
  }

  /**
   * 缩放静止或释放时保存最终边界；没有缩放方向说明当前 resize 不是用户手动调整，
   * 这类程序性窗口尺寸不能覆盖用户保存的 Box 宽高。
   */
  function persistResizeBounds(): void {
    clearResizePersistTimer();
    if (isResizingBox.value || activeResizeDirection === null) {
      return;
    }

    void persistCurrentWindowBounds(activeResizeDirection);
  }

  /**
   * 鼠标释放才结束 resize 交互；窗口 resize 静止保存不会提前触发自动收缩
   */
  function finishResizeInteraction(): void {
    const wasResizing = isResizingBox.value;
    const finalManualResizeFrame = manualResizeState?.lastFrame ?? null;

    isResizingBox.value = false;
    resizeReleasedStableTicks = 0;
    clearResizeReleaseEvents();
    clearResizePersistTimer();
    clearResizeInteractionReleaseProbe();
    clearManualResizeFrameLoop();
    manualResizeState = null;
    if (finalManualResizeFrame) {
      requestManualResizeFrameApply(finalManualResizeFrame);
      void persistWindowBounds(
        finalManualResizeFrame.x,
        finalManualResizeFrame.y,
        finalManualResizeFrame.width,
        finalManualResizeFrame.height,
      );
    } else {
      persistResizeBounds();
    }
    activeResizeDirection = null;
    if (wasResizing) {
      options.refreshCollapsedPreviewCloseSchedule();
    }
  }

  /**
   * 监听缩放释放事件；系统原生拖拽吞掉释放事件时，resize 静止兜底仍会保存
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
   * 清理缩放释放监听，避免重复缩放时多次写入最终边界
   */
  function clearResizeReleaseEvents(): void {
    resizeReleaseCleanup?.();
    resizeReleaseCleanup = null;
  }

  /**
   * 清理缩放兜底计时器，窗口销毁或已经松手保存时不再重复落库
   */
  function clearResizePersistTimer(): void {
    if (!resizePersistTimer) {
      return;
    }

    window.clearTimeout(resizePersistTimer);
    resizePersistTimer = null;
  }

  /**
   * WebView 在原生 resize 期间可能收不到 mouseup，短轮询左键状态作为结束交互的兜底
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
   * 清理 resize 释放兜底轮询，避免窗口卸载或缩放结束后继续读取全局鼠标状态
   */
  function clearResizeInteractionReleaseProbe(): void {
    if (!resizeInteractionReleaseProbeTimer) {
      return;
    }

    window.clearInterval(resizeInteractionReleaseProbeTimer);
    resizeInteractionReleaseProbeTimer = null;
  }

  /**
   * 同时清理缩放相关的监听和计时器，用于窗口卸载时释放异步回调
   */
  function clearResizePersistState(): void {
    clearResizeReleaseEvents();
    clearResizePersistTimer();
    clearResizeInteractionReleaseProbe();
    clearManualResizeFrameLoop();
    isResizingBox.value = false;
    activeResizeDirection = null;
    manualResizeState = null;
    pendingManualResizeFrame = null;
  }

  /**
   * resize 吸附会主动写回窗口尺寸和左上角，短暂屏蔽由这次写回触发的移动/缩放事件
   */
  async function applyResizeSnappedWindowBounds(frame: LogicalWindowFrame): Promise<void> {
    const applyVersion = programmaticResizeApplyVersion + 1;
    const positionApplyVersion = windowPositionApplyVersion + 1;

    programmaticResizeApplyVersion = applyVersion;
    windowPositionApplyVersion = positionApplyVersion;
    isApplyingProgrammaticResize = true;
    isApplyingWindowPosition = true;
    try {
      await Promise.all([
        options.currentWindow.setPosition(new LogicalPosition(frame.x, frame.y)),
        options.currentWindow.setSize(new LogicalSize(frame.width, frame.height)),
      ]);
    } finally {
      window.setTimeout(() => {
        if (programmaticResizeApplyVersion === applyVersion) {
          isApplyingProgrammaticResize = false;
        }
        if (windowPositionApplyVersion === positionApplyVersion) {
          isApplyingWindowPosition = false;
        }
      }, BOX_WINDOW_INTERACTION_TIMING.positionApplyLockMs);
    }
  }

  /**
   * 分辨率或 DPI 变化后，把 Box 拉回当前显示器工作区，避免窗口跑到屏幕外
   */
  async function ensureWindowInsideMonitor(): Promise<void> {
    if (!options.box.value || isApplyingWindowPosition) {
      return;
    }

    const monitor = (await currentMonitor()) ?? (await primaryMonitor());
    if (!monitor) {
      return;
    }

    const position = await options.currentWindow.outerPosition();
    const size = await options.currentWindow.outerSize();
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
   * 原生拖动：按下后立即向窗口线程发起模态移动循环请求——锚点取按下瞬间的光标，
   * 零启动延迟绝对跟手；吸附所需信息在拖动请求发出后并行补齐，不阻塞拖动启动。
   */
  async function startNativeDragging(): Promise<void> {
    if (!options.box.value || nativeDragState) {
      return;
    }

    try {
      dragStartPhysicalPosition = await options.currentWindow.outerPosition();
    } catch {
      dragStartPhysicalPosition = { x: 0, y: 0 };
    }

    isManualDraggingBox.value = true;
    options.clearExpandHoverTimer?.();
    // 若图标态展开动画仍在进行，先立即收回图标态再进入原生拖动：
    // 拖动必须携带稳定的目标尺寸，进行中的展开动画会在拖动中改写窗口大小造成拉伸
    await options.beforeNativeDragStart?.();
    if (!options.isBoxCollapsedToTitle()) {
      options.openCollapsedPreviewForActiveInteraction();
    }
    bindNativeDragReleaseProbe();

    try {
      const dragStarted = options.currentWindow.startDragging();
      void prepareNativeDragResources();
      await dragStarted;
    } catch (error) {
      stopManualDragging(false).catch(() => undefined);
      options.setLastError(error instanceof Error ? error.message : String(error));
    }
  }

  /**
   * 拖动循环运行期间并行预取吸附所需的窗口尺寸、缩放系数和工作区
   */
  async function prepareNativeDragResources(): Promise<void> {
    const [windowSize, scaleFactor, monitor] = await Promise.all([
      options.currentWindow.outerSize(),
      options.currentWindow.scaleFactor(),
      currentMonitor(),
    ]);
    const activeMonitor = monitor ?? (await primaryMonitor());

    nativeDragState = {
      height: windowSize.height,
      scaleFactor,
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
  }

  /**
   * 原生移动循环内 WebView 收不到 mouseup，短轮询左键状态作为结束信号；
   * 32ms 间隔的一次轻量状态查询远低于旧管线每 mousemove 一次 setPosition 的开销
   */
  function bindNativeDragReleaseProbe(): void {
    clearNativeDragReleaseProbe();
    nativeDragReleaseProbeTimer = window.setInterval(() => {
      void isPrimaryMouseButtonPressed()
        .then((isPressed) => {
          if (!isPressed && nativeDragState) {
            void stopManualDragging(true).catch(() => undefined);
          }
        })
        .catch(() => undefined);
    }, NATIVE_DRAG_RELEASE_PROBE_MS);
  }

  /**
   * 清理原生拖动的释放探测，窗口卸载或松手后不再读取全局鼠标状态
   */
  function clearNativeDragReleaseProbe(): void {
    if (!nativeDragReleaseProbeTimer) {
      return;
    }

    window.clearInterval(nativeDragReleaseProbeTimer);
    nativeDragReleaseProbeTimer = null;
  }

  /**
   * 结束拖动：shouldPersist 时取真实最终位置，过一遍吸附后由带锁 apply
   * 一次性校正定位并写入数据库；顺带覆盖原生循环可能的旧位置回写，
   * 并恢复被拖动期暂停的系统毛玻璃。
   * 若位移极小（< 4px）判定为单纯单击，折叠态下直接平滑展开。
   * 状态清理永远执行：极速拖动时预取可能尚未完成（dragState 为空），
   * 此时跳过吸附落库但绝不能留下拖动标记和探针计时器
   */
  async function stopManualDragging(shouldPersist: boolean): Promise<void> {
    const dragState = nativeDragState;

    nativeDragState = null;
    isManualDraggingBox.value = false;
    clearNativeDragReleaseProbe();
    options.refreshCollapsedPreviewCloseSchedule();
    if (!shouldPersist) {
      return;
    }

    const [finalPosition, monitor] = await Promise.all([
      options.currentWindow.outerPosition(),
      currentMonitor(),
    ]);

    if (!dragState) {
      // 预取未完成的极速拖动：位置依然准确（原生循环已落定），仅跳过吸附直接落库
      await persistWindowPositionFromPhysical(finalPosition.x, finalPosition.y);
      return;
    }

    const dragDistance = Math.hypot(
      finalPosition.x - dragStartPhysicalPosition.x,
      finalPosition.y - dragStartPhysicalPosition.y,
    );
    if (dragDistance < 4) {
      if (options.isBoxCollapsedToTitle()) {
        options.openCollapsedPreviewForActiveInteraction();
      }
      return;
    }

    const activeMonitor = monitor ?? (await primaryMonitor());
    if (activeMonitor) {
      dragState.workArea = {
        height: activeMonitor.workArea.size.height,
        width: activeMonitor.workArea.size.width,
        x: activeMonitor.workArea.position.x,
        y: activeMonitor.workArea.position.y,
      };
      dragState.scaleFactor = activeMonitor.scaleFactor;
    }

    const snappedPosition = resolveManualDragPosition(finalPosition, dragState);
    let persistY = snappedPosition.y;
    if (
      options.isBoxCollapsedToTitle() &&
      options.box.value?.collapseMode === "window" &&
      options.box.value?.titlePosition === "bottom"
    ) {
      const scaleFactor = dragState.scaleFactor;
      persistY = snappedPosition.y - Math.round((options.box.value.height - 40) * scaleFactor);
    }

    await applyWindowPhysicalPosition(snappedPosition.x, snappedPosition.y, true, persistY);
  }

  /**
   * 程序主动移动窗口时统一加锁，避免 setPosition 自己触发的 onMoved 被误判成外部移动
   */
  async function applyWindowPhysicalPosition(
    x: number,
    y: number,
    shouldPersist = true,
    persistPhysicalY?: number,
  ): Promise<void> {
    const applyVersion = windowPositionApplyVersion + 1;

    windowPositionApplyVersion = applyVersion;
    isApplyingWindowPosition = true;
    try {
      await options.currentWindow.setPosition(new PhysicalPosition(x, y));
      if (shouldPersist) {
        await persistWindowPositionFromPhysical(x, persistPhysicalY ?? y);
      }
    } finally {
      window.setTimeout(() => {
        if (windowPositionApplyVersion === applyVersion) {
          isApplyingWindowPosition = false;
        }
      }, BOX_WINDOW_INTERACTION_TIMING.positionApplyLockMs);
    }
  }

  /**
   * 拖动吸附与移出屏幕自动回滚：
   * 1. 实时参考其他 Box 的相邻边做磁吸
   * 2. 当靠近屏幕工作区边缘时在阈值内自动贴边吸附
   * 3. 当窗口被拖出屏幕边缘外时，松手自动回滚吸附到屏幕工作区边缘内，防止窗口移出丢失
   */
  function resolveManualDragPosition(
    rawPosition: PhysicalWindowPoint,
    dragState: NativeDragState,
  ): PhysicalWindowPoint {
    const threshold = options.getSnapThreshold();
    let nextX = rawPosition.x;
    let nextY = rawPosition.y;

    if (options.getSnapToEdges()) {
      for (const otherBox of options.getBoxes()) {
        if (otherBox.id === options.box.value?.id) {
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
    }

    if (dragState.workArea) {
      const workX = dragState.workArea.x;
      const workY = dragState.workArea.y;
      const workRight = workX + dragState.workArea.width;
      const workBottom = workY + dragState.workArea.height;
      const maxX = Math.max(workX, workRight - dragState.width);
      const maxY = Math.max(workY, workBottom - dragState.height);

      if (options.getSnapToEdges()) {
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

      // 移出屏幕边缘外的自动回滚吸附：当窗口任一边被拖拽越过屏幕工作区时，自动平滑弹回吸附在屏幕边缘内
      if (nextX < workX) {
        nextX = workX;
      } else if (nextX > maxX) {
        nextX = maxX;
      }

      if (nextY < workY) {
        nextY = workY;
      } else if (nextY > maxY) {
        nextY = maxY;
      }
    }

    return {
      x: nextX,
      y: nextY,
    };
  }

  return {
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
  };
}
