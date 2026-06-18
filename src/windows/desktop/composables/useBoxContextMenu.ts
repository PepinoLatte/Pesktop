import { ref } from "vue";
import { cursorPosition } from "@tauri-apps/api/window";
import {
  closeBoxContextMenuWindow,
  toggleBoxContextMenuWindow,
} from "@/entities/desktopBox/windows";

/**
 * Box 菜单组合式逻辑只负责独立菜单窗口的打开状态和切换保护，不直接读取 Store
 */
export function useBoxContextMenu(options: {
  boxId: () => string;
  menuToggleCloseGuardMs: number;
  openCollapsedPreviewForActiveInteraction: () => void;
  refreshCollapsedPreviewCloseSchedule: () => void;
  setLastError: (message: string) => void;
}) {
  const isContextMenuOpen = ref(false);
  let lastContextMenuClosedAt = 0;
  let lastContextMenuClosedBoxId = "";

  /**
   * 菜单窗口是单例复用的，Box 窗口不能再通过窗口是否存在判断自己菜单是否打开
   */
  function handleBoxContextMenuState(
    boxId: string,
    isOpen: boolean,
    reason?: "blur" | "request",
  ): void {
    const currentBoxId = options.boxId();

    isContextMenuOpen.value = isOpen && boxId === currentBoxId;
    if (isContextMenuOpen.value) {
      options.openCollapsedPreviewForActiveInteraction();
    }
    if (!isOpen && boxId === currentBoxId && reason === "blur") {
      lastContextMenuClosedAt = performance.now();
      lastContextMenuClosedBoxId = boxId;
    }

    if (!isContextMenuOpen.value) {
      options.refreshCollapsedPreviewCloseSchedule();
    }
  }

  /**
   * 更多菜单按钮使用切换语义；菜单打开时再次点击只触发关闭动画，不重新计算位置
   */
  function toggleContextMenu(): void {
    const currentBoxId = options.boxId();

    options.openCollapsedPreviewForActiveInteraction();

    const shouldCloseMenu = isContextMenuOpen.value || wasContextMenuJustClosedByButton();
    if (shouldCloseMenu) {
      lastContextMenuClosedAt = performance.now();
      lastContextMenuClosedBoxId = currentBoxId;
      isContextMenuOpen.value = false;
      void closeBoxContextMenuWindow(currentBoxId)
        .then(() => {
          options.refreshCollapsedPreviewCloseSchedule();
        })
        .catch(reportError);
      return;
    }

    void cursorPosition()
      .then((position) => toggleBoxContextMenuWindow(currentBoxId, position, false))
      .then((result) => {
        if (result === "closed") {
          options.refreshCollapsedPreviewCloseSchedule();
        }
      })
      .catch(reportError);
  }

  /**
   * 单例菜单失焦会先于更多按钮 click 到达；短时间内的关闭状态视为当前按钮二次点击关闭，避免误重新打开
   */
  function wasContextMenuJustClosedByButton(): boolean {
    return (
      lastContextMenuClosedBoxId === options.boxId() &&
      performance.now() - lastContextMenuClosedAt <= options.menuToggleCloseGuardMs
    );
  }

  /**
   * 独立菜单窗口可能正处于进入或退出动画中；Box 发生拖动、缩放或标题编辑前统一请求它关闭
   */
  function closeContextMenu(): void {
    void closeBoxContextMenuWindow(options.boxId()).catch(reportError);
    options.refreshCollapsedPreviewCloseSchedule();
  }

  /**
   * Tauri 事件错误统一转成字符串回写给调用方，避免组合式逻辑直接依赖具体 Store
   */
  function reportError(error: unknown): void {
    options.setLastError(error instanceof Error ? error.message : String(error));
  }

  return {
    closeContextMenu,
    handleBoxContextMenuState,
    isContextMenuOpen,
    toggleContextMenu,
  };
}
