import { computed, ref } from "vue";
import {
  isAutostartEnabled,
  setAutostartEnabled,
} from "@/entities/appSettings/api";
import { chooseCollectionRootFolder } from "@/entities/desktopBox/api";
import type { useDesktopStore } from "@/entities/desktopBox/store";

/**
 * 设置动作组合式逻辑封装系统自启、收纳根目录和迁移等跨面板操作。
 */
export function useSettingsActions(desktopStore: ReturnType<typeof useDesktopStore>) {
  const autostartEnabled = ref(false);
  const isMigratingCollectionRoot = ref(false);
  const migratableBoxCount = computed(() => desktopStore.getMigratableBoxes().length);

  /**
   * 开机自启以系统启动项为准，设置页每次需要展示时都重新读取，避免外部修改后状态滞后。
   */
  async function syncAutostartEnabled(): Promise<void> {
    try {
      autostartEnabled.value = await isAutostartEnabled();
    } catch (error) {
      desktopStore.lastError = error instanceof Error ? error.message : String(error);
    }
  }

  /**
   * 设置页切换自启后交给 Rust 同步托盘勾选状态，前端只保存后端确认后的真实结果。
   */
  async function updateAutostartEnabled(value: boolean): Promise<void> {
    try {
      autostartEnabled.value = await setAutostartEnabled(value);
    } catch (error) {
      desktopStore.lastError = error instanceof Error ? error.message : String(error);
      await syncAutostartEnabled();
    }
  }

  /**
   * 设置页选择收纳根目录只影响后续新建 Box，不移动已有真实文件夹。
   */
  async function chooseCollectionRootFromSettings(): Promise<void> {
    try {
      const selectedPath = await chooseCollectionRootFolder();
      if (selectedPath) {
        await desktopStore.updateCollectionRootPath(selectedPath);
      }
    } catch (error) {
      desktopStore.lastError = error instanceof Error ? error.message : String(error);
    }
  }

  /**
   * 打开当前收纳根目录，帮助用户确认迁移目标和新建 Box 的真实磁盘位置。
   */
  async function openCollectionRootFromSettings(): Promise<void> {
    try {
      await desktopStore.openCollectionRootPath();
    } catch (error) {
      desktopStore.lastError = error instanceof Error ? error.message : String(error);
    }
  }

  /**
   * 收纳位置迁移是真实文件移动，设置页只允许一次迁移任务进行，避免重复点击造成路径竞争。
   */
  async function migrateCollectionRootFromSettings(): Promise<void> {
    if (isMigratingCollectionRoot.value) {
      return;
    }

    isMigratingCollectionRoot.value = true;
    try {
      await desktopStore.migrateExistingBoxFolders();
    } catch (error) {
      desktopStore.lastError = error instanceof Error ? error.message : String(error);
    } finally {
      isMigratingCollectionRoot.value = false;
    }
  }

  return {
    autostartEnabled,
    chooseCollectionRootFromSettings,
    isMigratingCollectionRoot,
    migratableBoxCount,
    migrateCollectionRootFromSettings,
    openCollectionRootFromSettings,
    syncAutostartEnabled,
    updateAutostartEnabled,
  };
}
