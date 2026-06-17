import { computed, ref, type ComputedRef, type Ref } from "vue";
import type { CSSProperties } from "vue";
import { animate } from "motion";
import type { DesktopBox } from "@/entities/desktopBox/types";
import {
  BOX_COLLAPSE_INTERACTION,
  BOX_TITLE_OPACITY,
  BOX_TITLE_VISIBILITY,
  BOX_WINDOW_INTERACTION_TIMING,
} from "@/entities/desktopBox/layout";

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
 * 屏幕坐标换算结果只暴露 hover 判定所需字段，避免收缩逻辑反向依赖拖拽排序细节。
 */
interface BoxPointerLocalPoint {
  inside: boolean;
}

/**
 * Box 收缩预览组合式逻辑集中管理临时展开、收起动画和闲置透明度，不直接持久化 Box 数据。
 */
export function useBoxCollapsePreview(options: {
  applyWindowFrame: (frame: LogicalWindowFrame) => Promise<void>;
  box: ComputedRef<DesktopBox | undefined>;
  boxSurfaceRef: Ref<HTMLElement | null>;
  getBoxBackgroundOpacity: () => number;
  getBoxCollapseAnimationMs: () => number;
  getBoxCornerRadius: () => number;
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
  const isDragHoveringBox = ref(false);
  const isTitleHovered = ref(false);
  const isCollapseAnimating = ref(false);
  const boxSurfaceVisualHeight = ref<number | null>(null);
  const isBoxCollapsedToTitle = computed(() =>
    Boolean(options.box.value?.collapsed && !isCollapsedPreviewOpen.value),
  );
  const collapsedWindowHeight = computed(() => BOX_TITLE_VISIBILITY.expandedHeight);
  const boxIdleOpacity = computed(() =>
    isBoxHovered.value ||
    isDragHoveringBox.value ||
    options.isContextMenuOpen() ||
    options.isEditingTitle() ||
    options.isManualDraggingBox() ||
    options.isResizeHandleHovered() ||
    options.isResizingBox() ||
    isCollapsedPreviewOpen.value
      ? 1
      : (options.box.value?.titleOpacity ?? BOX_TITLE_OPACITY.max) / 100,
  );
  const boxSurfaceStyle = computed(
    () =>
      ({
        "--dasktop-box-background-opacity": `${options.getBoxBackgroundOpacity() / 100}`,
        "--dasktop-box-radius": `${options.getBoxCornerRadius()}px`,
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
      const isBottomTitle = options.box.value?.titlePosition === "bottom";
      const animatedHeight = boxSurfaceVisualHeight.value;
      const isCollapsingBottomTitle =
        isBottomTitle && (isCollapseAnimating.value || isBoxCollapsedToTitle.value);
      const bodyHeight =
        !isCollapsingBottomTitle
          ? undefined
          : Math.max(
              (animatedHeight ?? collapsedWindowHeight.value) -
                BOX_TITLE_VISIBILITY.expandedHeight,
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
  let isApplyingCollapseWindowSize = false;
  /**
   * 收缩动画可能被快速 hover 切换打断，版本号用于阻止旧动画的异步收尾覆盖新状态。
   */
  let collapseAnimationVersion = 0;
  let collapseAnimationTween: ReturnType<typeof animate> | null = null;
  let boxOpacityTween: ReturnType<typeof animate> | null = null;
  let collapsePreviewCloseTimer: ReturnType<typeof window.setTimeout> | null = null;
  let collapseSizeApplyLockTimer: ReturnType<typeof window.setTimeout> | null = null;

  /**
   * 根据收缩展示状态调整真实窗口高度，避免透明空白窗口挡住桌面点击。
   */
  async function applyCollapseWindowSize(shouldAnimate: boolean): Promise<void> {
    if (!options.box.value) {
      return;
    }

    cancelCollapseAnimationTween();
    collapseAnimationVersion += 1;
    const activeCollapseAnimationVersion = collapseAnimationVersion;

    const targetHeight = isBoxCollapsedToTitle.value
      ? collapsedWindowHeight.value
      : options.box.value.height;
    const targetWidth = options.box.value.width;
    const targetFrame = resolveCollapseWindowFrame(targetWidth, targetHeight);
    const animationMs = shouldAnimate ? options.getBoxCollapseAnimationMs() : 0;

    setCollapseSizeApplyLock(animationMs);

    if (!shouldAnimate) {
      boxSurfaceVisualHeight.value = null;
      await options.applyWindowFrame(targetFrame);
      return;
    }

    const currentWindowHeight = await options.resolveCurrentWindowHeight();
    const startHeight = boxSurfaceVisualHeight.value ?? currentWindowHeight;
    const heightDistance = targetHeight - startHeight;

    if (Math.abs(heightDistance) < 1) {
      boxSurfaceVisualHeight.value = null;
      await options.applyWindowFrame(targetFrame);
      return;
    }

    if (shouldReduceMotion()) {
      boxSurfaceVisualHeight.value = null;
      await options.applyWindowFrame(targetFrame);
      return;
    }

    isCollapseAnimating.value = true;
    boxSurfaceVisualHeight.value = startHeight;
    if (targetHeight > currentWindowHeight) {
      await options.applyWindowFrame(targetFrame);
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
    const surfaceElement = options.boxSurfaceRef.value;
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

    return Math.min(options.getBoxCollapseAnimationMs() * 0.36, 140) / 1000;
  }

  /**
   * 收缩态统一保留完整 Box 顶部的标题高度，即使标题配置在下方也向上收缩，避免视觉方向反转。
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
   * 收缩动画完成后一次性同步真实窗口高度，避免动画过程中暴露 Windows 原生直角边界。
   */
  function finishCollapseWindowResize(
    activeCollapseAnimationVersion: number,
    targetFrame: LogicalWindowFrame,
  ): void {
    boxSurfaceVisualHeight.value = targetFrame.height;
    void options.applyWindowFrame(targetFrame).finally(() => {
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
      animationMs + options.resizePersistSettleMs + 40,
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
    if (options.box.value?.collapsed) {
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
      options.isContextMenuOpen() ||
      options.isEditingTitle() ||
      options.isManualDraggingBox() ||
      options.isResizeHandleHovered() ||
      options.isResizingBox()
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
    const localPoint = await options.resolvePointerLocalPoint(screenX, screenY);

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
    if (!options.box.value?.collapsed || shouldKeepCollapsedPreviewOpen()) {
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

  return {
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
    isApplyingCollapseWindowSize: () => isApplyingCollapseWindowSize,
    isBoxCollapsedToTitle,
    isCollapseAnimating,
    openCollapsedPreviewForActiveInteraction,
    refreshCollapsedPreviewCloseSchedule,
    setDragHoveringBox,
    syncPointerHoverFromScreenPoint,
  };
}
