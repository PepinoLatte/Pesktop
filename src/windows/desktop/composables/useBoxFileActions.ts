import { nextTick, ref } from "vue";
import type { Ref } from "vue";
import {
  deleteDesktopItems,
  openDesktopItem,
  pasteDesktopItemsFromClipboard,
  renameDesktopItem,
  showNativeItemContextMenu,
  writeDesktopItemsToClipboard,
} from "@/entities/desktopItem/api";
import type { BoxConflictPolicy } from "@/entities/appSettings/types";
import type { DesktopItem } from "@/entities/desktopItem/types";

/**
 * 文件动作层依赖选择结果和刷新能力，但不直接维护扫描或框选状态。
 */
interface BoxFileActionsOptions {
  boxGridRef: Ref<HTMLElement | null>;
  closeContextMenu: () => void;
  getBoxConflictPolicy: () => BoxConflictPolicy;
  getBoxFolderPath: () => string;
  getDoubleClickOpenItems: () => boolean;
  removeBoxItemOrderPaths: (paths: string[]) => Promise<void>;
  removeBoxShellItems: (shellIds: string[]) => Promise<void>;
  refreshBoxFolderItems: (options?: { silent?: boolean }) => Promise<void>;
  replaceBoxItemOrderPath: (previousPath: string, nextPath: string) => Promise<void>;
  resolveSelectedItems: () => DesktopItem[];
  selectedPaths: Ref<Set<string>>;
  setLastError: (message: string) => void;
}

/**
 * Box 文件动作状态包含重命名编辑态和各类文件命令处理器。
 */
export interface BoxFileActionsState {
  cancelRename: () => void;
  commitRename: () => Promise<void>;
  copySelectedItems: () => Promise<void>;
  cutSelectedItems: () => Promise<void>;
  deleteSelectedItems: () => Promise<void>;
  editingPath: Ref<string | null>;
  handleItemContextMenu: (event: MouseEvent, item: DesktopItem) => void;
  handleItemDoubleClick: (item: DesktopItem) => Promise<void>;
  openItem: (item: DesktopItem) => Promise<void>;
  pasteClipboardItems: () => Promise<void>;
  renameDraft: Ref<string>;
  startSelectedItemRename: () => void;
}

/**
 * 管理 Box 文件项的系统动作，所有真实文件操作都继续交给后端或 Windows Shell 执行。
 */
export function useBoxFileActions(options: BoxFileActionsOptions): BoxFileActionsState {
  const editingPath = ref<string | null>(null);
  const renameDraft = ref("");

  /**
   * 打开文件项统一走系统默认方式，单击/双击/Enter 只决定触发时机，不改变打开语义。
   */
  async function openItem(item: DesktopItem): Promise<void> {
    try {
      await openDesktopItem(item.path);
    } catch (error) {
      options.setLastError(error instanceof Error ? error.message : String(error));
    }
  }

  /**
   * 双击文件项走系统默认打开方式，前端不判断具体扩展名或快捷方式目标。
   */
  async function handleItemDoubleClick(item: DesktopItem): Promise<void> {
    if (!options.getDoubleClickOpenItems()) {
      return;
    }

    await openItem(item);
  }

  /**
   * 右键先修正选区但不打开旧 Box 菜单，为后续文件级菜单保留准确上下文。
   */
  function handleItemContextMenu(event: MouseEvent, item: DesktopItem): void {
    options.boxGridRef.value?.focus();
    options.closeContextMenu();
    if (!options.selectedPaths.value.has(item.path)) {
      options.selectedPaths.value = new Set([item.path]);
    }
    event.preventDefault();
    void showNativeItemContextMenu(item.path, event.screenX, event.screenY)
      .then(() => options.refreshBoxFolderItems({ silent: true }))
      .catch((error) => {
        options.setLastError(error instanceof Error ? error.message : String(error));
      });
  }

  /**
   * F2 只编辑第一个选中项，符合 Explorer 多选时的基础重命名入口。
   */
  function startSelectedItemRename(): void {
    const item = options.resolveSelectedItems().find((selectedItem) => selectedItem.source !== "shell");
    if (!item) {
      return;
    }

    editingPath.value = item.path;
    renameDraft.value = item.name;
    void nextTick(() => {
      const input = options.boxGridRef.value?.querySelector<
        HTMLInputElement | HTMLTextAreaElement
      >("[data-rename-input='true']");
      input?.focus();
      input?.select();
    });
  }

  /**
   * 重命名只提交用户确认后的完整文件名，跨目录移动和冲突检查交给 Rust 层拦截。
   */
  async function commitRename(): Promise<void> {
    const targetPath = editingPath.value;
    const nextName = renameDraft.value.trim();
    if (!targetPath || !nextName) {
      editingPath.value = null;
      return;
    }

    try {
      const nextPath = await renameDesktopItem(targetPath, nextName);
      await options.replaceBoxItemOrderPath(targetPath, nextPath);
      editingPath.value = null;
      await options.refreshBoxFolderItems();
    } catch (error) {
      options.setLastError(error instanceof Error ? error.message : String(error));
    }
  }

  /**
   * 取消重命名只恢复前端编辑态，真实文件名保持不变。
   */
  function cancelRename(): void {
    editingPath.value = null;
    renameDraft.value = "";
  }

  /**
   * Delete 对真实文件走回收站，对 Shell 虚拟项只移除 Box 引用，避免误删系统对象。
   */
  async function deleteSelectedItems(): Promise<void> {
    const selectedItems = options.resolveSelectedItems();
    const filePaths = selectedItems
      .filter((item) => item.source !== "shell")
      .map((item) => item.path);
    const shellIds = selectedItems
      .map((item) => item.shellId)
      .filter((shellId): shellId is string => Boolean(shellId));
    if (filePaths.length === 0 && shellIds.length === 0) {
      return;
    }

    try {
      if (filePaths.length > 0) {
        await deleteDesktopItems(filePaths);
        await options.removeBoxItemOrderPaths(filePaths);
      }
      if (shellIds.length > 0) {
        await options.removeBoxShellItems(shellIds);
      }
      options.selectedPaths.value = new Set();
      await options.refreshBoxFolderItems();
    } catch (error) {
      options.setLastError(error instanceof Error ? error.message : String(error));
    }
  }

  /**
   * Ctrl+C 写入 Windows 文件剪贴板，后续可粘贴到 Box、Explorer 或其他支持 CF_HDROP 的程序。
   */
  async function copySelectedItems(): Promise<void> {
    await writeSelectedItemsToClipboard("copy");
  }

  /**
   * Ctrl+X 使用 Shell 标准剪切意图，粘贴到其他 Box 或 Explorer 时按移动语义处理。
   */
  async function cutSelectedItems(): Promise<void> {
    await writeSelectedItemsToClipboard("cut");
  }

  /**
   * Ctrl+V 从 Windows 文件剪贴板读取路径，复制或移动进当前 Box 后刷新文件网格。
   */
  async function pasteClipboardItems(): Promise<void> {
    const folderPath = options.getBoxFolderPath().trim();
    if (!folderPath) {
      return;
    }

    try {
      const didPaste = await pasteDesktopItemsFromClipboard(
        folderPath,
        options.getBoxConflictPolicy(),
      );
      if (didPaste) {
        await options.refreshBoxFolderItems();
      }
    } catch (error) {
      options.setLastError(error instanceof Error ? error.message : String(error));
    }
  }

  /**
   * 复制和剪切共享选区读取逻辑，空选区时保持和 Explorer 一样安静无动作。
   */
  async function writeSelectedItemsToClipboard(operation: "copy" | "cut"): Promise<void> {
    const paths = options
      .resolveSelectedItems()
      .filter((item) => item.source !== "shell")
      .map((item) => item.path);
    if (paths.length === 0) {
      return;
    }

    try {
      await writeDesktopItemsToClipboard(paths, operation);
    } catch (error) {
      options.setLastError(error instanceof Error ? error.message : String(error));
    }
  }

  return {
    cancelRename,
    commitRename,
    copySelectedItems,
    cutSelectedItems,
    deleteSelectedItems,
    editingPath,
    handleItemContextMenu,
    handleItemDoubleClick,
    openItem,
    pasteClipboardItems,
    renameDraft,
    startSelectedItemRename,
  };
}
