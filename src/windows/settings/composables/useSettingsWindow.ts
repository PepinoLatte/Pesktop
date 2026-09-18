import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { SETTINGS_WINDOW_SYNC_TIMING } from "@/entities/desktopBox/layout";
import type { useDesktopStore } from "@/entities/desktopBox/store";

/**
 * 设置窗口组合式逻辑负责无边框窗口操作、标题栏拖动和聚焦后的状态同步。
 */
export function useSettingsWindow(
  desktopStore: ReturnType<typeof useDesktopStore>,
  syncAutostartEnabled: () => Promise<void>,
) {
  const currentWindow = getCurrentWindow();
  const FOCUS_SYNC_DELAY_MS = SETTINGS_WINDOW_SYNC_TIMING.focusSyncDelayMs;
  const DRAG_RELEASE_FALLBACK_MS = SETTINGS_WINDOW_SYNC_TIMING.dragReleaseFallbackMs;
  let focusSyncTimer: ReturnType<typeof window.setTimeout> | null = null;
  let isDraggingSettingsWindow = false;
  let dragReleaseCleanup: (() => void) | null = null;
  const unlistenFns: UnlistenFn[] = [];

  /**
   * 设置窗获得焦点时不立刻刷新桌面快照，避免 Shell 缩略图扫描卡住标题栏拖动首帧。
   */
  async function bindFocusSync(): Promise<void> {
    unlistenFns.push(
      await currentWindow.onFocusChanged(({ payload }) => {
        if (!payload) {
          clearFocusSyncTimer();
          return;
        }

        scheduleFocusSync();
      }),
    );
  }

  /**
   * 自定义标题栏通过 Tauri 转交拖动，保持无边框窗口仍可移动。
   */
  function startDragging(event: MouseEvent): void {
    if (event.button !== 0 || event.detail > 1) {
      return;
    }

    isDraggingSettingsWindow = true;
    clearFocusSyncTimer();
    bindDragReleaseListeners();
    void currentWindow.startDragging();
  }

  /**
   * 焦点同步延迟执行，让用户点击标题栏拖动时先进入系统拖动流程，再刷新 SQLite 和桌面快照。
   */
  function scheduleFocusSync(): void {
    clearFocusSyncTimer();
    focusSyncTimer = window.setTimeout(() => {
      focusSyncTimer = null;
      void syncSettingsWindowState();
    }, FOCUS_SYNC_DELAY_MS);
  }

  /**
   * 聚焦后同步持久化状态和桌面快照；拖动中收到兜底触发时继续后延，避免刷新抢占拖动。
   */
  async function syncSettingsWindowState(): Promise<void> {
    if (isDraggingSettingsWindow) {
      scheduleFocusSync();
      return;
    }

    await desktopStore.reloadPersistedState();
    await syncAutostartEnabled();
    await desktopStore.refreshSnapshot(false);
  }

  /**
   * 清理焦点同步计时器，避免隐藏或拖动窗口时仍然启动一次昂贵的桌面扫描。
   */
  function clearFocusSyncTimer(): void {
    if (!focusSyncTimer) {
      return;
    }

    window.clearTimeout(focusSyncTimer);
    focusSyncTimer = null;
  }

  /**
   * 监听拖动释放后再恢复设置同步；原生拖动吞掉释放事件时用短兜底恢复。
   */
  function bindDragReleaseListeners(): void {
    clearDragReleaseListeners();

    const fallbackTimer = window.setTimeout(finishDragging, DRAG_RELEASE_FALLBACK_MS);
    function finishDragging(): void {
      isDraggingSettingsWindow = false;
      clearDragReleaseListeners();
      scheduleFocusSync();
    }

    window.addEventListener("mouseup", finishDragging, { capture: true, once: true });
    window.addEventListener("pointerup", finishDragging, { capture: true, once: true });
    document.addEventListener("mouseup", finishDragging, { capture: true, once: true });
    document.addEventListener("pointerup", finishDragging, { capture: true, once: true });
    dragReleaseCleanup = () => {
      window.clearTimeout(fallbackTimer);
      window.removeEventListener("mouseup", finishDragging, { capture: true });
      window.removeEventListener("pointerup", finishDragging, { capture: true });
      document.removeEventListener("mouseup", finishDragging, { capture: true });
      document.removeEventListener("pointerup", finishDragging, { capture: true });
    };
  }

  /**
   * 拖动释放监听每次只保留一组，防止多次按住标题栏后重复安排同步。
   */
  function clearDragReleaseListeners(): void {
    dragReleaseCleanup?.();
    dragReleaseCleanup = null;
  }

  /**
   * 最小化只作用于设置窗口，桌面上的 Box 会继续保持显示。
   */
  function minimizeSettings(): void {
    void currentWindow.minimize();
  }

  /**
   * 最大化只作用于设置窗口，不改变任何 Box 的桌面位置和尺寸。
   */
  function toggleSettingsMaximize(): void {
    void currentWindow.toggleMaximize();
  }

  /**
   * 关闭设置页时隐藏主窗口，Box 右键菜单仍可重新唤起设置。
   */
  function closeSettings(): void {
    void currentWindow.hide();
  }

  /**
   * 设置窗口销毁时注销窗口事件，避免开发热更新后重复刷新状态。
   */
  function disposeSettingsWindow(): void {
    clearFocusSyncTimer();
    clearDragReleaseListeners();
    for (const unlisten of unlistenFns) {
      unlisten();
    }
    unlistenFns.splice(0);
  }

  return {
    bindFocusSync,
    closeSettings,
    currentWindow,
    disposeSettingsWindow,
    minimizeSettings,
    startDragging,
    toggleSettingsMaximize,
  };
}
