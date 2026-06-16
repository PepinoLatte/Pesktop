import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { getDesktopSnapshot } from "../../../shared/api/desktop";
import {
  assignBoxItems,
  deleteBoxRecord,
  initializeStorage,
  loadBoxItems,
  loadBoxes,
  loadSettings,
  saveBox,
  saveSetting,
} from "../../../shared/storage/database";
import { APP_SETTING_KEYS, DEFAULT_APP_SETTINGS } from "../../../shared/config/appSettings";
import {
  BOX_WINDOW_PLACEMENT,
  BOX_WINDOW_SIZE,
  THEME_TRANSITION_CONFIG,
} from "../../../shared/config/desktopLayout";
import {
  listenDesktopStateChanged,
  notifyDesktopStateChanged,
  type DesktopStateChangeScope,
} from "../../../shared/events/desktopEvents";
import type {
  AppSettings,
  DesktopBox,
  DesktopBoxItem,
  DesktopItem,
  DesktopNameDisplayMode,
  ThemeMode,
} from "../../../shared/types/desktop";

/**
 * 更新 Box 时允许调用方控制是否清洗尺寸和广播事件，缩放等高频场景需要避免跨窗口刷新抖动。
 */
interface UpdateBoxOptions {
  broadcast?: boolean;
  sanitize?: boolean;
}

/**
 * 桌面 Store 只维护 Box 映射和展示偏好，不移动、改名或删除真实桌面文件。
 */
export const useDesktopStore = defineStore("desktop", () => {
  const desktopPath = ref("");
  const items = ref<DesktopItem[]>([]);
  const boxes = ref<DesktopBox[]>([]);
  const boxItems = ref<DesktopBoxItem[]>([]);
  const isLoading = ref(true);
  const isInitialized = ref(false);
  const lastError = ref("");
  const settings = ref<AppSettings>({ ...DEFAULT_APP_SETTINGS });
  const storeInstanceId = crypto.randomUUID();
  let stateListenerRegistered = false;
  let themeTransitionTimer: ReturnType<typeof window.setTimeout> | null = null;

  /**
   * 桌面视图只显示尚未归入 Box 的项目；拖入 Box 后通过映射过滤实现“桌面消失”，不移动真实文件。
   */
  const unassignedItems = computed(() => {
    const assignedPaths = new Set(boxItems.value.map((boxItem) => boxItem.itemPath));
    return items.value.filter((item) => !assignedPaths.has(item.path));
  });

  /**
   * 所有 Box 的项目总数直接来自关联表，避免窗口布局数据承担统计职责。
   */
  const totalBoxItems = computed(() => boxItems.value.length);

  /**
   * 通过真实路径查找桌面项目，保证 Box 映射不依赖易变化的展示名称。
   */
  function findItem(path: string): DesktopItem | undefined {
    return items.value.find((item) => item.path === path);
  }

  /**
   * 通过关联表读取目标 Box 的路径列表，保持窗口布局对象只承载几何信息。
   */
  function getBoxItemPaths(boxId: string): string[] {
    return boxItems.value
      .filter((boxItem) => boxItem.boxId === boxId)
      .map((boxItem) => boxItem.itemPath);
  }

  /**
   * Box 渲染时从桌面快照反查项目详情，桌面文件不存在时自动不显示悬空映射。
   */
  function getBoxItems(boxId: string): DesktopItem[] {
    return getBoxItemPaths(boxId)
      .map((path) => findItem(path))
      .filter((item) => item !== undefined);
  }

  /**
   * 初始化桌面快照、Box 布局和设置；同一窗口生命周期内只执行一次。
   */
  async function initialize(): Promise<void> {
    if (isInitialized.value) {
      return;
    }

    isLoading.value = true;
    lastError.value = "";

    try {
      await initializeStorage();
      registerStateListener();
      const [snapshot, savedBoxes, savedBoxItems, savedSettings] = await Promise.all([
        getDesktopSnapshot(),
        loadBoxes(),
        loadBoxItems(),
        loadSettings(),
      ]);

      desktopPath.value = snapshot.desktopPath;
      items.value = snapshot.items;
      settings.value = savedSettings;
      applyTheme(settings.value.theme);
      boxes.value = (savedBoxes.length > 0 ? savedBoxes : [createDefaultBox()]).map((box) =>
        sanitizeBoxSize(box),
      );
      boxItems.value = savedBoxItems;

      await Promise.all(boxes.value.map((box) => saveBox(box)));
      isInitialized.value = true;
    } catch (error) {
      lastError.value = error instanceof Error ? error.message : String(error);
    } finally {
      isLoading.value = false;
    }
  }

  /**
   * 主动刷新桌面目录快照，用于用户手动同步新增或删除的桌面文件。
   */
  async function refreshSnapshot(shouldBroadcast = true): Promise<void> {
    const snapshot = await getDesktopSnapshot();
    desktopPath.value = snapshot.desktopPath;
    items.value = snapshot.items;

    if (shouldBroadcast) {
      await broadcastStateChanged("desktop");
    }
  }

  /**
   * 设置窗重新获得焦点时从 SQLite 读取最新状态，解决多个独立 Box 窗口各自持有 Store 的问题。
   */
  async function reloadPersistedState(): Promise<void> {
    const [savedBoxes, savedBoxItems, savedSettings] = await Promise.all([
      loadBoxes(),
      loadBoxItems(),
      loadSettings(),
    ]);

    const previousTheme = settings.value.theme;
    boxes.value = savedBoxes.map((savedBox) => sanitizeBoxSize(savedBox));
    boxItems.value = savedBoxItems;
    settings.value = savedSettings;
    applyTheme(savedSettings.theme, previousTheme !== savedSettings.theme);
  }

  /**
   * 跨窗口监听只注册一次，接收通知后从 SQLite 重新读取，避免信任可能过期的事件载荷。
   */
  function registerStateListener(): void {
    if (stateListenerRegistered) {
      return;
    }

    stateListenerRegistered = true;
    void listenDesktopStateChanged(async ({ payload }) => {
      if (payload.sourceId === storeInstanceId || !isInitialized.value) {
        return;
      }

      await reloadPersistedState();
      if (payload.scope === "desktop") {
        await refreshSnapshot(false);
      }
    }).catch((error) => {
      stateListenerRegistered = false;
      lastError.value = error instanceof Error ? error.message : String(error);
    });
  }

  /**
   * 状态广播只作为失效通知使用，接收方会自行读取数据库中的最新状态。
   */
  async function broadcastStateChanged(scope: DesktopStateChangeScope): Promise<void> {
    await notifyDesktopStateChanged({
      sourceId: storeInstanceId,
      scope,
    });
  }

  /**
   * 创建一个新的独立 Box，默认位置错开，避免覆盖现有窗口。
   */
  async function createBox(title = "新建 Box"): Promise<DesktopBox> {
    const nextBox = sanitizeBoxSize({
      id: crypto.randomUUID(),
      title,
      x: BOX_WINDOW_PLACEMENT.initialX + boxes.value.length * BOX_WINDOW_PLACEMENT.cascadeStep,
      y: BOX_WINDOW_PLACEMENT.initialY + boxes.value.length * BOX_WINDOW_PLACEMENT.cascadeStep,
      ...BOX_WINDOW_SIZE.default,
    });

    boxes.value.push(nextBox);
    await saveBox(nextBox);
    await broadcastStateChanged("boxes");
    return nextBox;
  }

  /**
   * 更新 Box 的布局或映射，尺寸默认会经过最小可操作范围保护。
   */
  async function updateBox(nextBox: DesktopBox, options: UpdateBoxOptions = {}): Promise<void> {
    const index = boxes.value.findIndex((box) => box.id === nextBox.id);
    if (index === -1) {
      return;
    }

    const { broadcast = true, sanitize = true } = options;
    const normalizedBox = sanitize ? sanitizeBoxSize(nextBox) : nextBox;
    boxes.value[index] = normalizedBox;
    await saveBox(normalizedBox);

    if (broadcast) {
      await broadcastStateChanged("boxes");
    }
  }

  /**
   * 文件只会被重新映射到目标 Box，不会移动、改名或删除真实桌面文件。
   */
  async function assignItemToBox(itemPath: string, boxId: string): Promise<void> {
    await assignItemsToBox([itemPath], boxId);
  }

  /**
   * 批量映射用于原生文件拖放，保证一次 Drop 只触发一次数据库写入。
   */
  async function assignItemsToBox(itemPaths: string[], boxId: string): Promise<void> {
    const uniqueItemPaths = Array.from(new Set(itemPaths)).filter(Boolean);
    if (uniqueItemPaths.length === 0) {
      return;
    }

    await assignBoxItems(boxId, uniqueItemPaths);
    boxItems.value = boxItems.value.filter((boxItem) => !uniqueItemPaths.includes(boxItem.itemPath));
    boxItems.value.push(
      ...uniqueItemPaths.map((itemPath) => ({
        boxId,
        itemPath,
      })),
    );
    await broadcastStateChanged("boxes");
  }

  /**
   * 原生 Windows 桌面拖入时只接收当前桌面目录下的文件，避免 Box 变成任意文件收藏夹。
   */
  async function assignDroppedPathsToBox(paths: string[], boxId: string): Promise<void> {
    await refreshSnapshot(false);

    const desktopItemPaths = new Set(items.value.map((item) => item.path));
    const acceptedPaths = paths.filter((path) => desktopItemPaths.has(path));

    await assignItemsToBox(acceptedPaths, boxId);
  }

  /**
   * 删除 Box 只删除分组映射，真实桌面文件继续保留在 Explorer 管理下。
   */
  async function deleteBox(boxId: string): Promise<void> {
    boxes.value = boxes.value.filter((box) => box.id !== boxId);
    boxItems.value = boxItems.value.filter((boxItem) => boxItem.boxId !== boxId);
    await deleteBoxRecord(boxId);
    await broadcastStateChanged("boxes");
  }

  /**
   * Drop 事件只接受路径文本，避免把浏览器临时拖拽数据写入映射。
   */
  async function handleItemDrop(event: DragEvent, boxId: string): Promise<void> {
    const itemPath = event.dataTransfer?.getData("text/plain");
    if (!itemPath) {
      return;
    }

    await assignItemToBox(itemPath, boxId);
  }

  /**
   * 首次启动时创建一个默认 Box，让用户能立即看到桌面扩展本体。
   */
  function createDefaultBox(): DesktopBox {
    return {
      id: crypto.randomUUID(),
      title: "工作",
      x: BOX_WINDOW_PLACEMENT.initialX,
      y: BOX_WINDOW_PLACEMENT.initialY,
      ...BOX_WINDOW_SIZE.default,
    };
  }

  /**
   * 更新主题时同步根节点标识，确保设置页和 Box 窗口一起响应。
   */
  async function updateTheme(theme: ThemeMode): Promise<void> {
    settings.value.theme = theme;
    applyTheme(theme, true);
    await saveSetting(APP_SETTING_KEYS.theme, theme);
    await broadcastStateChanged("settings");
  }

  /**
   * 边缘吸附开关立即持久化，避免下次启动恢复旧交互习惯。
   */
  async function updateSnapToEdges(enabled: boolean): Promise<void> {
    settings.value.snapToEdges = enabled;
    await saveSetting(APP_SETTING_KEYS.snapToEdges, enabled);
    await broadcastStateChanged("settings");
  }

  /**
   * 吸附阈值使用像素保存，和 Tauri 窗口移动事件的坐标体系保持一致。
   */
  async function updateSnapThreshold(threshold: number): Promise<void> {
    settings.value.snapThreshold = threshold;
    await saveSetting(APP_SETTING_KEYS.snapThreshold, threshold);
    await broadcastStateChanged("settings");
  }

  /**
   * 图标名称显示属于纯渲染偏好，不影响 Box 与真实文件的映射关系。
   */
  async function updateShowItemLabels(value: boolean): Promise<void> {
    settings.value.showItemLabels = value;
    await saveSetting(APP_SETTING_KEYS.showItemLabels, value);
    await broadcastStateChanged("settings");
  }

  /**
   * 快捷方式箭头只影响 Dasktop 的视觉提示，不改 Windows 快捷方式文件本身。
   */
  async function updateShowShortcutArrow(value: boolean): Promise<void> {
    settings.value.showShortcutArrow = value;
    await saveSetting(APP_SETTING_KEYS.showShortcutArrow, value);
    await broadcastStateChanged("settings");
  }

  /**
   * 打开方式只影响 Dasktop 图标的点击交互，不改变系统默认打开程序。
   */
  async function updateDoubleClickOpenItems(value: boolean): Promise<void> {
    settings.value.doubleClickOpenItems = value;
    await saveSetting(APP_SETTING_KEYS.doubleClickOpenItems, value);
    await broadcastStateChanged("settings");
  }

  /**
   * 文件名显示模式用于统一控制 Box 内后缀呈现，避免同一路径在多个 Box 中展示不一致。
   */
  async function updateNameDisplayMode(value: DesktopNameDisplayMode): Promise<void> {
    settings.value.nameDisplayMode = value;
    await saveSetting(APP_SETTING_KEYS.nameDisplayMode, value);
    await broadcastStateChanged("settings");
  }

  /**
   * Box 的最小尺寸与窗口配置保持一致，避免拖动缩放后出现不可操作区域。
   */
  function sanitizeBoxSize(box: DesktopBox): DesktopBox {
    return {
      ...box,
      width: Math.max(box.width, BOX_WINDOW_SIZE.min.width),
      height: Math.max(box.height, BOX_WINDOW_SIZE.min.height),
    };
  }

  /**
   * 跟随系统只影响渲染标识，不把解析后的明暗值写回数据库。
   */
  function applyTheme(theme: ThemeMode, shouldAnimate = false): void {
    const root = document.documentElement;
    const systemDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    const resolvedTheme = theme === "system" ? (systemDark ? "dark" : "light") : theme;

    if (shouldAnimate) {
      enableThemeTransition(root);
    }

    root.dataset.themeMode = theme;
    root.dataset.theme = resolvedTheme;
    root.classList.toggle("dark", resolvedTheme === "dark");
  }

  /**
   * 主题动画尊重系统的减少动态效果设置，只做短时颜色过渡，不影响拖动和缩放性能。
   */
  function enableThemeTransition(root: HTMLElement): void {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      return;
    }

    if (themeTransitionTimer) {
      window.clearTimeout(themeTransitionTimer);
    }

    root.classList.add(THEME_TRANSITION_CONFIG.className);
    /**
     * 强制浏览器先计算一次旧主题样式，随后切换 dark class 时才能拿到明确的过渡起点。
     */
    void root.offsetHeight;
    themeTransitionTimer = window.setTimeout(() => {
      root.classList.remove(THEME_TRANSITION_CONFIG.className);
      themeTransitionTimer = null;
    }, THEME_TRANSITION_CONFIG.timeoutMs);
  }

  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    if (settings.value.theme === "system") {
      applyTheme(settings.value.theme, true);
    }
  });

  return {
    assignItemToBox,
    assignDroppedPathsToBox,
    boxes,
    createBox,
    deleteBox,
    desktopPath,
    findItem,
    getBoxItemPaths,
    getBoxItems,
    handleItemDrop,
    initialize,
    isInitialized,
    isLoading,
    items,
    lastError,
    reloadPersistedState,
    refreshSnapshot,
    settings,
    boxItems,
    totalBoxItems,
    unassignedItems,
    updateBox,
    updateDoubleClickOpenItems,
    updateNameDisplayMode,
    updateShowItemLabels,
    updateShowShortcutArrow,
    updateSnapThreshold,
    updateSnapToEdges,
    updateTheme,
  };
});
