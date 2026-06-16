import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { getDesktopSnapshot } from "../../../shared/api/desktop";
import {
  deleteBoxRecord,
  initializeStorage,
  loadBoxes,
  loadSettings,
  saveBox,
  saveSetting,
} from "../../../shared/storage/database";
import {
  listenDesktopStateChanged,
  notifyDesktopStateChanged,
  type DesktopStateChangeScope,
} from "../../../shared/events/desktopEvents";
import type { AppSettings, DesktopBox, DesktopItem, ThemeMode } from "../../../shared/types/desktop";

/**
 * 默认尺寸需要保证能容纳标题栏、空状态和最小拖拽热区。
 */
const DEFAULT_BOX_SIZE = {
  width: 340,
  height: 280,
};

/**
 * 主题过渡类只在用户主动切换或系统主题变化时短暂启用，避免启动时出现不必要的闪动。
 */
const THEME_TRANSITION_CLASS = "theme-transition";
const THEME_TRANSITION_TIMEOUT_MS = 260;

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
  const isLoading = ref(true);
  const isInitialized = ref(false);
  const lastError = ref("");
  const settings = ref<AppSettings>({
    theme: "system",
    snapToEdges: true,
    snapThreshold: 20,
    showItemLabels: true,
  });
  const storeInstanceId = crypto.randomUUID();
  let stateListenerRegistered = false;
  let themeTransitionTimer: ReturnType<typeof window.setTimeout> | null = null;

  const unassignedItems = computed(() => {
    const assignedPaths = new Set(boxes.value.flatMap((box) => box.itemPaths));
    return items.value.filter((item) => !assignedPaths.has(item.path));
  });

  /**
   * 通过真实路径查找桌面项目，保证 Box 映射不依赖易变化的展示名称。
   */
  function findItem(path: string): DesktopItem | undefined {
    return items.value.find((item) => item.path === path);
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
      const [snapshot, savedBoxes, savedSettings] = await Promise.all([
        getDesktopSnapshot(),
        loadBoxes(),
        loadSettings(),
      ]);

      desktopPath.value = snapshot.desktopPath;
      items.value = snapshot.items;
      settings.value = savedSettings;
      applyTheme(settings.value.theme);
      boxes.value = (savedBoxes.length > 0 ? savedBoxes : [createDefaultBox()]).map((box) =>
        sanitizeBoxSize(box),
      );

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
    const [savedBoxes, savedSettings] = await Promise.all([loadBoxes(), loadSettings()]);

    const previousTheme = settings.value.theme;
    boxes.value = savedBoxes.map((savedBox) => sanitizeBoxSize(savedBox));
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
      x: 96 + boxes.value.length * 28,
      y: 96 + boxes.value.length * 28,
      ...DEFAULT_BOX_SIZE,
      itemPaths: [],
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

    const nextBoxes = boxes.value.map((box) => ({
      ...box,
      itemPaths: box.itemPaths.filter((path) => !uniqueItemPaths.includes(path)),
    }));
    const targetBox = nextBoxes.find((box) => box.id === boxId);

    if (!targetBox) {
      return;
    }

    targetBox.itemPaths.push(...uniqueItemPaths);
    boxes.value = nextBoxes;
    await Promise.all(nextBoxes.map((box) => saveBox(box)));
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
      x: 96,
      y: 96,
      ...DEFAULT_BOX_SIZE,
      itemPaths: [],
    };
  }

  /**
   * 更新主题时同步根节点标识，确保设置页和 Box 窗口一起响应。
   */
  async function updateTheme(theme: ThemeMode): Promise<void> {
    settings.value.theme = theme;
    applyTheme(theme, true);
    await saveSetting("theme", theme);
    await broadcastStateChanged("settings");
  }

  /**
   * 边缘吸附开关立即持久化，避免下次启动恢复旧交互习惯。
   */
  async function updateSnapToEdges(enabled: boolean): Promise<void> {
    settings.value.snapToEdges = enabled;
    await saveSetting("snapToEdges", enabled);
    await broadcastStateChanged("settings");
  }

  /**
   * 吸附阈值使用像素保存，和 Tauri 窗口移动事件的坐标体系保持一致。
   */
  async function updateSnapThreshold(threshold: number): Promise<void> {
    settings.value.snapThreshold = threshold;
    await saveSetting("snapThreshold", threshold);
    await broadcastStateChanged("settings");
  }

  /**
   * 图标名称显示属于纯渲染偏好，不影响 Box 与真实文件的映射关系。
   */
  async function updateShowItemLabels(value: boolean): Promise<void> {
    settings.value.showItemLabels = value;
    await saveSetting("showItemLabels", value);
    await broadcastStateChanged("settings");
  }

  /**
   * Box 的最小尺寸与窗口配置保持一致，避免拖动缩放后出现不可操作区域。
   */
  function sanitizeBoxSize(box: DesktopBox): DesktopBox {
    return {
      ...box,
      width: Math.max(box.width, 240),
      height: Math.max(box.height, 184),
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

    root.classList.add(THEME_TRANSITION_CLASS);
    /**
     * 强制浏览器先计算一次旧主题样式，随后切换 dark class 时才能拿到明确的过渡起点。
     */
    void root.offsetHeight;
    themeTransitionTimer = window.setTimeout(() => {
      root.classList.remove(THEME_TRANSITION_CLASS);
      themeTransitionTimer = null;
    }, THEME_TRANSITION_TIMEOUT_MS);
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
    handleItemDrop,
    initialize,
    isInitialized,
    isLoading,
    items,
    lastError,
    reloadPersistedState,
    refreshSnapshot,
    settings,
    unassignedItems,
    updateBox,
    updateShowItemLabels,
    updateSnapThreshold,
    updateSnapToEdges,
    updateTheme,
  };
});
