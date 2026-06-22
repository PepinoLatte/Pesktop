import { ref } from "vue";
import type { ComputedRef, Ref } from "vue";
import { listBoxFolderItems } from "@/entities/desktopItem/api";
import type { DesktopItem } from "@/entities/desktopItem/types";
import type { DesktopBox } from "@/entities/desktopBox/types";

/**
 * 文件视图轮询配置只影响当前 Box WebView，外部 Explorer 改动会在下一轮扫描中同步。
 */
const BOX_FILE_VIEW = {
  refreshIntervalMs: 1400,
} as const;

/**
 * 文件列表扫描只需要当前 Box 的真实文件夹路径和错误回写入口。
 */
interface BoxFileItemsOptions {
  box: ComputedRef<DesktopBox | undefined>;
  setLastError: (message: string) => void;
}

/**
 * Box 文件列表状态和刷新能力，供选择、拖拽和文件操作共享同一份扫描结果。
 */
export interface BoxFileItemsState {
  boxItems: Ref<DesktopItem[]>;
  refreshBoxFolderItems: (options?: { silent?: boolean }) => Promise<void>;
  startFolderRefreshPolling: () => void;
  stopFolderRefreshPolling: () => void;
}

/**
 * 管理 Box 文件夹扫描和兜底轮询，保证外部 Explorer 手动修改后前端文件网格会自动追上。
 */
export function useBoxFileItems(options: BoxFileItemsOptions): BoxFileItemsState {
  const boxItems = ref<DesktopItem[]>([]);
  let refreshTimer: ReturnType<typeof window.setInterval> | null = null;

  /**
   * 轮询是当前 WebView 文件视图的兜底刷新机制，避免外部文件变更长期停留在旧列表。
   */
  function startFolderRefreshPolling(): void {
    stopFolderRefreshPolling();
    refreshTimer = window.setInterval(() => {
      void refreshBoxFolderItems({ silent: true });
    }, BOX_FILE_VIEW.refreshIntervalMs);
  }

  /**
   * 清理文件扫描轮询，窗口卸载或重新启动轮询时必须成对调用。
   */
  function stopFolderRefreshPolling(): void {
    if (!refreshTimer) {
      return;
    }

    window.clearInterval(refreshTimer);
    refreshTimer = null;
  }

  /**
   * 扫描当前 Box 真实文件夹；静默刷新只吞掉错误，显式刷新会把错误交给窗口展示。
   */
  async function refreshBoxFolderItems(
    refreshOptions: { silent?: boolean } = {},
  ): Promise<void> {
    if (!options.box.value?.folderPath) {
      boxItems.value = [];
      return;
    }

    try {
      boxItems.value = await listBoxFolderItems(options.box.value.folderPath);
    } catch (error) {
      if (!refreshOptions.silent) {
        options.setLastError(error instanceof Error ? error.message : String(error));
      }
    }
  }

  return {
    boxItems,
    refreshBoxFolderItems,
    startFolderRefreshPolling,
    stopFolderRefreshPolling,
  };
}
