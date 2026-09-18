import { ref } from "vue";
import type { ComputedRef, Ref } from "vue";
import {
  appendBoxVirtualItemIds,
  loadBoxItemOrder,
  loadBoxVirtualItemIds,
  removeBoxVirtualItemIds,
  saveBoxItemOrder,
  saveBoxVirtualItemIds,
} from "@/shared/storage/database";
import {
  getBoxFolderRevision,
  listBoxFolderItems,
  listShellDesktopItems,
} from "@/entities/desktopItem/api";
import type { DesktopItem } from "@/entities/desktopItem/types";
import type { DesktopBox } from "@/entities/desktopBox/types";
import { syncShellDesktopIconVisibility } from "@/entities/desktopItem/shellDesktopIconVisibilitySync";

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
  getAutoHideNativeShellIcons: () => boolean;
  setLastError: (message: string) => void;
}

/**
 * 文件列表刷新可由轮询或用户操作触发；强制刷新用于绕过 revision 快捷判断。
 */
interface BoxFileRefreshOptions {
  force?: boolean;
  silent?: boolean;
}

/**
 * Box 文件列表状态和刷新能力，供选择、拖拽和文件操作共享同一份扫描结果。
 */
export interface BoxFileItemsState {
  boxItems: Ref<DesktopItem[]>;
  moveBoxItemsInOrder: (
    draggedPaths: string[],
    targetPath: string | null,
    placement: "after" | "before" | "end",
  ) => Promise<void>;
  appendBoxShellItems: (shellIds: string[]) => Promise<void>;
  removeBoxItemOrderPaths: (paths: string[]) => Promise<void>;
  removeBoxShellItems: (shellIds: string[]) => Promise<void>;
  replaceBoxItemOrderPath: (previousPath: string, nextPath: string) => Promise<void>;
  refreshBoxFolderItems: (options?: BoxFileRefreshOptions) => Promise<void>;
  startFolderRefreshPolling: () => void;
  stopFolderRefreshPolling: () => void;
}

/**
 * 管理 Box 文件夹扫描和兜底轮询，保证外部 Explorer 手动修改后前端文件网格会自动追上。
 */
export function useBoxFileItems(options: BoxFileItemsOptions): BoxFileItemsState {
  const boxItems = ref<DesktopItem[]>([]);
  let loadedOrderBoxId: string | null = null;
  let orderedPaths: string[] = [];
  let virtualShellIds: string[] = [];
  let refreshTimer: ReturnType<typeof window.setTimeout> | null = null;
  let isRefreshPollingActive = false;
  let isRefreshInFlight = false;
  let lastFullRefreshSignature: string | null = null;
  let queuedRefreshOptions: BoxFileRefreshOptions | null = null;

  /**
   * 轮询是当前 WebView 文件视图的兜底刷新机制；下一轮必须等上一轮完成后再排队，避免慢速 Shell 扫描堆叠造成周期性卡顿。
   */
  function startFolderRefreshPolling(): void {
    stopFolderRefreshPolling();
    isRefreshPollingActive = true;
    scheduleNextFolderRefreshPoll();
  }

  /**
   * 清理文件扫描轮询，窗口卸载或重新启动轮询时必须成对调用。
   */
  function stopFolderRefreshPolling(): void {
    isRefreshPollingActive = false;
    queuedRefreshOptions = null;
    if (refreshTimer) {
      window.clearTimeout(refreshTimer);
      refreshTimer = null;
    }
  }

  /**
   * 下一轮轮询由当前刷新完成后安排，保证低性能磁盘或大量图标解析时不会形成并发扫描。
   */
  function scheduleNextFolderRefreshPoll(): void {
    if (!isRefreshPollingActive || refreshTimer) {
      return;
    }

    refreshTimer = window.setTimeout(() => {
      refreshTimer = null;
      void refreshBoxFolderItems({ silent: true }).finally(scheduleNextFolderRefreshPoll);
    }, BOX_FILE_VIEW.refreshIntervalMs);
  }

  /**
   * 扫描当前 Box 真实文件夹；并发请求只保留一次后续刷新，避免多个入口同时触发重复扫描。
   */
  async function refreshBoxFolderItems(
    refreshOptions: BoxFileRefreshOptions = {},
  ): Promise<void> {
    if (isRefreshInFlight) {
      const previousQueuedRefreshOptions = queuedRefreshOptions;
      queuedRefreshOptions = {
        force: Boolean(previousQueuedRefreshOptions?.force || refreshOptions.force),
        silent: previousQueuedRefreshOptions
          ? previousQueuedRefreshOptions.silent === true && refreshOptions.silent === true
          : refreshOptions.silent === true,
      };
      return;
    }

    isRefreshInFlight = true;
    try {
      await refreshBoxFolderItemsOnce(refreshOptions);
    } finally {
      isRefreshInFlight = false;
    }

    const nextRefreshOptions = queuedRefreshOptions;
    queuedRefreshOptions = null;
    if (nextRefreshOptions) {
      await refreshBoxFolderItems(nextRefreshOptions);
    }
  }

  /**
   * 执行一次真实扫描；静默刷新只吞掉错误，显式刷新会把错误交给窗口展示。
   */
  async function refreshBoxFolderItemsOnce(
    refreshOptions: BoxFileRefreshOptions = {},
  ): Promise<void> {
    const currentBox = options.box.value;
    if (!currentBox?.folderPath) {
      lastFullRefreshSignature = null;
      updateBoxItemsIfChanged([]);
      return;
    }

    try {
      await ensureBoxItemOrderLoaded(currentBox.id);
      const nextFullRefreshSignature = await resolveFullRefreshSignature(currentBox.folderPath);
      if (!refreshOptions.force && lastFullRefreshSignature === nextFullRefreshSignature) {
        return;
      }

      const [scannedItems, shellItems] = await Promise.all([
        listBoxFolderItems(currentBox.folderPath),
        listShellDesktopItems(virtualShellIds),
      ]);
      updateBoxItemsIfChanged(applyManualOrder([...scannedItems, ...shellItems]));
      lastFullRefreshSignature = nextFullRefreshSignature;
    } catch (error) {
      if (!refreshOptions.silent) {
        options.setLastError(error instanceof Error ? error.message : String(error));
      }
    }
  }

  /**
   * 每个 Box 只在窗口首次扫描或切换 Box 时读取一次顺序，避免轮询刷新频繁访问 SQLite。
   */
  async function ensureBoxItemOrderLoaded(boxId: string): Promise<void> {
    if (loadedOrderBoxId === boxId) {
      return;
    }

    const [loadedOrderedPaths, loadedVirtualShellIds] = await Promise.all([
      loadBoxItemOrder(boxId),
      loadBoxVirtualItemIds(boxId),
    ]);
    orderedPaths = loadedOrderedPaths;
    virtualShellIds = loadedVirtualShellIds;
    loadedOrderBoxId = boxId;
  }

  /**
   * 扫描结果以真实文件为准，手动顺序只负责重排仍存在的路径，新文件自然追加到末尾。
   */
  function applyManualOrder(items: DesktopItem[]): DesktopItem[] {
    if (orderedPaths.length === 0) {
      return items;
    }

    const itemByPath = new Map(items.map((item) => [item.path, item]));
    const orderedItems = orderedPaths
      .map((path) => itemByPath.get(path))
      .filter((item): item is DesktopItem => Boolean(item));
    const orderedPathSet = new Set(orderedItems.map((item) => item.path));
    const unorderedItems = items.filter((item) => !orderedPathSet.has(item.path));

    return [...orderedItems, ...unorderedItems];
  }

  /**
   * 完整刷新签名合并真实文件夹 revision 和 Shell 虚拟项列表，二者任一变化都需要重新拉完整展示模型。
   */
  async function resolveFullRefreshSignature(folderPath: string): Promise<string> {
    const folderRevision = await getBoxFolderRevision(folderPath);

    return [
      folderPath,
      folderRevision,
      virtualShellIds.join("\u0000"),
    ].join("\u0001");
  }

  /**
   * 轮询结果未变化时不替换数组引用，避免 Vue 重新渲染整个文件网格造成可感知顿挫。
   */
  function updateBoxItemsIfChanged(nextItems: DesktopItem[]): void {
    if (areDesktopItemListsEqual(boxItems.value, nextItems)) {
      return;
    }

    boxItems.value = nextItems;
  }

  /**
   * 文件项展示字段完全一致时视为同一快照；图标 data URL 也纳入比较，确保真实图标变化仍会刷新。
   */
  function areDesktopItemListsEqual(left: DesktopItem[], right: DesktopItem[]): boolean {
    return (
      left.length === right.length &&
      left.every((leftItem, index) => areDesktopItemsEqual(leftItem, right[index]))
    );
  }

  /**
   * 比较展示层会读取的所有字段，避免只按路径判断而漏掉重命名、类型或图标变化。
   */
  function areDesktopItemsEqual(left: DesktopItem, right: DesktopItem | undefined): boolean {
    if (!right) {
      return false;
    }

    return (
      left.extension === right.extension &&
      left.iconDataUrl === right.iconDataUrl &&
      left.id === right.id &&
      left.kind === right.kind &&
      left.name === right.name &&
      left.path === right.path &&
      left.shellId === right.shellId &&
      left.source === right.source
    );
  }

  /**
   * 当前列表顺序是用户视觉上看到的真实顺序，保存前用它兜底未持久化过的 Box。
   */
  function resolveCurrentOrderedPaths(): string[] {
    return boxItems.value.map((item) => item.path);
  }

  /**
   * Box 内拖拽排序只改变当前 Box 的展示顺序，不移动、复制或重命名真实文件。
   */
  async function moveBoxItemsInOrder(
    draggedPaths: string[],
    targetPath: string | null,
    placement: "after" | "before" | "end",
  ): Promise<void> {
    const currentBox = options.box.value;
    if (!currentBox || draggedPaths.length === 0) {
      return;
    }

    const currentPaths = resolveCurrentOrderedPaths();
    const draggedPathSet = new Set(draggedPaths);
    const activeDraggedPaths = currentPaths.filter((path) => draggedPathSet.has(path));
    if (activeDraggedPaths.length === 0 || (targetPath && draggedPathSet.has(targetPath))) {
      return;
    }

    const remainingPaths = currentPaths.filter((path) => !draggedPathSet.has(path));
    const insertionIndex = resolveInsertionIndex(remainingPaths, targetPath, placement);
    const nextPaths = [...remainingPaths];
    nextPaths.splice(insertionIndex, 0, ...activeDraggedPaths);
    if (arePathOrdersEqual(currentPaths, nextPaths)) {
      return;
    }

    orderedPaths = nextPaths;
    boxItems.value = applyManualOrder(boxItems.value);
    await saveBoxItemOrder(currentBox.id, orderedPaths);
    await saveBoxVirtualItemIds(currentBox.id, resolveOrderedShellIdsFromPaths(orderedPaths));
  }

  /**
   * 系统桌面图标拖入 Box 时只追加引用，展示模型由后端按当前 Windows 环境重新解析。
   */
  async function appendBoxShellItems(shellIds: string[]): Promise<void> {
    const currentBox = options.box.value;
    if (!currentBox || shellIds.length === 0) {
      return;
    }

    await appendBoxVirtualItemIds(currentBox.id, shellIds);
    virtualShellIds = await loadBoxVirtualItemIds(currentBox.id);
    await syncShellIconVisibilityAfterReferenceChange();
  }

  /**
   * 文件重命名后真实路径变化，顺序表同步替换路径以保持原位置不跳到末尾。
   */
  async function replaceBoxItemOrderPath(previousPath: string, nextPath: string): Promise<void> {
    const currentBox = options.box.value;
    if (!currentBox || previousPath === nextPath) {
      return;
    }

    const baseOrder = orderedPaths.length > 0 ? orderedPaths : resolveCurrentOrderedPaths();
    orderedPaths = baseOrder.map((path) => (path === previousPath ? nextPath : path));
    await saveBoxItemOrder(currentBox.id, orderedPaths);
  }

  /**
   * 删除文件后清理顺序表中的旧路径，避免后续新增同名路径时继承已删除项目的位置。
   */
  async function removeBoxItemOrderPaths(paths: string[]): Promise<void> {
    const currentBox = options.box.value;
    if (!currentBox || orderedPaths.length === 0 || paths.length === 0) {
      return;
    }

    const removedPathSet = new Set(paths);
    orderedPaths = orderedPaths.filter((path) => !removedPathSet.has(path));
    await saveBoxItemOrder(currentBox.id, orderedPaths);
  }

  /**
   * 删除 Shell 项只清理 Box 引用和排序键，不能走真实文件删除链路。
   */
  async function removeBoxShellItems(shellIds: string[]): Promise<void> {
    const currentBox = options.box.value;
    if (!currentBox || shellIds.length === 0) {
      return;
    }

    await removeBoxVirtualItemIds(currentBox.id, shellIds);
    virtualShellIds = await loadBoxVirtualItemIds(currentBox.id);
    await removeBoxItemOrderPaths(shellIds.map((shellId) => `shell::${shellId}`));
    await syncShellIconVisibilityAfterReferenceChange();
  }

  /**
   * 系统图标显示同步失败不回滚 Box 引用，避免注册表权限或 Explorer 刷新异常破坏收纳结果。
   */
  async function syncShellIconVisibilityAfterReferenceChange(): Promise<void> {
    try {
      await syncShellDesktopIconVisibility({
        autoHideEnabled: options.getAutoHideNativeShellIcons(),
      });
    } catch (error) {
      options.setLastError(error instanceof Error ? error.message : String(error));
    }
  }

  /**
   * 拖拽释放到图标前半区插到目标前，释放到后半区插到目标后，空白处则追加到末尾。
   */
  function resolveInsertionIndex(
    paths: string[],
    targetPath: string | null,
    placement: "after" | "before" | "end",
  ): number {
    if (!targetPath || placement === "end") {
      return paths.length;
    }

    const targetIndex = paths.indexOf(targetPath);
    if (targetIndex === -1) {
      return paths.length;
    }

    return placement === "after" ? targetIndex + 1 : targetIndex;
  }

  /**
   * 顺序未变化时不写数据库，减少普通点击和短距离拖动带来的无意义持久化。
   */
  function arePathOrdersEqual(left: string[], right: string[]): boolean {
    return left.length === right.length && left.every((path, index) => path === right[index]);
  }

  /**
   * 虚拟项表只保存 shellId，排序表保存统一 path，因此同步时需要从 path 反解回 ID。
   */
  function resolveOrderedShellIdsFromPaths(paths: string[]): string[] {
    return paths
      .map((path) => path.match(/^shell::(.+)$/)?.[1])
      .filter((shellId): shellId is string => Boolean(shellId));
  }

  return {
    appendBoxShellItems,
    boxItems,
    moveBoxItemsInOrder,
    removeBoxItemOrderPaths,
    removeBoxShellItems,
    replaceBoxItemOrderPath,
    refreshBoxFolderItems,
    startFolderRefreshPolling,
    stopFolderRefreshPolling,
  };
}
