import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import type { ComputedRef, Ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { animate } from "motion";
import { BOX_CONTEXT_MENU_LAYOUT } from "@/entities/desktopBox/layout";
import { useDesktopStore } from "@/entities/desktopBox/store";
import type { DesktopBox } from "@/entities/desktopBox/types";
import {
  listenBoxContextMenuClose,
  listenBoxContextMenuOpen,
  notifyBoxContextMenuPrepared,
  notifyBoxContextMenuReady,
  notifyBoxContextMenuState,
} from "@/shared/ipc/boxContextMenu";

/**
 * 菜单窗口生命周期依赖外部删除确认状态，打开或关闭菜单时必须清掉危险二次确认。
 */
interface BoxContextMenuWindowOptions {
  clearBoxDeleteConfirmation: () => void;
}

/**
 * 菜单壳组件只需要这些状态和关闭能力，具体按钮动作由 action composable 承担。
 */
export interface BoxContextMenuWindowState {
  activeBoxId: Readonly<Ref<string | null>>;
  box: ComputedRef<DesktopBox | undefined>;
  closeAnimated: (requestedBoxId?: string) => Promise<void>;
  isMenuRendered: Readonly<Ref<boolean>>;
  menuRef: Ref<HTMLElement | null>;
}

/**
 * 管理独立 Box 菜单 WebView 的打开准备、进入动画、退出动画和跨窗口监听。
 */
export function useBoxContextMenuWindow(
  options: BoxContextMenuWindowOptions,
): BoxContextMenuWindowState {
  const currentWindow = getCurrentWindow();
  const desktopStore = useDesktopStore();
  const activeBoxId = ref<string | null>(null);
  const isMenuRendered = ref(false);
  const menuRef = ref<HTMLElement | null>(null);
  const unlistenFns: UnlistenFn[] = [];
  let menuAnimation: ReturnType<typeof animate> | null = null;
  let menuAnimationVersion = 0;
  let lastBlurCloseAt = 0;
  let activeOpenRequestId = "";

  const box = computed(() =>
    activeBoxId.value
      ? desktopStore.boxes.find((item) => item.id === activeBoxId.value)
      : undefined,
  );

  onMounted(async () => {
    await desktopStore.initializeBoxMenu();

    unlistenFns.push(
      await currentWindow.onFocusChanged(({ payload }) => {
        if (!payload) {
          lastBlurCloseAt = performance.now();
          void closeAnimated();
        }
      }),
    );

    unlistenFns.push(
      await listenBoxContextMenuClose(({ payload }) => {
        void closeAnimated(payload.boxId);
      }),
    );

    unlistenFns.push(
      await listenBoxContextMenuOpen(({ payload }) => {
        if (payload.stage === "hidden") {
          void prepareMenuOpen(payload.boxId, payload.requestId);
          return;
        }

        void openAnimated(payload.boxId, payload.requestId);
      }),
    );

    window.addEventListener("keydown", handleKeyDown);
    await notifyBoxContextMenuReady();
  });

  /**
   * 销毁时清理跨窗口监听和动画句柄，避免开发热更新后重复响应菜单事件。
   */
  onUnmounted(() => {
    menuAnimation?.stop();
    menuAnimation = null;
    window.removeEventListener("keydown", handleKeyDown);
    for (const unlisten of unlistenFns) {
      unlisten();
    }
  });

  /**
   * Escape 是系统菜单的常见关闭方式，给键盘用户保留一致退路。
   */
  function handleKeyDown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      void closeAnimated();
    }
  }

  /**
   * 隐藏窗口里先渲染菜单并写入透明首帧，避免原生窗口 show 时闪出旧内容。
   */
  async function prepareMenuOpen(boxId: string, requestId: string): Promise<void> {
    activeOpenRequestId = requestId;
    activeBoxId.value = boxId;
    isMenuRendered.value = true;
    options.clearBoxDeleteConfirmation();
    menuAnimationVersion += 1;

    await nextTick();
    if (!box.value) {
      await closeAnimated(boxId);
      return;
    }

    const menuElement = menuRef.value;
    stopMenuAnimation();
    if (menuElement) {
      applyMenuOpacity(0);
    }
    await notifyBoxContextMenuPrepared(requestId, resolveMenuContentHeight(menuElement));
  }

  /**
   * 菜单打开只做淡入动画，避免快速开关时位移和缩放被打断造成卡顿感。
   */
  async function openAnimated(boxId: string, requestId: string): Promise<void> {
    if (activeOpenRequestId !== requestId) {
      return;
    }

    activeBoxId.value = boxId;
    isMenuRendered.value = true;
    menuAnimationVersion += 1;
    const activeAnimationVersion = menuAnimationVersion;

    await nextTick();
    if (!box.value) {
      await closeAnimated(boxId);
      return;
    }

    await notifyBoxContextMenuState({ boxId, isOpen: true });
    const menuElement = menuRef.value;
    stopMenuAnimation();
    if (!menuElement || shouldReduceMotion()) {
      applyMenuOpacity(1);
      return;
    }

    menuElement.style.willChange = "opacity";
    menuAnimation = animate(
      menuElement,
      {
        opacity: 1,
      },
      {
        duration: BOX_CONTEXT_MENU_LAYOUT.openAnimationMs / 1000,
        ease: [0.22, 1, 0.36, 1],
      },
    );

    try {
      await menuAnimation.finished;
    } catch {
      // motion 在被新动画打断时会拒绝 finished，版本号会阻止旧动画继续收尾。
    } finally {
      if (activeAnimationVersion === menuAnimationVersion) {
        menuAnimation = null;
        applyMenuOpacity(1);
        menuElement.style.willChange = "";
      }
    }
  }

  /**
   * 菜单关闭只做短淡出，再隐藏预载窗口；没有位移可减少快速点击时的视觉冲突。
   */
  async function closeAnimated(requestedBoxId?: string): Promise<void> {
    const closingBoxId = activeBoxId.value;
    if (requestedBoxId && closingBoxId && requestedBoxId !== closingBoxId) {
      return;
    }

    options.clearBoxDeleteConfirmation();
    if (!closingBoxId && !isMenuRendered.value) {
      await currentWindow.hide();
      return;
    }

    menuAnimationVersion += 1;
    const activeAnimationVersion = menuAnimationVersion;
    if (closingBoxId) {
      await notifyBoxContextMenuState({
        boxId: closingBoxId,
        isOpen: false,
        reason: resolveCloseReason(),
      });
    }

    const menuElement = menuRef.value;
    stopMenuAnimation();
    if (!menuElement || shouldReduceMotion()) {
      await finishCloseAnimation(activeAnimationVersion);
      return;
    }

    menuElement.style.willChange = "opacity";
    menuAnimation = animate(
      menuElement,
      {
        opacity: 0,
      },
      {
        duration: BOX_CONTEXT_MENU_LAYOUT.closeAnimationMs / 1000,
        ease: [0.4, 0, 1, 1],
      },
    );

    try {
      await menuAnimation.finished;
    } catch {
      // 退出动画被新的打开动作打断时不再隐藏窗口，版本号会拦住旧收尾。
    } finally {
      await finishCloseAnimation(activeAnimationVersion);
    }
  }

  /**
   * 关闭动画收尾必须检查版本号，避免旧的退出动画覆盖新的菜单打开状态。
   */
  async function finishCloseAnimation(activeAnimationVersion: number): Promise<void> {
    if (activeAnimationVersion !== menuAnimationVersion) {
      return;
    }

    applyMenuOpacity(0);
    if (menuRef.value) {
      menuRef.value.style.willChange = "";
    }
    menuAnimation = null;
    activeOpenRequestId = "";
    activeBoxId.value = null;
    isMenuRendered.value = false;
    await currentWindow.hide();
  }

  /**
   * 快速连续开关菜单时先停掉上一段动画，并清理历史 transform 动画，避免热更新或旧版本动画残留缩放。
   */
  function stopMenuAnimation(): void {
    menuAnimation?.stop();
    menuAnimation = null;
    const menuElement = menuRef.value;
    if (!menuElement) {
      return;
    }

    for (const animation of menuElement.getAnimations()) {
      animation.cancel();
    }
    clearMenuTransform(menuElement);
  }

  /**
   * 菜单当前只允许透明度变化；每次写终态都同步清空 transform，避免窗口 resize 被误看成菜单缩放。
   */
  function applyMenuOpacity(opacity: number): void {
    const menuElement = menuRef.value;
    if (!menuElement) {
      return;
    }

    menuElement.style.opacity = String(opacity);
    clearMenuTransform(menuElement);
  }

  /**
   * 历史版本曾经使用 scale/translate 动画，显式清理可防止 Web Animations 残留到复用窗口。
   */
  function clearMenuTransform(menuElement: HTMLElement): void {
    menuElement.style.transform = "none";
    menuElement.style.removeProperty("rotate");
    menuElement.style.removeProperty("scale");
    menuElement.style.removeProperty("translate");
  }

  /**
   * 菜单窗口高度由父窗口按显示器工作区裁剪；这里返回完整内容高度，避免新增菜单项后被固定常量裁掉。
   */
  function resolveMenuContentHeight(menuElement: HTMLElement | null): number {
    if (!menuElement) {
      return Math.max(
        1,
        document.documentElement.scrollHeight,
        document.body.scrollHeight,
      );
    }

    const computedStyle = window.getComputedStyle(menuElement);
    const verticalBorderWidth =
      Number.parseFloat(computedStyle.borderTopWidth) +
      Number.parseFloat(computedStyle.borderBottomWidth);

    return Math.ceil(
      Math.max(menuElement.scrollHeight + verticalBorderWidth + 2, menuElement.offsetHeight),
    );
  }

  /**
   * 动效统一通过 motion 承载，但系统要求减少动态效果时仍需要跳过过渡。
   */
  function shouldReduceMotion(): boolean {
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  }

  /**
   * 菜单失焦可能由更多按钮二次点击触发，父窗口据此避免把这次 click 当成重新打开。
   */
  function resolveCloseReason(): "blur" | "request" {
    return performance.now() - lastBlurCloseAt <= 120 ? "blur" : "request";
  }

  return {
    activeBoxId,
    box,
    closeAnimated,
    isMenuRendered,
    menuRef,
  };
}
