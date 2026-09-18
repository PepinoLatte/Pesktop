import { computed, ref, watch } from "vue";
import type { CSSProperties, Ref } from "vue";
import type { DesktopItem } from "@/entities/desktopItem/types";
import { DESKTOP_ICON_VIEW } from "@/windows/desktop/config/desktopIcon";
import {
  type GridPoint,
  type GridRect,
  normalizeRect,
  rectsIntersect,
} from "@/windows/desktop/model/selection";

/**
 * 文件选择层只关心选择状态和键鼠语义，真实文件动作由外部回调执行。
 */
interface BoxFileSelectionOptions {
  boxGridRef: Ref<HTMLElement | null>;
  boxItems: Ref<DesktopItem[]>;
  cancelFileRename: () => void;
  closeContextMenu: () => void;
  consumeSuppressedItemClick: () => boolean;
  copySelectedItems: () => Promise<void>;
  cutSelectedItems: () => Promise<void>;
  deleteSelectedItems: () => Promise<void>;
  getDoubleClickOpenItems: () => boolean;
  isEditingTitle: () => boolean;
  openItem: (item: DesktopItem) => Promise<void>;
  pasteClipboardItems: () => Promise<void>;
  startSelectedItemRename: () => void;
}

/**
 * Box 文件选择能力对外暴露给文件网格、文件操作和拖拽逻辑复用。
 */
export interface BoxFileSelectionState {
  handleFileViewKeydown: (event: KeyboardEvent) => Promise<void>;
  handleGlobalFileViewKeydown: (event: KeyboardEvent) => void;
  handleGridPointerDown: (event: PointerEvent) => void;
  handleItemClick: (event: MouseEvent, item: DesktopItem) => void;
  isItemSelected: (item: DesktopItem) => boolean;
  isSelecting: Readonly<Ref<boolean>>;
  resolveSelectedItems: () => DesktopItem[];
  selectedPaths: Ref<Set<string>>;
  selectionRectStyle: Readonly<Ref<CSSProperties>>;
  stopSelectionRectangle: () => void;
}

/**
 * 管理 Box 文件区的 Explorer 式选择语义，包括 Ctrl/Meta 多选、框选和快捷键。
 */
export function useBoxFileSelection(
  options: BoxFileSelectionOptions,
): BoxFileSelectionState {
  const selectedPaths = ref<Set<string>>(new Set());
  const selectionStart = ref<GridPoint | null>(null);
  const selectionCurrent = ref<GridPoint | null>(null);
  const isSelecting = computed(() => Boolean(selectionStart.value && selectionCurrent.value));
  let renameClickTimer: ReturnType<typeof window.setTimeout> | null = null;
  const selectionRectStyle = computed<CSSProperties>(() => {
    if (!selectionStart.value || !selectionCurrent.value) {
      return { display: "none" };
    }

    const rect = clampSelectionRectToViewport(
      normalizeRect(selectionStart.value, selectionCurrent.value),
    );

    return {
      display: rect.width > 0 && rect.height > 0 ? "block" : "none",
      height: `${rect.height}px`,
      left: `${rect.left}px`,
      top: `${rect.top}px`,
      width: `${rect.width}px`,
    };
  });

  watch(
    options.boxItems,
    (items) => {
      pruneSelection(items);
    },
  );

  /**
   * 刷新后只保留仍存在的选中项，避免外部删除文件后键盘操作命中旧路径。
   */
  function pruneSelection(items: DesktopItem[]): void {
    const existingPaths = new Set(items.map((item) => item.path));
    selectedPaths.value = new Set(
      [...selectedPaths.value].filter((path) => existingPaths.has(path)),
    );
  }

  /**
   * 选中态以真实路径为主键，刷新后同名文件也不会互相串选中状态。
   */
  function isItemSelected(item: DesktopItem): boolean {
    return selectedPaths.value.has(item.path);
  }

  /**
   * 单击负责选中，Ctrl/Meta 单击负责增删选区，行为尽量贴近 Explorer 的多选直觉。
   */
  function handleItemClick(event: MouseEvent, item: DesktopItem): void {
    if (options.consumeSuppressedItemClick()) {
      return;
    }

    options.boxGridRef.value?.focus();
    options.closeContextMenu();
    const wasOnlySelected = selectedPaths.value.size === 1 && selectedPaths.value.has(item.path);
    clearScheduledRename();
    if (event.detail > DESKTOP_ICON_VIEW.openClickDetail) {
      return;
    }

    if (event.ctrlKey || event.metaKey) {
      const nextSelection = new Set(selectedPaths.value);
      if (nextSelection.has(item.path)) {
        nextSelection.delete(item.path);
      } else {
        nextSelection.add(item.path);
      }
      selectedPaths.value = nextSelection;
      return;
    }

    selectedPaths.value = new Set([item.path]);
    if (!options.getDoubleClickOpenItems() && event.detail === DESKTOP_ICON_VIEW.openClickDetail) {
      void options.openItem(item);
      return;
    }

    if (wasOnlySelected && options.getDoubleClickOpenItems()) {
      scheduleSelectedItemRename(item.path);
    }
  }

  /**
   * 空白区域按下鼠标才启动框选，避免和图标点击、窗口拖动、缩放热区互相抢事件。
   */
  function handleGridPointerDown(event: PointerEvent): void {
    if (event.button !== 0 || !options.boxGridRef.value) {
      return;
    }
    clearScheduledRename();
    const target = event.target;
    if (target instanceof HTMLElement && target.closest("[data-box-item-path]")) {
      return;
    }

    options.boxGridRef.value.focus();
    const point = resolveGridLocalPoint(event);
    selectionStart.value = point;
    selectionCurrent.value = point;
    if (!event.ctrlKey && !event.metaKey) {
      selectedPaths.value = new Set();
    }
    window.addEventListener("pointermove", handleSelectionPointerMove, { capture: true });
    window.addEventListener("pointerup", handleSelectionPointerUp, { capture: true, once: true });
    window.addEventListener("pointercancel", handleSelectionPointerUp, {
      capture: true,
      once: true,
    });
  }

  /**
   * 框选移动阶段实时换算网格内坐标，滚动区域内也能得到稳定选择矩形。
   */
  function handleSelectionPointerMove(event: PointerEvent): void {
    selectionCurrent.value = resolveGridLocalPoint(event);
    applySelectionRectangle();
  }

  /**
   * 鼠标释放时做最后一次命中计算，再清理全局 pointer 监听。
   */
  function handleSelectionPointerUp(): void {
    applySelectionRectangle();
    stopSelectionRectangle();
  }

  /**
   * 框选监听挂在 window 上，清理必须成对执行，防止指针离开 Box 后残留选择状态。
   */
  function stopSelectionRectangle(): void {
    window.removeEventListener("pointermove", handleSelectionPointerMove, { capture: true });
    window.removeEventListener("pointerup", handleSelectionPointerUp, { capture: true });
    window.removeEventListener("pointercancel", handleSelectionPointerUp, { capture: true });
    selectionStart.value = null;
    selectionCurrent.value = null;
  }

  /**
   * 已选中文件再次单击后延迟进入重命名；双击会在第二次 click 时取消该计时器。
   */
  function scheduleSelectedItemRename(path: string): void {
    renameClickTimer = window.setTimeout(() => {
      renameClickTimer = null;
      if (selectedPaths.value.size !== 1 || !selectedPaths.value.has(path)) {
        return;
      }

      options.startSelectedItemRename();
    }, DESKTOP_ICON_VIEW.renameClickDelayMs);
  }

  /**
   * 用户继续双击、框选、按快捷键或切换选择时，待触发重命名都应立即取消。
   */
  function clearScheduledRename(): void {
    if (!renameClickTimer) {
      return;
    }

    window.clearTimeout(renameClickTimer);
    renameClickTimer = null;
  }

  /**
   * 将视口坐标换算为滚动容器内坐标，保证滚动后框选命中仍与视觉位置一致。
   */
  function resolveGridLocalPoint(event: PointerEvent): GridPoint {
    const gridRect = options.boxGridRef.value?.getBoundingClientRect();
    if (!gridRect) {
      return { x: 0, y: 0 };
    }

    return {
      x: event.clientX - gridRect.left + (options.boxGridRef.value?.scrollLeft ?? 0),
      y: event.clientY - gridRect.top + (options.boxGridRef.value?.scrollTop ?? 0),
    };
  }

  /**
   * 框选视觉层位于滚动内容内部，必须裁到当前可见视口，避免拖到边缘时扩大 scrollWidth 或 scrollHeight。
   */
  function clampSelectionRectToViewport(rect: GridRect): GridRect {
    const gridElement = options.boxGridRef.value;
    if (!gridElement) {
      return rect;
    }

    const viewportLeft = gridElement.scrollLeft;
    const viewportTop = gridElement.scrollTop;
    const viewportRight = viewportLeft + gridElement.clientWidth;
    const viewportBottom = viewportTop + gridElement.clientHeight;
    const rectRight = rect.left + rect.width;
    const rectBottom = rect.top + rect.height;
    const left = Math.max(rect.left, viewportLeft);
    const top = Math.max(rect.top, viewportTop);
    const right = Math.min(rectRight, viewportRight);
    const bottom = Math.min(rectBottom, viewportBottom);

    return {
      height: Math.max(0, bottom - top),
      left,
      top,
      width: Math.max(0, right - left),
    };
  }

  /**
   * 使用 DOM 矩形做交集判断，避免根据网格列数推断位置时受字体和缩放影响。
   */
  function applySelectionRectangle(): void {
    if (!options.boxGridRef.value || !selectionStart.value || !selectionCurrent.value) {
      return;
    }

    const selectionRect = normalizeRect(selectionStart.value, selectionCurrent.value);
    const gridRect = options.boxGridRef.value.getBoundingClientRect();
    const nextSelection = new Set(selectedPaths.value);
    for (const item of options.boxItems.value) {
      const element = options.boxGridRef.value.querySelector<HTMLElement>(
        `[data-box-item-path="${CSS.escape(item.path)}"]`,
      );
      if (!element) {
        continue;
      }

      const elementRect = element.getBoundingClientRect();
      const itemRect = {
        height: elementRect.height,
        left: elementRect.left - gridRect.left + options.boxGridRef.value.scrollLeft,
        top: elementRect.top - gridRect.top + options.boxGridRef.value.scrollTop,
        width: elementRect.width,
      };
      if (rectsIntersect(selectionRect, itemRect)) {
        nextSelection.add(item.path);
      }
    }
    selectedPaths.value = nextSelection;
  }

  /**
   * 文件区键盘快捷键覆盖高频整理操作，标题编辑中会跳过避免误删或误改文件。
   */
  async function handleFileViewKeydown(event: KeyboardEvent): Promise<void> {
    if (isEditableKeyboardTarget(event)) {
      return;
    }

    if (options.isEditingTitle()) {
      return;
    }

    clearScheduledRename();
    const selectedItems = resolveSelectedItems();
    const normalizedKey = event.key.toLowerCase();
    if ((event.ctrlKey || event.metaKey) && normalizedKey === "a") {
      selectedPaths.value = new Set(options.boxItems.value.map((item) => item.path));
      event.preventDefault();
      return;
    }
    if ((event.ctrlKey || event.metaKey) && normalizedKey === "c") {
      event.preventDefault();
      await options.copySelectedItems();
      return;
    }
    if ((event.ctrlKey || event.metaKey) && normalizedKey === "x") {
      event.preventDefault();
      await options.cutSelectedItems();
      return;
    }
    if ((event.ctrlKey || event.metaKey) && normalizedKey === "v") {
      event.preventDefault();
      await options.pasteClipboardItems();
      return;
    }
    if (event.key === "F2") {
      options.startSelectedItemRename();
      event.preventDefault();
      return;
    }
    if (event.key === "Delete") {
      event.preventDefault();
      await options.deleteSelectedItems();
      return;
    }
    if (event.key === "Enter" && selectedItems[0]) {
      event.preventDefault();
      await options.openItem(selectedItems[0]);
      return;
    }
    if (event.key === "Escape") {
      options.cancelFileRename();
      selectedPaths.value = new Set();
      event.preventDefault();
    }
  }

  /**
   * F2/Delete/Enter 不依赖文件网格是否正好拿到焦点，避免点击图标后快捷键被窗口吞掉。
   */
  function handleGlobalFileViewKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || shouldIgnoreGlobalFileShortcut()) {
      return;
    }

    void handleFileViewKeydown(event);
  }

  /**
   * 输入框和可编辑区域保留系统键盘行为，避免文件重命名或标题编辑时触发全局快捷键。
   */
  function shouldIgnoreGlobalFileShortcut(): boolean {
    return isEditableElement(document.activeElement);
  }

  /**
   * 网格自身的 keydown 也会收到输入框冒泡事件，必须在入口放行 Ctrl+A/C/V/X 等原生编辑快捷键。
   */
  function isEditableKeyboardTarget(event: KeyboardEvent): boolean {
    return isEditableElement(event.target instanceof Element ? event.target : null);
  }

  /**
   * 可编辑元素统一保留浏览器默认文本编辑语义，避免文件区和输入框争抢同一组快捷键。
   */
  function isEditableElement(element: Element | null): boolean {
    if (!element) {
      return false;
    }

    if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
      return true;
    }

    return element instanceof HTMLElement && element.isContentEditable;
  }

  /**
   * 选中项每次从最新扫描结果解析，避免文件刷新后继续操作已不存在的路径。
   */
  function resolveSelectedItems(): DesktopItem[] {
    return options.boxItems.value.filter((item) => selectedPaths.value.has(item.path));
  }

  return {
    handleFileViewKeydown,
    handleGlobalFileViewKeydown,
    handleGridPointerDown,
    handleItemClick,
    isItemSelected,
    isSelecting,
    resolveSelectedItems,
    selectedPaths,
    selectionRectStyle,
    stopSelectionRectangle,
  };
}
