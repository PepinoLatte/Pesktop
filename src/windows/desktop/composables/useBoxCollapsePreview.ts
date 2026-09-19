import { computed, ref, watch, type ComputedRef, type Ref } from "vue";
import type { CSSProperties } from "vue";
import { animate } from "motion";
import type { BoxCollapseMode, DesktopBox } from "@/entities/desktopBox/types";
import {
  BOX_COLLAPSE_INTERACTION,
  BOX_GRID_LAYOUT,
  BOX_ICON_STATE_SIZE,
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
  getBoxCollapseAnimationMs: () => number;
  getBoxCollapseDelayMs: () => number;
  getBoxCollapseMode: () => BoxCollapseMode;
  getBoxCornerRadius: () => number;
  getBoxExpandHoverDelayMs: () => number;
  getBoxIconFadeInMs: () => number;
  getBoxIconFadeOutMs: () => number;
  getBoxIdleOpacityHideAnimationMs: () => number;
  getBoxIdleOpacityShowAnimationMs: () => number;
  isContextMenuOpen: () => boolean;
  isEditingTitle: () => boolean;
  isManualDraggingBox: () => boolean;
  isResizeHandleHovered: () => boolean;
  isResizingBox: () => boolean;
  resolveCurrentWindowHeight: () => Promise<number>;
  resolveCurrentWindowWidth: () => Promise<number>;
  resolvePointerLocalPoint: (screenX: number, screenY: number) => Promise<BoxPointerLocalPoint>;
  resizePersistSettleMs: number;
}) {
  const isCollapsedPreviewOpen = ref(false);
  const isBoxHovered = ref(false);
  const isDragHoveringBox = ref(false);
  const isNativeItemContextMenuOpen = ref(false);
  const isTitleHovered = ref(false);
  const isCollapseAnimating = ref(false);
  /**
   * 收缩或展开动画期间隐藏标题与内容，动画只呈现面板缩放本身；
   * 完成后内容统一淡入，避免网格重排和形态切换暴露在中途帧里
   */
  const isCollapseContentFaded = ref(false);
  const boxSurfaceVisualHeight = ref<number | null>(null);
  const boxSurfaceVisualWidth = ref<number | null>(null);
  const isBoxCollapsedToTitle = computed(() =>
    Boolean(options.box.value?.collapsed && !isCollapsedPreviewOpen.value),
  );
  /**
   * 图标态是收缩闲置形态中的图标模式：窗口收敛到单图标方块并渲染图标入口
   */
  const isBoxInIconState = computed(
    () => isBoxCollapsedToTitle.value && options.getBoxCollapseMode() === "icon",
  );
  /**
   * 正在从图标态向完整面板展开的过渡期：图标平滑淡出，面板同步平滑变形
   */
  const isBoxExpandingFromIcon = computed(
    () =>
      isCollapseAnimating.value &&
      !isBoxCollapsedToTitle.value &&
      options.getBoxCollapseMode() === "icon",
  );
  /**
   * 窗口模式的闲置形态保留标题条入口（40px），图标模式收敛到单图标方块边长
   */
  const collapsedWindowHeight = computed(() =>
    options.getBoxCollapseMode() === "icon"
      ? BOX_ICON_STATE_SIZE
      : BOX_TITLE_VISIBILITY.expandedHeight,
  );
  const collapsedWindowWidth = computed(() =>
    options.getBoxCollapseMode() === "icon"
      ? BOX_ICON_STATE_SIZE
      : options.box.value?.width ?? BOX_ICON_STATE_SIZE,
  );
  /**
   * 标题在下方时，内容区高度跟随可视高度变化，让标题自身从下往上收到顶部入口
   */
  const isBottomTitleMovingDuringCollapse = computed(() =>
    Boolean(
      options.box.value?.titlePosition === "bottom" &&
        (isCollapseAnimating.value || isBoxCollapsedToTitle.value),
    ),
  );
  const boxIdleOpacity = computed(() =>
    isBoxHovered.value ||
    isDragHoveringBox.value ||
    isNativeItemContextMenuOpen.value ||
    options.isContextMenuOpen() ||
    options.isEditingTitle() ||
    options.isManualDraggingBox() ||
    options.isResizeHandleHovered() ||
    options.isResizingBox() ||
    Boolean(options.box.value?.collapsed && isCollapsedPreviewOpen.value)
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
        width: boxSurfaceVisualWidth.value === null ? "100%" : `${boxSurfaceVisualWidth.value}px`,
      }) as CSSProperties,
  );
  const boxTitleAreaStyle = computed(
    () =>
      ({
        height: `${BOX_TITLE_VISIBILITY.expandedHeight}px`,
        opacity: isCollapseContentFaded.value ? "0" : "1",
        transition: `opacity ${options.getBoxIconFadeInMs()}ms ease-out`,
      }) as CSSProperties,
  );
  const boxBodyStyle = computed(
    () => {
      const animatedHeight = boxSurfaceVisualHeight.value;
      const isCollapsingBottomTitle = isBottomTitleMovingDuringCollapse.value;
      const bodyHeight =
        !isCollapsingBottomTitle
          ? undefined
          : Math.max(
              (animatedHeight ?? collapsedWindowHeight.value) -
                BOX_TITLE_VISIBILITY.expandedHeight,
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
        opacity: isCollapseContentFaded.value || isBoxCollapsedToTitle.value ? "0" : "1",
        paddingBottom: verticalPadding === undefined ? undefined : `${verticalPadding}px`,
        paddingTop: verticalPadding === undefined ? undefined : `${verticalPadding}px`,
        pointerEvents: isBoxCollapsedToTitle.value ? "none" : "auto",
        transform: isBoxCollapsedToTitle.value ? "translateY(-6px)" : "translateY(0)",
        transition: `opacity ${options.getBoxIconFadeInMs()}ms ease-out`,
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
  let expandHoverTimer: ReturnType<typeof window.setTimeout> | null = null;
  let collapseSizeApplyLockTimer: ReturnType<typeof window.setTimeout> | null = null;
  let lastAppliedWindowHeight: number | null = null;
  let lastAppliedWindowWidth: number | null = null;

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
   * 菜单里切换收缩形态后，已处于收缩闲置的 Box 需要立即按新模式重新适配窗口尺寸
   */
  watch(
    () => options.box.value?.collapseMode,
    () => {
      if (options.box.value?.collapsed && !isCollapsedPreviewOpen.value) {
        void applyCollapseWindowSize(true);
      }
    },
  );

  /**
   * 根据收缩展示状态调整真实窗口高度，避免透明空白窗口挡住桌面点击
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
    const targetWidth = isBoxCollapsedToTitle.value
      ? collapsedWindowWidth.value
      : options.box.value.width;
    const targetFrame = resolveCollapseWindowFrame(targetWidth, targetHeight);
    const animationMs = shouldAnimate ? options.getBoxCollapseAnimationMs() : 0;
    const canAnimate = shouldAnimate && !shouldReduceMotion();

    setCollapseSizeApplyLock(animationMs);

    if (!canAnimate) {
      boxSurfaceVisualHeight.value = null;
      boxSurfaceVisualWidth.value = null;
      isCollapseAnimating.value = false;
      isCollapseContentFaded.value = false;
      await applyCollapseFrame(targetFrame);
      return;
    }

    // 动画期间先淡出标题和内容，只呈现面板缩放本身；完成后统一淡入，掩盖中途重排
    isCollapseContentFaded.value = true;
    boxSurfaceVisualHeight.value = resolveOptimisticAnimationStartHeight();
    isCollapseAnimating.value = true;
    const [currentWindowHeight, currentWindowWidth] = await Promise.all([
      options.resolveCurrentWindowHeight(),
      options.resolveCurrentWindowWidth(),
    ]);
    const startHeight = boxSurfaceVisualHeight.value ?? currentWindowHeight;
    const startWidth = resolveOptimisticAnimationStartWidth();
    const heightDistance = targetHeight - startHeight;
    const widthDistance = targetWidth - startWidth;

    if (Math.abs(heightDistance) < 1 && Math.abs(widthDistance) < 1) {
      boxSurfaceVisualHeight.value = null;
      boxSurfaceVisualWidth.value = null;
      isCollapseAnimating.value = false;
      isCollapseContentFaded.value = false;
      await applyCollapseFrame(targetFrame);
      return;
    }

    boxSurfaceVisualHeight.value = startHeight;
    boxSurfaceVisualWidth.value = startWidth;
    if (targetHeight > currentWindowHeight || targetWidth > currentWindowWidth) {
      await applyCollapseFrame(targetFrame);
    }

    const tweenState = {
      height: startHeight,
      width: startWidth,
    };
    collapseAnimationTween = animate(
      tweenState,
      {
        height: targetHeight,
        width: targetWidth,
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
          boxSurfaceVisualWidth.value = Math.round(tweenState.width);
        },
      },
    );
  }

  /**
   * motion 驱动 Box 闲置可见度，hover 进入时即使配置为 0 也能平滑恢复到完全可见
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
        duration: resolveBoxIdleOpacityAnimationDuration(),
        ease: [0.16, 1, 0.3, 1],
        onComplete: () => {
          boxOpacityTween = null;
        },
      },
    );
  }

  /**
   * 减少动态效果时直接应用终态，遵守系统辅助功能设置
   */
  function shouldReduceMotion(): boolean {
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  }

  /**
   * 收缩触发闲置透明时延后淡出，让用户先感知 Box 收回动作，再看到透明度过渡
   */
  function resolveBoxIdleOpacityAnimationDelay(): number {
    if (boxIdleOpacity.value >= 1 || !isBoxCollapsedToTitle.value) {
      return 0;
    }

    return Math.min(options.getBoxCollapseAnimationMs() * 0.2, 140) / 1000;
  }

  /**
   * 闲置隐藏比 hover 显示略慢，保留柔和淡出；显示仍保持短时长以确保指针进入后立即可操作
   */
  function resolveBoxIdleOpacityAnimationDuration(): number {
    const durationMs =
      boxIdleOpacity.value >= 1
        ? options.getBoxIdleOpacityShowAnimationMs()
        : options.getBoxIdleOpacityHideAnimationMs();

    return durationMs / 1000;
  }

  /**
   * 状态切换到“只保留标题”后，Vue 会先计算一次布局；提前给出动画起点可避免底部标题先塌到顶部再弹回
   */
  function resolveOptimisticAnimationStartHeight(): number {
    const currentBox = options.box.value;

    if (!currentBox) {
      return collapsedWindowHeight.value;
    }

    if (boxSurfaceVisualHeight.value !== null) {
      return boxSurfaceVisualHeight.value;
    }

    if (isBoxCollapsedToTitle.value) {
      return lastAppliedWindowHeight !== null &&
        lastAppliedWindowHeight <= collapsedWindowHeight.value + 1
        ? lastAppliedWindowHeight
        : currentBox.height;
    }

    if (
      currentBox.collapsed &&
      lastAppliedWindowHeight !== null &&
      lastAppliedWindowHeight <= collapsedWindowHeight.value + 1
    ) {
      return lastAppliedWindowHeight;
    }

    return currentBox.height;
  }

  /**
   * 宽度动画起点与高度同构：图标态收敛到固定边长，展开时再回到用户保存的宽度
   */
  function resolveOptimisticAnimationStartWidth(): number {
    const currentBox = options.box.value;

    if (!currentBox) {
      return collapsedWindowWidth.value;
    }

    if (boxSurfaceVisualWidth.value !== null) {
      return boxSurfaceVisualWidth.value;
    }

    if (isBoxCollapsedToTitle.value) {
      return lastAppliedWindowWidth !== null &&
        lastAppliedWindowWidth <= collapsedWindowWidth.value + 1
        ? lastAppliedWindowWidth
        : currentBox.width;
    }

    if (
      currentBox.collapsed &&
      lastAppliedWindowWidth !== null &&
      lastAppliedWindowWidth <= collapsedWindowWidth.value + 1
    ) {
      return lastAppliedWindowWidth;
    }

    return currentBox.width;
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
   * 所有收缩尺寸写入都在这里记录最终宽高，后续动画可用它判断当前窗口是完整态还是图标态
   */
  async function applyCollapseFrame(frame: LogicalWindowFrame): Promise<void> {
    await options.applyWindowFrame(frame);
    lastAppliedWindowHeight = frame.height;
    lastAppliedWindowWidth = frame.width;
  }

  /**
   * 收缩动画完成后一次性同步真实窗口高度，避免动画过程中暴露 Windows 原生直角边界
   */
  function finishCollapseWindowResize(
    activeCollapseAnimationVersion: number,
    targetFrame: LogicalWindowFrame,
  ): void {
    boxSurfaceVisualHeight.value = targetFrame.height;
    boxSurfaceVisualWidth.value = targetFrame.width;
    void applyCollapseFrame(targetFrame).finally(() => {
      if (activeCollapseAnimationVersion !== collapseAnimationVersion) {
        return;
      }

      boxSurfaceVisualHeight.value = null;
      boxSurfaceVisualWidth.value = null;
      isCollapseAnimating.value = false;
      // 窗口就位后再淡入内容，淡出与淡入夹住缩放段，视觉上只看到面板平滑变形
      isCollapseContentFaded.value = false;
    });
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
    clearExpandHoverTimer();
    boxSurfaceVisualHeight.value = null;
    boxSurfaceVisualWidth.value = null;
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
   * Box 区域 hover 进入时取消延迟收起；图标态下按设置延迟展开（防误触），
   * 其余形态保持立即展开的既有手感
   */
  function handleBoxMouseEnter(): void {
    isBoxHovered.value = true;
    if (isBoxInIconState.value && options.getBoxExpandHoverDelayMs() > 0) {
      clearExpandHoverTimer();
      expandHoverTimer = window.setTimeout(() => {
        expandHoverTimer = null;
        openCollapsedPreviewForActiveInteraction();
      }, options.getBoxExpandHoverDelayMs());
      return;
    }

    openCollapsedPreviewForActiveInteraction();
  }

  /**
   * 清理悬停展开延迟计时器，离开 Box、菜单打开或窗口卸载时都应取消旧任务
   */
  function clearExpandHoverTimer(): void {
    if (!expandHoverTimer) {
      return;
    }

    window.clearTimeout(expandHoverTimer);
    expandHoverTimer = null;
  }

  /**
   * 标题 hover 是收缩 Box 的展开入口，内容是否展开仍由标题区域单独控制
   */
  function handleBoxTitleMouseEnter(): void {
    isTitleHovered.value = true;
    openCollapsedPreviewForActiveInteraction();
  }

  /**
   * 离开标题后只取消 roll-up 展开入口，内容收回仍等鼠标离开整个 Box
   */
  function handleBoxTitleMouseLeave(): void {
    isTitleHovered.value = false;
    refreshCollapsedPreviewCloseSchedule();
  }

  function handleBoxMouseLeave(): void {
    isBoxHovered.value = false;
    isTitleHovered.value = false;
    clearExpandHoverTimer();
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
    isBoxExpandingFromIcon,
    isBoxInIconState,
    isCollapseAnimating,
    openCollapsedPreviewForActiveInteraction,
    refreshCollapsedPreviewCloseSchedule,
    setDragHoveringBox,
    setNativeItemContextMenuOpen,
    syncPointerHoverFromScreenPoint,
  };
}
