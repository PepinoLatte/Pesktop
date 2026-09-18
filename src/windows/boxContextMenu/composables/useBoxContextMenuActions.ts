import { computed } from "vue";
import type { ComputedRef } from "vue";
import { openBoxFolder } from "@/entities/desktopBox/api";
import type { DesktopBoxDeleteConfirmationController } from "@/entities/desktopBox/deleteConfirmation";
import { useDesktopStore } from "@/entities/desktopBox/store";
import type { BoxCollapseMode, DesktopBox, DesktopBoxTitlePosition } from "@/entities/desktopBox/types";
import { closeBoxWindow, openBoxWindow, openSettingsWindow } from "@/entities/desktopBox/windows";
import type { BoxAutoCollapseMode } from "@/windows/boxContextMenu/model/menuOptions";

/**
 * 菜单动作依赖当前激活 Box 和关闭动画；真实业务仍统一走 desktop store。
 */
interface BoxContextMenuActionsOptions {
  box: ComputedRef<DesktopBox | undefined>;
  clearBoxDeleteConfirmation: DesktopBoxDeleteConfirmationController["clearBoxDeleteConfirmation"];
  closeAnimated: (requestedBoxId?: string) => Promise<void>;
  requestBoxDeleteConfirmation: DesktopBoxDeleteConfirmationController["requestBoxDeleteConfirmation"];
}

/**
 * 更多菜单对外暴露的动作集合，便于壳组件按组件事件逐一绑定。
 */
export interface BoxContextMenuActions {
  boxAutoCollapseMode: ComputedRef<BoxAutoCollapseMode>;
  createBoxFromMenu: () => Promise<void>;
  deleteCurrentBox: () => Promise<void>;
  openFolderFromMenu: () => Promise<void>;
  openSettingsFromMenu: () => Promise<void>;
  refreshDesktopFromMenu: () => Promise<void>;
  toggleBoxLockedFromMenu: () => Promise<void>;
  updateBoxAutoCollapseFromMenu: (mode: BoxAutoCollapseMode) => Promise<void>;
  updateBoxCollapseModeFromMenu: (mode: BoxCollapseMode) => Promise<void>;
  updateIdleOpacityFromMenu: (nextOpacity: number) => Promise<void>;
  updateTitlePositionFromMenu: (position: DesktopBoxTitlePosition) => Promise<void>;
}

/**
 * 管理 Box 更多菜单中的业务动作，保持模板和纯展示组件不直接关心 Store 细节。
 */
export function useBoxContextMenuActions(
  options: BoxContextMenuActionsOptions,
): BoxContextMenuActions {
  const desktopStore = useDesktopStore();
  const boxAutoCollapseMode = computed<BoxAutoCollapseMode>(() =>
    options.box.value?.collapsed ? "rollup" : "always",
  );

  /**
   * Box 菜单只负责唤起设置页，具体配置仍由主窗口统一承载。
   */
  async function openSettingsFromMenu(): Promise<void> {
    void options.closeAnimated();
    await openSettingsWindow();
  }

  /**
   * 新增 Box 复用桌面 Store 和窗口打开逻辑，确保菜单入口与设置页创建行为一致。
   */
  async function createBoxFromMenu(): Promise<void> {
    options.clearBoxDeleteConfirmation();
    void options.closeAnimated();
    try {
      const createdBox = await desktopStore.createBox();

      await openBoxWindow(createdBox, { focus: true });
    } catch (error) {
      desktopStore.lastError = error instanceof Error ? error.message : String(error);
    }
  }

  /**
   * 刷新只重新读取桌面目录，不改变任何 Box 布局和真实文件位置。
   */
  async function refreshDesktopFromMenu(): Promise<void> {
    void options.closeAnimated();
    await desktopStore.refreshSnapshot();
  }

  /**
   * 菜单里的打开文件夹入口直接交给系统 Explorer，便于用户跳出 Box 检查真实目录。
   */
  async function openFolderFromMenu(): Promise<void> {
    if (!options.box.value) {
      return;
    }

    const folderPath = options.box.value.folderPath;
    void options.closeAnimated();
    await openBoxFolder(folderPath).catch((error) => {
      desktopStore.lastError = error instanceof Error ? error.message : String(error);
    });
  }

  /**
   * Box 标题位置从菜单直接切换，适合用户在整理时即时调整窗口布局。
   */
  async function updateTitlePositionFromMenu(
    position: DesktopBoxTitlePosition,
  ): Promise<void> {
    if (!options.box.value) {
      return;
    }

    options.clearBoxDeleteConfirmation();
    await desktopStore.updateBoxTitlePosition(options.box.value.id, position);
  }

  /**
   * 自动收起属于当前 Box 的桌面整理习惯，落库后由对应 Box 窗口自行同步窗口高度。
   */
  async function updateBoxAutoCollapseFromMenu(mode: BoxAutoCollapseMode): Promise<void> {
    if (!options.box.value) {
      return;
    }

    options.clearBoxDeleteConfirmation();
    await desktopStore.updateBoxCollapsed(options.box.value.id, mode === "rollup");
  }

  /**
   * 收缩形态决定闲置时保留标题条还是缩成图标，切换后由对应 Box 窗口自行重适配。
   */
  async function updateBoxCollapseModeFromMenu(mode: BoxCollapseMode): Promise<void> {
    if (!options.box.value) {
      return;
    }

    options.clearBoxDeleteConfirmation();
    await desktopStore.updateBox({ ...options.box.value, collapseMode: mode });
  }

  /**
   * 锁定只冻结当前 Box 的几何操作，不影响内部图标打开、排序和右键。
   */
  async function toggleBoxLockedFromMenu(): Promise<void> {
    if (!options.box.value) {
      return;
    }

    options.clearBoxDeleteConfirmation();
    await desktopStore.updateBoxLocked(options.box.value.id, !options.box.value.locked);
  }

  /**
   * 闲置可见度从菜单滑块实时保存，0 表示未 hover 时整个 Box 隐形，hover 后仍恢复可见。
   */
  async function updateIdleOpacityFromMenu(nextOpacity: number): Promise<void> {
    if (!options.box.value) {
      return;
    }

    options.clearBoxDeleteConfirmation();
    await desktopStore.updateBoxTitleOpacity(options.box.value.id, nextOpacity);
  }

  /**
   * 删除 Box 前先按当前删除策略处理真实文件夹，用户取消 Shell 操作时保留 Box。
   */
  async function deleteCurrentBox(): Promise<void> {
    if (!options.box.value) {
      return;
    }

    const targetBoxId = options.box.value.id;
    if (!options.requestBoxDeleteConfirmation(options.box.value)) {
      return;
    }

    void options.closeAnimated();
    try {
      await desktopStore.deleteBox(targetBoxId);
      await closeBoxWindow(targetBoxId);
    } catch (error) {
      desktopStore.lastError = error instanceof Error ? error.message : String(error);
    }
  }

  return {
    boxAutoCollapseMode,
    createBoxFromMenu,
    deleteCurrentBox,
    openFolderFromMenu,
    openSettingsFromMenu,
    refreshDesktopFromMenu,
    toggleBoxLockedFromMenu,
    updateBoxAutoCollapseFromMenu,
    updateBoxCollapseModeFromMenu,
    updateIdleOpacityFromMenu,
    updateTitlePositionFromMenu,
  };
}
