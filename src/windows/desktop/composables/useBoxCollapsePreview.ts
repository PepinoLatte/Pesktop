import { computed, onMounted, onUnmounted, ref, watch, type ComputedRef, type Ref } from "vue";
import type { CSSProperties } from "vue";
import { animate } from "motion";
import { broadcastBoxHoverTransfer, listenBoxHoverTransfer } from "@/shared/ipc/desktop";
import type { DesktopBox } from "@/entities/desktopBox/types";
import {
  BOX_COLLAPSE_INTERACTION,
  BOX_GRID_LAYOUT,
  BOX_TITLE_OPACITY,
  BOX_TITLE_VISIBILITY,
} from "@/entities/desktopBox/layout";

/**
 * 收缩动画最终写回原生窗口时同时包含位置和尺寸，标题在下方时需要用它保持标题视觉锚点
 */
interface LogicalWindowFrame {
  height: number;
  width: number;
  x: number;
  y: number;
}

/**
 * 屏幕坐标换算结果只暴露 hover 判定所需字段，避免收缩逻辑反向依赖拖拽排序细节
 */
interface BoxPointerLocalPoint {
  inside: boolean;
}

/**
 * Box 收缩预览组合式逻辑集中管理临时展开、收起动画和闲置透明度，不直接持久化 Box 数据
 */
export function useBoxCollapsePreview(options: {
  applyWindowFrame: (frame: LogicalWindowFrame) => Promise<void>;
  box: ComputedRef<DesktopBox | undefined>;
  boxSurfaceRef: Ref<HTMLElement | null>;
  getBoxBackgroundOpacity: () => number;
  getBoxBlurStrength?: () => number;
  getBoxCollapseAnimationMs: () => number;
  getBoxCollapseDelayMs: () => number;
  getBoxCornerRadius: () => number;
  getBoxIdleOpacityHideAnimationMs: () => number;
  getBoxIdleOpacityShowAnimationMs: () => number;
  isContextMenuOpen: () => boolean;
  isEditingTitle: () => boolean;
  isManualDraggingBox: () => boolean;
  isResizeHandleHovered: () => boolean;
  isResizingBox: () => boolean;
  resolveCurrentWindowHeight: () => Promise<number>;
  resolvePointerLocalPoint: (screenX: number, screenY: number) => Promise<BoxPointerLocalPoint>;
  resizePersistSettleMs: number;
}) {
  const isCollapsedPreviewOpen = ref(false);
  const isBoxHovered = ref(false);
  const isBoxConfirmed = ref(false);
  const isDragHoveringBox = ref(false);
  const isNativeItemContextMenuOpen = ref(false);
  const isTitleHovered = ref(false);
  const isCollapseAnimating = ref(false);
  const boxSurfaceVisualHeight = ref<number | null>(null);
  let unlistenHoverTransfer: (() => void) | null = null;

  const isBoxCollapsedToTitle = computed(() =>
    Boolean(options.box.value?.collapsed && !isCollapsedPreviewOpen.value),
  );
  const collapsedWindowHeight = computed(() => BOX_TITLE_VISIBILITY.expandedHeight);
  /**
   * 标题在下方时，内容区高度跟随可视高度变化，让标题自身从下往上收到顶部入口
   */
  const isBottomTitleMovingDuringCollapse = computed(() =>
    Boolean(
      options.box.value?.titlePosition === "bottom" &&
        options.box.value?.collapsed &&
        isCollapseAnimating.value,
    ),
  );
  const boxIdleOpacity = computed(() =>
    Boolean(options.box.value?.collapsed && isCollapsedPreviewOpen.value)
      ? 1
      : (options.box.value?.titleOpacity ?? BOX_TITLE_OPACITY.max) / 100,
  );
  const boxSurfaceStyle = computed(
    () =>
      ({
        "--dasktop-box-background-opacity": `${options.getBoxBackgroundOpacity() / 100}`,
        "--dasktop-box-radius": `${options.getBoxCornerRadius()}px`,
        "--dasktop-box-blur": `${options.getBoxBlurStrength ? options.getBoxBlurStrength() : 16}px`,
        borderRadius: "var(--dasktop-box-radius)",
        clipPath: "inset(0 round var(--dasktop-box-radius))",
        height: "100%",
        position: "relative",
        transition: "opacity 160ms cubic-bezier(0.16, 1, 0.3, 1)",
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
      const isCollapsingBottomTitle = isBottomTitleMovingDuringCollapse.value;
      const bodyHeight = !isCollapsingBottomTitle
        ? undefined
        : Math.max(
            (options.box.value?.height ?? 320) - BOX_TITLE_VISIBILITY.expandedHeight,
            0,
          );
      const verticalPadding =
        bodyHeight === undefined
          ? undefined
          : Math.min(BOX_GRID_LAYOUT.padding, bodyHeight / 2);

      return {
        flex: isCollapsingBottomTitle ? "0 0 auto" : undefined,
        height: bodyHeight === undefined ? undefined : `${bodyHeight}px`,
        minHeight: isCollapsingBottomTitle ? "0px" : undefined,
        opacity: isBoxCollapsedToTitle.value ? "0" : "1",
        paddingBottom: verticalPadding === undefined ? undefined : `${verticalPadding}px`,
        paddingTop: verticalPadding === undefined ? undefined : `${verticalPadding}px`,
        pointerEvents: isBoxCollapsedToTitle.value ? "none" : "auto",
        transition: "opacity 120ms ease-out",
      } as CSSProperties;
    },
  );
  /**
   * 收缩窗口高度动画期间隐藏内部滚动条；常态只允许纵向滚动，避免框选层或插入线制造横向滚动条。
   */
  const boxGridOverflowClass = computed(() =>
    isCollapseAnimating.value || isBoxCollapsedToTitle.value
      ? "overflow-hidden"
      : "overflow-x-hidden overflow-y-auto",
  );
  let isApplyingCollapseWindowSize = false;
  /**
   * 收缩动画可能被快速 hover 切换打断，版本号用于阻止旧动画的异步收尾覆盖新状态
   */
  let collapseAnimationVersion = 0;
  let collapseAnimationTween: ReturnType<typeof animate> | null = null;
  let boxOpacityTween: ReturnType<typeof animate> | null = null;
  let collapsePreviewCloseTimer: ReturnType<typeof window.setTimeout> | null = null;
  let collapseSizeApplyLockTimer: ReturnType<typeof window.setTimeout> | null = null;

  /**
   * 自动收起关闭后必须释放临时展开状态，否则旧的预览入口会持续把闲置透明度覆盖为完全可见
   */
  watch(
    () => options.box.value?.collapsed,
    (isCollapsed) => {
      if (isCollapsed) {
        return;
      }

      closeCollapsedPreview();
    },
  );

  /**
   * 根据收缩展示状态调整真实窗口高度，避免透明空白窗口挡住桌面点击
   */
  /**
   * 根据收缩展示状态调整真实窗口尺寸，直接写入目标尺寸并通过 CSS opacity 纯 GPU 淡入淡出，
   * 彻底根除因 frame-by-frame 频繁修改 DOM 高度导致的 Windows DWM 毛玻璃撕裂和刷新延迟。
   */
  async function applyCollapseWindowSize(shouldAnimate: boolean): Promise<void> {
    if (!options.box.value) {
      return;
    }

    cancelCollapseAnimationTween();
    collapseAnimationVersion += 1;

    const targetHeight = isBoxCollapsedToTitle.value
      ? collapsedWindowHeight.value
      : options.box.value.height;
    const targetWidth = options.box.value.width;
    const targetFrame = resolveCollapseWindowFrame(targetWidth, targetHeight);
    const animationMs = shouldAnimate ? options.getBoxCollapseAnimationMs() : 0;

    setCollapseSizeApplyLock(animationMs);
    boxSurfaceVisualHeight.value = null;
    isCollapseAnimating.value = false;
    await applyCollapseFrame(targetFrame);
  }

  /**
   * Box 闲置可见度切换：通过 CSS opacity 过渡平滑恢复，避免 JS 逐帧动画导致 CPU 波动与微抽动
   */
  function animateBoxIdleOpacity(_shouldAnimate: boolean): void {
    const surfaceElement = options.boxSurfaceRef.value;
    if (!surfaceElement) {
      return;
    }

    surfaceElement.style.opacity = String(boxIdleOpacity.value);
  }



  /**
   * 收缩态统一保留 Box 顶部标题入口，底部标题通过内部布局从下往上移动到这个入口
   */
  function resolveCollapseWindowFrame(width: number, height: number): LogicalWindowFrame {
    if (!options.box.value) {
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
      x: options.box.value.x,
      y: options.box.value.y,
    };
  }

  /**
   * 所有收缩尺寸写入都在这里记录最终高度，后续动画可用它判断当前窗口是完整态还是标题态
   */
  async function applyCollapseFrame(frame: LogicalWindowFrame): Promise<void> {
    await options.applyWindowFrame(frame);
  }



  /**
   * 收缩动画期间屏蔽 resize 落库，最终同步真实窗口高度时也不能覆盖用户保存的 Box 尺寸
   */
  function setCollapseSizeApplyLock(animationMs = 0): void {
    isApplyingCollapseWindowSize = true;
    if (collapseSizeApplyLockTimer) {
      window.clearTimeout(collapseSizeApplyLockTimer);
    }

    const lockMs = Math.max(
      BOX_COLLAPSE_INTERACTION.sizeApplyLockMs,
      animationMs + options.resizePersistSettleMs + 40,
    );
    collapseSizeApplyLockTimer = window.setTimeout(() => {
      isApplyingCollapseWindowSize = false;
      collapseSizeApplyLockTimer = null;
    }, lockMs);
  }

  /**
   * 取消未完成的 motion 收缩动画，用于快速 hover 切换或窗口卸载
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
   * 完整清理收缩动画和尺寸锁，避免窗口关闭后继续触发异步 setSize
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
   * 清理延迟收起计时器，所有进入 Box、菜单、拖动和缩放的交互都应先取消旧的收起任务
   */
  function clearCollapsedPreviewCloseTimer(): void {
    if (!collapsePreviewCloseTimer) {
      return;
    }

    window.clearTimeout(collapsePreviewCloseTimer);
    collapsePreviewCloseTimer = null;
  }

  /**
   * 收起预览状态只在自动收起模式内有效，退出该模式或延迟关闭时需要一起清理计时器和标记位
   */
  function closeCollapsedPreview(): void {
    clearCollapsedPreviewCloseTimer();
    isCollapsedPreviewOpen.value = false;
  }

  /**
   * 交互命中 Box 时使用同一套展开入口；拖拽命中也按普通鼠标进入处理，不再维护独立拖拽展开状态
   */
  function openCollapsedPreviewForActiveInteraction(): void {
    clearCollapsedPreviewCloseTimer();
    if (options.box.value?.collapsed) {
      isCollapsedPreviewOpen.value = true;
    }
  }

  /**
   * Windows Shell 右键菜单运行在原生消息循环中，打开期间 WebView 可能失焦或收到 mouseleave。
   * 将它纳入保持展开条件，避免系统菜单还在屏幕上时旧计时器提前收起 Box。
   */
  function setNativeItemContextMenuOpen(isOpen: boolean): void {
    if (isNativeItemContextMenuOpen.value === isOpen) {
      return;
    }

    isNativeItemContextMenuOpen.value = isOpen;
    if (isOpen) {
      openCollapsedPreviewForActiveInteraction();
      return;
    }

    refreshCollapsedPreviewCloseSchedule();
  }

  /**
   * 判断是否存在需要保持 Box 展开的交互，避免菜单、缩放、拖动过程中被 mouseleave 抢先收起
   */
  function shouldKeepCollapsedPreviewOpen(): boolean {
    return (
      isBoxHovered.value ||
      isDragHoveringBox.value ||
      isTitleHovered.value ||
      isNativeItemContextMenuOpen.value ||
      options.isContextMenuOpen() ||
      options.isEditingTitle() ||
      options.isManualDraggingBox() ||
      options.isResizeHandleHovered() ||
      options.isResizingBox()
    );
  }

  /**
   * 拖拽坐标命中独立于真实 mouseenter/mouseleave，避免拖拽结束后把普通 hover 状态卡住
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
   * 拖拽结束后用最终屏幕坐标恢复普通 hover 状态，避免拖拽 hover 残留或误清真实鼠标停留
   */
  async function syncPointerHoverFromScreenPoint(screenX: number, screenY: number): Promise<void> {
    const localPoint = await options.resolvePointerLocalPoint(screenX, screenY);

    isBoxHovered.value = localPoint.inside;
    if (localPoint.inside) {
      openCollapsedPreviewForActiveInteraction();
      return;
    }

    refreshCollapsedPreviewCloseSchedule();
  }

  /**
   * 鼠标离开后延迟收起，给用户从标题移动到边缘缩放或菜单窗口留出缓冲时间
   */
  function scheduleCollapsedPreviewClose(): void {
    clearCollapsedPreviewCloseTimer();
    if (!options.box.value?.collapsed || shouldKeepCollapsedPreviewOpen()) {
      return;
    }

    collapsePreviewCloseTimer = window.setTimeout(() => {
      collapsePreviewCloseTimer = null;
      if (!shouldKeepCollapsedPreviewOpen()) {
        closeCollapsedPreview();
      }
    }, options.getBoxCollapseDelayMs());
  }

  /**
   * 状态退出时统一刷新收起计划，保证透明度淡出只会在收缩动画真正启动后再延迟发生
   */
  function refreshCollapsedPreviewCloseSchedule(): void {
    if (shouldKeepCollapsedPreviewOpen()) {
      clearCollapsedPreviewCloseTimer();
      return;
    }

    scheduleCollapsedPreviewClose();
  }

  /**
   * 挂载全局跨 Box 悬浮激活转移监听：当鼠标进入其它 Box 时，若本 Box 尚未被点击确认激活，立即无感平滑让位收回
   */
  onMounted(() => {
    void listenBoxHoverTransfer((payload) => {
      if (!options.box.value || payload.activeBoxId === options.box.value.id) {
        return;
      }

      if (
        !isBoxConfirmed.value &&
        !options.isContextMenuOpen() &&
        !options.isEditingTitle() &&
        !options.isManualDraggingBox() &&
        !options.isResizingBox()
      ) {
        isBoxHovered.value = false;
        isTitleHovered.value = false;
        closeCollapsedPreview();
      }
    }).then((unlisten) => {
      unlistenHoverTransfer = unlisten;
    });
  });

  onUnmounted(() => {
    unlistenHoverTransfer?.();
    unlistenHoverTransfer = null;
  });

  /**
   * 用户在 Box 内点击、选框、重命名或展开菜单后，标记本 Box 为显式确认态，防止鼠标掠过邻近窗口时被强行切走
   */
  function confirmBoxActiveState(): void {
    isBoxConfirmed.value = true;
  }

  /**
   * Box 区域 hover 进入时取消延迟收起，并广播激活转移事件使其它未确认 Box 立即让位收起
   */
  function handleBoxMouseEnter(): void {
    isBoxHovered.value = true;
    openCollapsedPreviewForActiveInteraction();
    if (options.box.value) {
      void broadcastBoxHoverTransfer(options.box.value.id);
    }
  }

  /**
   * 标题 hover 是收缩 Box 的展开入口，内容是否展开仍由标题区域单独控制
   */
  function handleBoxTitleMouseEnter(): void {
    isTitleHovered.value = true;
    openCollapsedPreviewForActiveInteraction();
    if (options.box.value) {
      void broadcastBoxHoverTransfer(options.box.value.id);
    }
  }

  /**
   * 离开标题后只取消 roll-up 展开入口，内容收回仍等鼠标离开整个 Box
   */
  function handleBoxTitleMouseLeave(): void {
    isTitleHovered.value = false;
    refreshCollapsedPreviewCloseSchedule();
  }

  /**
   * 鼠标离开整个 Box 后延迟收回临时展开内容，并复位确认标记，使下次滑入能重新响应自由切换
   */
  function handleBoxMouseLeave(): void {
    isBoxHovered.value = false;
    isTitleHovered.value = false;
    isBoxConfirmed.value = false;
    refreshCollapsedPreviewCloseSchedule();
  }

  return {
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
    isApplyingCollapseWindowSize: () => isApplyingCollapseWindowSize,
    isBoxCollapsedToTitle,
    isBoxConfirmed,
    isCollapseAnimating,
    openCollapsedPreviewForActiveInteraction,
    refreshCollapsedPreviewCloseSchedule,
    setDragHoveringBox,
    setNativeItemContextMenuOpen,
    syncPointerHoverFromScreenPoint,
  };
}
