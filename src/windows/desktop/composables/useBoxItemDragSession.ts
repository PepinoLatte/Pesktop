import { computed, ref, type ComputedRef, type Ref } from "vue";
import type { CSSProperties } from "vue";
import { cursorPosition } from "@tauri-apps/api/window";
import { useDesktopStore } from "@/entities/desktopBox/store";
import type { DesktopBox } from "@/entities/desktopBox/types";
import type { DesktopItem } from "@/entities/desktopItem/types";
import {
  getDesktopItemsByPaths,
  isPrimaryMouseButtonPressed,
} from "@/entities/desktopItem/api";
import {
  notifyBoxItemDrag,
  notifyBoxItemDragAccepted,
  type BoxItemDragPayload,
  type BoxItemDragPreviewOptions,
} from "@/shared/ipc/boxItemDrag";
import { openDragPreviewWindow } from "@/windows/dragPreview/lifecycle";
import { BOX_ITEM_DRAG_INTERACTION } from "@/entities/desktopBox/layout";
import {
  resolveBoxGridDragInsertTarget,
  type BoxGridDragInsertTarget,
} from "../utils/dragGeometry";
import type {
  BoxItemDragLocalPoint,
  PhysicalWindowPoint,
} from "./useBoxWindowFrame";

/**
 * 跨窗口拖拽会话保存在来源 Box 中，用于轮询全局鼠标并在未被接收时执行拖出删除映射
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
 * 外部桌面文件拖入 Box 时没有来源 Box，会话只负责拖影和插入线接力，不参与拖出删除
 */
interface ExternalFileDragState {
  item: DesktopItem;
  preview: BoxItemDragPreviewOptions;
  sessionId: string;
  usesScreenPosition: boolean;
}

/**
 * 外部 Windows 拖拽取消时可延迟清理 hover，保证鼠标仍按下时 Box 不会收缩导致原生拖放链路抖动
 */
interface ExternalFileDragCancelOptions {
  deferHoverClearUntilRelease?: boolean;
}

/**
 * Box 图标拖拽会话组合式逻辑负责拖影、插入线、跨窗口 drop 和外部文件拖入
 */
export function useBoxItemDragSession(options: {
  box: ComputedRef<DesktopBox | undefined>;
  boxGridRef: Ref<HTMLElement | null>;
  closeContextMenu: () => void;
  refreshCollapsedPreviewCloseSchedule: () => void;
  resolveBoxItemDragLocalPoint: (
    screenX: number,
    screenY: number,
  ) => Promise<BoxItemDragLocalPoint>;
  resolveWindowClientPhysicalPointToScreen: (
    x: number,
    y: number,
  ) => Promise<PhysicalWindowPoint>;
  setDragHoveringBox: (isHovering: boolean) => void;
  setLastError: (message: string) => void;
  syncPointerHoverFromScreenPoint: (screenX: number, screenY: number) => Promise<void>;
}) {
  const desktopStore = useDesktopStore();
  const draggingBoxItemPath = ref<string | null>(null);
  const draggingBoxItemSessionId = ref<string | null>(null);
  const draggingBoxItemSourceBoxId = ref<string | null>(null);
  const dragInsertLineStyle = ref<CSSProperties | null>(null);
  /**
   * 只有 Box 图标拖拽需要临时关闭点击和缩放；Windows 外部文件拖入只展示拖影和落点，避免原生 leave 竞态卡住交互
   */
  const isBoxItemDragActive = computed(() =>
    Boolean(draggingBoxItemPath.value && draggingBoxItemSourceBoxId.value),
  );
  let boxItemGlobalDragState: BoxItemGlobalDragState | null = null;
  let boxItemDragPollTimer: ReturnType<typeof window.setInterval> | null = null;
  let isBoxItemDragPollTickPending = false;
  let externalFileDragState: ExternalFileDragState | null = null;
  /**
   * 外部拖拽 enter 需要异步解析文件信息；版本号用于让已经 leave 的旧 enter 结果失效
   */
  let externalFileDragRequestVersion = 0;
  let externalFileDragReleaseProbeTimer: ReturnType<typeof window.setInterval> | null = null;
  let externalFileDragReleaseProbeStartedAt = 0;
  let externalFileDragReleaseSessionId = "";
  const acceptedBoxItemDragSessions = new Set<string>();
  const ignoredExternalDragSessions = new Set<string>();
  const settledBoxItemDragSessions = new Set<string>();

  /**
   * Box 图标 pointer 拖拽开始时创建跨窗口会话，并启动全局鼠标轮询和拖影窗口
   */
  function startBoxItemPointerDrag(event: PointerEvent, itemPath: string): void {
    draggingBoxItemPath.value = itemPath;
    draggingBoxItemSourceBoxId.value = options.box.value?.id ?? null;
    clearBoxItemDragIndicator();
    options.closeContextMenu();
    void beginBoxItemGlobalDrag(event, itemPath);
  }

  /**
   * 外部文件拖入 Box 时声明当前区域可接收文件；内部排序已改用 pointer 拖拽
   */
  function handleBoxGridDragOver(event: DragEvent): void {
    options.setDragHoveringBox(true);
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = "copy";
    }
  }

  /**
   * DOM drop 只处理外部文件路径；Box 内部排序由 pointer up 提交，避免浏览器 DnD 禁用光标
   */
  async function handleBoxGridDrop(event: DragEvent): Promise<void> {
    if (!options.box.value || externalFileDragState) {
      return;
    }

    await desktopStore.handleItemDrop(event, options.box.value.id);
  }

  /**
   * pointer 释放时只通知全局拖拽会话结束；实际排序由命中的 Box 处理 Drop 事件
   */
  async function finishBoxItemPointerDrag(
    event: PointerEvent,
    itemPath: string,
  ): Promise<void> {
    if (draggingBoxItemPath.value !== itemPath) {
      return;
    }

    await finishBoxItemGlobalDrag(event);
  }

  /**
   * 清理排序提示线，避免拖拽离开窗口或落到空白区后保留旧位置
   */
  function clearBoxItemDragIndicator(): void {
    dragInsertLineStyle.value = null;
  }

  /**
   * 拖拽来源状态必须和路径一起清理，避免旧来源 Box 让后续外部拖入误隐藏同名项目
   */
  function clearDraggingBoxItemState(): void {
    draggingBoxItemPath.value = null;
    draggingBoxItemSessionId.value = null;
    draggingBoxItemSourceBoxId.value = null;
  }

  /**
   * 清理拖拽视觉状态时校验 session，避免旧窗口的 cancel/drop 事件覆盖新的同路径拖拽
   */
  function isCurrentBoxItemDragPayload(payload: BoxItemDragPayload): boolean {
    return (
      draggingBoxItemPath.value === payload.item.path &&
      (!draggingBoxItemSessionId.value || draggingBoxItemSessionId.value === payload.sessionId)
    );
  }

  /**
   * 记录当前窗口正在展示的拖拽载荷；外部文件拖入没有来源 Box，不能驱动禁用点击和缩放的状态
   */
  function applyDraggingBoxItemPayload(payload: BoxItemDragPayload): void {
    draggingBoxItemPath.value = payload.item.path;
    draggingBoxItemSessionId.value = payload.sessionId;
    draggingBoxItemSourceBoxId.value = payload.sourceBoxId || null;
  }

  /**
   * 应用当前拖拽插入目标；目标为空时保留 Drop 到末尾的语义但隐藏竖线
   */
  function applyBoxItemDragIndicator(insertTarget: BoxGridDragInsertTarget | null): void {
    dragInsertLineStyle.value = insertTarget?.indicatorStyle ?? null;
  }

  /**
   * 外部文件进入 Box 时先解析第一项作为拖影代表，其余路径在 Drop 时一起收纳
   */
  async function beginExternalFileDrag(
    paths: string[],
    x: number,
    y: number,
    isScreenPosition = false,
  ): Promise<void> {
    const requestVersion = externalFileDragRequestVersion + 1;
    externalFileDragRequestVersion = requestVersion;
    const acceptedPaths = paths.filter(Boolean);
    if (acceptedPaths.length === 0) {
      return;
    }

    clearExternalFileDragReleaseProbe();
    const previewItems = await resolveExternalDragPreviewItems(acceptedPaths);
    if (externalFileDragRequestVersion !== requestVersion) {
      return;
    }

    const firstItem = previewItems[0];
    if (!firstItem) {
      return;
    }

    const screenPoint = isScreenPosition
      ? { x, y }
      : await options.resolveWindowClientPhysicalPointToScreen(x, y);
    options.setDragHoveringBox(true);
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
        options.setLastError(error instanceof Error ? error.message : String(error));
      });
    await emitExternalFileDragPhase("start", screenPoint.x, screenPoint.y, sessionId);
  }

  /**
   * 外部文件悬停时持续广播坐标，目标 Box 用同一套逻辑绘制插入线
   */
  async function moveExternalFileDrag(x: number, y: number): Promise<void> {
    const dragState = externalFileDragState;
    if (!dragState) {
      return;
    }

    const screenPoint = dragState.usesScreenPosition
      ? { x, y }
      : await options.resolveWindowClientPhysicalPointToScreen(x, y);
    await emitExternalFileDragPhase("move", screenPoint.x, screenPoint.y, dragState.sessionId);
  }

  /**
   * 外部文件 Drop 后按当前插入点写入 Box，随后关闭拖影
   */
  async function finishExternalFileDrag(
    paths: string[],
    x: number,
    y: number,
  ): Promise<void> {
    if (!options.box.value) {
      cancelExternalFileDrag();
      return;
    }

    const acceptedPaths = paths.filter(Boolean);
    if (acceptedPaths.length === 0) {
      cancelExternalFileDrag();
      return;
    }

    const dragState =
      externalFileDragState ?? (await createExternalFileDragStateForDrop(acceptedPaths));

    if (!dragState) {
      return;
    }

    externalFileDragState = dragState;
    const screenPoint = dragState.usesScreenPosition
      ? { x, y }
      : await options.resolveWindowClientPhysicalPointToScreen(x, y);
    const localPoint = await options.resolveBoxItemDragLocalPoint(screenPoint.x, screenPoint.y);
    const insertTarget =
      localPoint.inside && options.boxGridRef.value
        ? resolveBoxGridDragInsertTarget(
            localPoint.x,
            localPoint.y,
            options.boxGridRef.value,
            dragState.item.path,
          )
        : null;

    await desktopStore.assignDroppedPathsToBox(acceptedPaths, options.box.value.id);
    if (insertTarget) {
      await desktopStore.reorderBoxItem(
        options.box.value.id,
        dragState.item.path,
        insertTarget.path,
        insertTarget.placement,
      );
    }
    markBoxItemDragSessionSettled(dragState.sessionId);
    await emitExternalFileDragPhase("drop", screenPoint.x, screenPoint.y, dragState.sessionId);
    ignoreExternalDragSession(dragState.sessionId);
    externalFileDragState = null;
    clearExternalFileDragReleaseProbe();
    options.setDragHoveringBox(false);
    await options.syncPointerHoverFromScreenPoint(screenPoint.x, screenPoint.y);
    options.refreshCollapsedPreviewCloseSchedule();
    clearBoxItemDragIndicator();
  }

  /**
   * 外部拖拽离开窗口时关闭拖影和插入线，真实文件不做任何处理
   */
  function cancelExternalFileDrag(optionsValue: ExternalFileDragCancelOptions = {}): void {
    externalFileDragRequestVersion += 1;
    const dragState = externalFileDragState;
    externalFileDragState = null;
    clearDraggingBoxItemState();
    if (dragState) {
      markBoxItemDragSessionSettled(dragState.sessionId);
      ignoreExternalDragSession(dragState.sessionId);
    }
    if (optionsValue.deferHoverClearUntilRelease && dragState) {
      startExternalFileDragReleaseProbe(dragState.sessionId);
    } else {
      clearExternalFileDragReleaseProbe();
      options.setDragHoveringBox(false);
      options.refreshCollapsedPreviewCloseSchedule();
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
   * 原生拖拽 leave 后鼠标仍处于按下态，保持临时展开直到释放，避免收缩 setSize 让 WebView 拖放状态卡死
   */
  function startExternalFileDragReleaseProbe(sessionId: string): void {
    externalFileDragReleaseSessionId = sessionId;
    externalFileDragReleaseProbeStartedAt = performance.now();
    clearExternalFileDragReleaseProbe(false);
    options.setDragHoveringBox(true);
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
   * 外部拖拽释放兜底只清理自己的会话，防止旧 leave 的轮询把新拖拽 hover 状态误关闭
   */
  function finishExternalFileDragReleaseProbe(sessionId: string): void {
    if (externalFileDragReleaseSessionId !== sessionId) {
      return;
    }

    clearExternalFileDragReleaseProbe();
    options.setDragHoveringBox(false);
    options.refreshCollapsedPreviewCloseSchedule();
  }

  /**
   * 清理外部拖拽释放轮询；保留会话时用于先停旧 timer 再启动新 timer
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
   * Windows 原生拖放的 move/cancel 到达顺序不稳定；已取消 session 的晚到 move 必须忽略
   */
  function ignoreExternalDragSession(sessionId: string): void {
    ignoredExternalDragSessions.add(sessionId);
    window.setTimeout(() => {
      ignoredExternalDragSessions.delete(sessionId);
    }, BOX_ITEM_DRAG_INTERACTION.externalReleaseFallbackMs);
  }

  /**
   * Drop/cancel 到达后可能仍有旧的 start/move 处理函数卡在异步坐标换算中；结束标记用于阻止旧事件回写交互状态
   */
  function markBoxItemDragSessionSettled(sessionId: string): void {
    settledBoxItemDragSessions.add(sessionId);
    window.setTimeout(() => {
      settledBoxItemDragSessions.delete(sessionId);
    }, BOX_ITEM_DRAG_INTERACTION.externalReleaseFallbackMs);
  }

  /**
   * 已结束会话只拦截非终态事件，Drop/cancel 本身仍需要进入清理分支完成收尾
   */
  function isSettledBoxItemDragMovePayload(payload: BoxItemDragPayload): boolean {
    return (
      settledBoxItemDragSessions.has(payload.sessionId) &&
      (payload.phase === "start" || payload.phase === "move")
    );
  }

  /**
   * 外部原生拖放和 IPC 事件顺序不稳定；已取消或已结束的旧载荷不能再改变 Box 视觉状态
   */
  function shouldIgnoreBoxItemDragPayload(payload: BoxItemDragPayload): boolean {
    return (
      isSettledBoxItemDragMovePayload(payload) ||
      (!payload.sourceBoxId &&
        payload.phase !== "cancel" &&
        ignoredExternalDragSessions.has(payload.sessionId))
    );
  }

  /**
   * 清理旧载荷遗留的插入线和拖拽标记；等待鼠标释放时保留 hover，避免 Box 收缩打断 Windows 原生拖放
   */
  function clearIgnoredBoxItemDragPayloadVisuals(payload: BoxItemDragPayload): void {
    const shouldClearPayloadVisuals =
      !draggingBoxItemSessionId.value || draggingBoxItemSessionId.value === payload.sessionId;
    const isWaitingForExternalRelease =
      !payload.sourceBoxId && externalFileDragReleaseSessionId === payload.sessionId;

    if (shouldClearPayloadVisuals && isCurrentBoxItemDragPayload(payload)) {
      clearDraggingBoxItemState();
    }
    if (!shouldClearPayloadVisuals) {
      return;
    }

    clearBoxItemDragIndicator();
    if (isWaitingForExternalRelease) {
      return;
    }

    options.setDragHoveringBox(false);
    options.refreshCollapsedPreviewCloseSchedule();
  }

  /**
   * Drop 事件可能先于 enter 状态初始化到达，此时临时创建一次会话以复用排序流程
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
   * 外部拖入只解析第一条路径作为拖影代表，避免为了预览提前写入 Box 映射
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
   * 外部文件拖影复用 Box 内部拖拽事件，sourceBoxId 为空表示不会触发拖出删除
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
   * 拖影沿用当前 Box 的展示配置，保持拖动中的图标大小、文字和圆角与目标工作区一致
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
   * 来源 Box 创建拖拽会话后持续广播屏幕坐标，目标 Box 不需要依赖浏览器原生 DnD
   */
  async function beginBoxItemGlobalDrag(event: PointerEvent, itemPath: string): Promise<void> {
    if (!options.box.value || boxItemGlobalDragState) {
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
      sourceBoxId: options.box.value.id,
    };
    acceptedBoxItemDragSessions.delete(sessionId);
    void openDragPreviewWindow()
      .then(() => emitBoxItemDragPhase("move", cursor.x, cursor.y))
      .catch((error) => {
        options.setLastError(error instanceof Error ? error.message : String(error));
      });
    await emitBoxItemDragPhase("start", cursor.x, cursor.y);
    startBoxItemDragPolling();
  }

  /**
   * 全局鼠标轮询让拖拽在离开当前 WebView 后仍能更新拖影和目标 Box 插入线
   */
  function startBoxItemDragPolling(): void {
    clearBoxItemDragPolling();
    boxItemDragPollTimer = window.setInterval(() => {
      void pollBoxItemDragCursor();
    }, BOX_ITEM_DRAG_INTERACTION.pollIntervalMs);
  }

  /**
   * 每一帧读取鼠标位置和左键状态；左键释放时统一派发 Drop
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
   * 清理拖拽轮询定时器，避免 Drop 结束后继续发送旧坐标
   */
  function clearBoxItemDragPolling(): void {
    if (!boxItemDragPollTimer) {
      return;
    }

    window.clearInterval(boxItemDragPollTimer);
    boxItemDragPollTimer = null;
  }

  /**
   * pointerup 仍在当前窗口内时使用实时鼠标坐标结束会话，避免等待下一次轮询
   */
  async function finishBoxItemGlobalDrag(event: PointerEvent): Promise<void> {
    const cursor = await cursorPosition().catch(() => ({
      x: Math.round(event.screenX),
      y: Math.round(event.screenY),
    }));

    await finishBoxItemGlobalDragAt(cursor.x, cursor.y);
  }

  /**
   * 结束拖拽时先广播 Drop，再给目标窗口一个短暂提交窗口；无人接收才按拖出 Box 删除映射
   */
  async function finishBoxItemGlobalDragAt(screenX: number, screenY: number): Promise<void> {
    const dragState = boxItemGlobalDragState;
    if (!dragState || dragState.finishing) {
      return;
    }

    dragState.finishing = true;
    clearBoxItemDragPolling();
    markBoxItemDragSessionSettled(dragState.sessionId);
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
   * 来源窗口销毁或异常结束时取消拖影，避免残留一个始终置顶的小透明窗口
   */
  function cancelActiveBoxItemDrag(): void {
    const dragState = boxItemGlobalDragState;
    clearBoxItemDragPolling();
    clearDraggingBoxItemState();
    clearBoxItemDragIndicator();
    boxItemGlobalDragState = null;
    options.refreshCollapsedPreviewCloseSchedule();

    if (dragState) {
      markBoxItemDragSessionSettled(dragState.sessionId);
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
   * 广播拖拽阶段时复用当前会话数据，保证预览窗和所有 Box 看到同一个 sessionId
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
   * 如果没有任何 Box 回执接收 Drop，就按用户拖出 Box 处理，只删除映射不动真实文件
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
   * 所有 Box 都监听拖拽事件，但只有鼠标落入当前窗口时才展示插入线或接收 Drop
   */
  async function handleBoxItemDragPayload(payload: BoxItemDragPayload): Promise<void> {
    if (!payload.sourceBoxId && payload.phase === "cancel") {
      ignoreExternalDragSession(payload.sessionId);
    }

    if (payload.phase === "cancel") {
      markBoxItemDragSessionSettled(payload.sessionId);
    }

    if (shouldIgnoreBoxItemDragPayload(payload)) {
      clearIgnoredBoxItemDragPayloadVisuals(payload);
      return;
    }

    if (!options.box.value || payload.phase === "cancel") {
      if (isCurrentBoxItemDragPayload(payload)) {
        clearDraggingBoxItemState();
      }
      const isWaitingForExternalRelease =
        !payload.sourceBoxId && externalFileDragReleaseSessionId === payload.sessionId;
      if (!isWaitingForExternalRelease) {
        options.setDragHoveringBox(false);
      }
      clearBoxItemDragIndicator();
      if (payload.phase === "cancel" && !isWaitingForExternalRelease) {
        options.refreshCollapsedPreviewCloseSchedule();
      }
      return;
    }

    const localPoint = await options.resolveBoxItemDragLocalPoint(
      payload.screenX,
      payload.screenY,
    );
    if (payload.phase === "drop") {
      markBoxItemDragSessionSettled(payload.sessionId);
    }
    if (shouldIgnoreBoxItemDragPayload(payload)) {
      clearIgnoredBoxItemDragPayloadVisuals(payload);
      return;
    }

    if (!localPoint.inside) {
      if (isCurrentBoxItemDragPayload(payload)) {
        clearDraggingBoxItemState();
      }
      options.setDragHoveringBox(false);
      clearBoxItemDragIndicator();
      options.refreshCollapsedPreviewCloseSchedule();
      return;
    }

    if (payload.phase !== "drop") {
      options.setDragHoveringBox(true);
    }
    const insertTarget = options.boxGridRef.value
      ? resolveBoxGridDragInsertTarget(
          localPoint.x,
          localPoint.y,
          options.boxGridRef.value,
          payload.item.path,
        )
      : null;

    applyBoxItemDragIndicator(insertTarget);
    if (payload.phase !== "drop") {
      applyDraggingBoxItemPayload(payload);
      return;
    }

    if (!payload.sourceBoxId) {
      if (isCurrentBoxItemDragPayload(payload)) {
        clearDraggingBoxItemState();
      }
      clearBoxItemDragIndicator();
      options.setDragHoveringBox(false);
      await options.syncPointerHoverFromScreenPoint(payload.screenX, payload.screenY);
      options.refreshCollapsedPreviewCloseSchedule();
      return;
    }

    await commitBoxItemDragDrop(payload, insertTarget);
    clearDraggingBoxItemState();
    clearBoxItemDragIndicator();
    await notifyBoxItemDragAccepted({
      sessionId: payload.sessionId,
      targetBoxId: options.box.value.id,
    });
    options.setDragHoveringBox(false);
    await options.syncPointerHoverFromScreenPoint(payload.screenX, payload.screenY);
    options.refreshCollapsedPreviewCloseSchedule();
  }

  /**
   * Drop 命中当前 Box 时按目标决定是本 Box 排序、移动到末尾，还是跨 Box 重新收纳
   */
  async function commitBoxItemDragDrop(
    payload: BoxItemDragPayload,
    insertTarget: BoxGridDragInsertTarget | null,
  ): Promise<void> {
    if (!options.box.value) {
      return;
    }

    if (payload.sourceBoxId === options.box.value.id) {
      if (insertTarget) {
        await desktopStore.reorderBoxItem(
          options.box.value.id,
          payload.item.path,
          insertTarget.path,
          insertTarget.placement,
        );
        return;
      }

      await desktopStore.moveBoxItemToEnd(options.box.value.id, payload.item.path);
      return;
    }

    await desktopStore.assignDroppedPathsToBox([payload.item.path], options.box.value.id);
    if (insertTarget) {
      await desktopStore.reorderBoxItem(
        options.box.value.id,
        payload.item.path,
        insertTarget.path,
        insertTarget.placement,
      );
    }
  }

  /**
   * 目标 Box 回执会同步给来源拖拽会话，防止拖到其他 Box 时被误判为拖出删除
   */
  function acceptBoxItemDragSession(sessionId: string): void {
    acceptedBoxItemDragSessions.add(sessionId);
    if (boxItemGlobalDragState?.sessionId === sessionId) {
      boxItemGlobalDragState.accepted = true;
    }
  }

  return {
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
  };
}
