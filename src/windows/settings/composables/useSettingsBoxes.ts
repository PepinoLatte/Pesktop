import { openBoxFolder } from "@/entities/desktopBox/api";
import { closeBoxWindow, openBoxWindow } from "@/entities/desktopBox/windows";
import type { useDesktopStore } from "@/entities/desktopBox/store";
import type { DesktopBox } from "@/entities/desktopBox/types";

/**
 * Box 设置组合式逻辑封装列表面板触发的 Box 生命周期操作。
 */
export function useSettingsBoxes(desktopStore: ReturnType<typeof useDesktopStore>) {
  /**
   * 新增 Box 后立即打开独立窗口，确保用户看到的是桌面扩展本体。
   */
  async function createAndOpenBox(): Promise<void> {
    try {
      const box = await desktopStore.createBox();
      await openBoxWindow(box, { focus: true });
    } catch (error) {
      desktopStore.lastError = error instanceof Error ? error.message : String(error);
    }
  }

  /**
   * 设置页里的锁定入口与 Box 更多菜单共用同一 Store 字段，确保窗口拖动和缩放行为一致。
   */
  async function toggleBoxLockedFromSettings(box: DesktopBox): Promise<void> {
    await desktopStore.updateBoxLocked(box.id, !box.locked);
  }

  /**
   * 设置页收到的删除事件已经由按钮完成二段式确认，这里只执行真实删除和窗口关闭。
   */
  async function deleteBoxFromSettings(box: DesktopBox): Promise<void> {
    try {
      await desktopStore.deleteBox(box.id);
      await closeBoxWindow(box.id);
    } catch (error) {
      desktopStore.lastError = error instanceof Error ? error.message : String(error);
    }
  }

  /**
   * 打开 Box 对应的真实文件夹，便于用户确认原生 Explorer 视图背后的磁盘位置。
   */
  async function openBoxFolderFromSettings(box: DesktopBox): Promise<void> {
    try {
      await openBoxFolder(box.folderPath);
    } catch (error) {
      desktopStore.lastError = error instanceof Error ? error.message : String(error);
    }
  }

  return {
    createAndOpenBox,
    deleteBoxFromSettings,
    openBoxFolderFromSettings,
    openBoxWindow,
    toggleBoxLockedFromSettings,
  };
}
