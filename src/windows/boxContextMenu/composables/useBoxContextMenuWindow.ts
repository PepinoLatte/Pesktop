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
   * 隐藏窗口里先渲染菜单并写入动画首帧，避免原生窗口 show 时闪出旧内容。
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
    menuAnimation?.stop();
    menuAnimation = null;
    if (menuElement) {
      applyMenuVisibleState(0, "translateY(-6px) scale(0.98)");
    }
    await notifyBoxContextMenuPrepared(requestId);
  }

  /**
   * 菜单打开只播放 motion 进入动画，窗口创建和首帧准备已在 hidden 阶段完成。
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
    if (!menuElement || shouldReduceMotion()) {
      applyMenuVisibleState(1, "translateY(0) scale(1)");
      return;
    }

    menuAnimation?.stop();
    applyMenuVisibleState(0, "translateY(-6px) scale(0.98)");
    menuAnimation = animate(
      menuElement,
      {
        opacity: 1,
        transform: "translateY(0) scale(1)",
      },
      {
        duration: BOX_CONTEXT_MENU_LAYOUT.openAnimationMs / 1000,
        ease: [0.2, 0.8, 0.2, 1],
      },
    );

    try {
      await menuAnimation.finished;
    } catch {
      // motion 在被新动画打断时会拒绝 finished，版本号会阻止旧动画继续收尾。
    } finally {
      if (activeAnimationVersion === menuAnimationVersion) {
        menuAnimation = null;
      }
    }
  }

  /**
   * 菜单关闭先播放 motion 退出动画，再隐藏预载窗口，后续打开可以复用同一个 WebView。
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
    menuAnimation?.stop();
    if (!menuElement || shouldReduceMotion()) {
      await finishCloseAnimation(activeAnimationVersion);
      return;
    }

    menuAnimation = animate(
      menuElement,
      {
        opacity: 0,
        transform: "translateY(-4px) scale(0.98)",
      },
      {
        duration: BOX_CONTEXT_MENU_LAYOUT.closeAnimationMs / 1000,
        ease: [0.4, 0, 1, 1],
      },
    );

    try {
      await menuAnimation.finished;
    } catch {
      // 退出动画被新的打开动作打断时不再隐藏窗口，避免快速切换时闪烁。
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

    menuAnimation = null;
    activeBoxId.value = null;
    isMenuRendered.value = false;
    await currentWindow.hide();
  }

  /**
   * 减少动态效果时直接写入终态，既保留可见性语义，也遵守系统辅助功能设置。
   */
  function applyMenuVisibleState(opacity: number, transform: string): void {
    const menuElement = menuRef.value;
    if (!menuElement) {
      return;
    }

    menuElement.style.opacity = String(opacity);
    menuElement.style.transform = transform;
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
