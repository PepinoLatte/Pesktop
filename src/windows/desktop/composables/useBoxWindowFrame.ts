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
 * 手写拖动状态保存鼠标与窗口左上角的偏移，实时移动时复用窗口尺寸和缩放系数
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
 * 无边框 Box 需要显式提供缩放热区，否则透明窗口在 Windows 上不一定有稳定边缘命中
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

/**
 * 只声明当前桌面窗口用到的 Tauri 能力，降低组合式逻辑对具体窗口类的类型耦合
 */
interface DesktopWindowHandle {
  outerPosition: () => Promise<PhysicalWindowPoint>;
  outerSize: () => Promise<{ height: number; width: number }>;
  scaleFactor: () => Promise<number>;
  setPosition: (position: LogicalPosition | PhysicalPosition) => Promise<void>;
  setResizable: (resizable: boolean) => Promise<void>;
  setSize: (size: LogicalSize) => Promise<void>;
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
  box: ComputedRef<DesktopBox | undefined>;
  closeContextMenu: () => void;
  currentWindow: DesktopWindowHandle;
  getBoxes: () => DesktopBox[];
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
  let manualDragState: ManualDragState | null = null;
  let manualDragCleanup: (() => void) | null = null;
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
   * 外部窗口移动只负责持久化，手写拖动期间的位置由拖动循环统一保存
   */
  async function handleWindowMoved(x: number, y: number): Promise<void> {
    if (isApplyingWindowPosition || manualDragState) {
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
    void startManualDragging(event);
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
   * 缩放事件只安排最终保存，不在拖动过程中写 SQLite
   */
  function scheduleResizePersist(): void {
    if (options.isCollapseWindowSizeApplying() || isApplyingProgrammaticResize) {
      return;
    }

    clearResizePersistTimer();
    resizePersistTimer = window.setTimeout(() => {
      resizePersistTimer = null;
      persistResizeBounds();
    }, BOX_WINDOW_INTERACTION_TIMING.resizePersistSettleMs);
  }

  /**
   * 缩放静止或释放时保存最终边界，并同步给设置页与其他 Box 窗口
   */
  function persistResizeBounds(): void {
    clearResizePersistTimer();
    if (isResizingBox.value) {
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
   * 手写拖动从全局鼠标坐标开始，避免 Tauri 原生拖动在松手时回写旧位置
   */
  async function startManualDragging(event: MouseEvent): Promise<void> {
    if (!options.box.value || manualDragState) {
      return;
    }

    isManualDraggingBox.value = true;
    options.openCollapsedPreviewForActiveInteraction();
    const [windowPosition, windowSize, cursor, scaleFactor, monitor] = await Promise.all([
      options.currentWindow.outerPosition(),
      options.currentWindow.outerSize(),
      cursorPosition(),
      options.currentWindow.scaleFactor(),
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
   * 鼠标释放时停止拖动循环，并把最终物理坐标转换成逻辑坐标写入数据库
   */
  function stopManualDragging(shouldPersist: boolean): void {
    const dragState = manualDragState;

    manualDragState = null;
    isManualDraggingBox.value = false;
    manualDragCleanup?.();
    manualDragCleanup = null;
    options.refreshCollapsedPreviewCloseSchedule();

    if (shouldPersist && dragState) {
      void persistManualDragPosition(dragState.lastPosition);
    }
  }

  /**
   * 松手时用最后一次计算出的吸附坐标落库，移动过程中只改变窗口位置不写 SQLite
   */
  async function persistManualDragPosition(position: PhysicalWindowPoint): Promise<void> {
    await applyWindowPhysicalPosition(position.x, position.y, false);
    await persistWindowPositionFromPhysical(position.x, position.y);
  }

  /**
   * 释放监听同时挂在 window 和 document，确保窗口跟随鼠标移动时仍能收到 mouseup
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
   * 鼠标移动时按用户给出的 DOM 示例实时计算位置，只移动窗口不持久化数据库
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
   * 程序主动移动窗口时统一加锁，避免 setPosition 自己触发的 onMoved 被误判成外部移动
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
      await options.currentWindow.setPosition(new PhysicalPosition(x, y));
      if (shouldPersist) {
        await persistWindowPositionFromPhysical(x, y);
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
   * 拖动吸附实时参考其他 Box 的相邻边和屏幕工作区边缘，不做延迟二次定位
   */
  function resolveManualDragPosition(
    rawPosition: PhysicalWindowPoint,
    dragState: ManualDragState,
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
