<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { Boxes, Info, MonitorCog, Palette } from "@lucide/vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import AboutPanel from "./components/AboutPanel.vue";
import AppearancePanel from "./components/AppearancePanel.vue";
import BoxDisplayPanel from "./components/BoxDisplayPanel.vue";
import BoxesPanel from "./components/BoxesPanel.vue";
import SettingsHeader from "./components/SettingsHeader.vue";
import SettingsSidebar from "./components/SettingsSidebar.vue";
import {
  isAutostartEnabled,
  setAutostartEnabled,
} from "@/entities/appSettings/api";
import { useDesktopStore } from "@/entities/desktopBox/store";
import { chooseCollectionRootFolder, openBoxFolder } from "@/entities/desktopBox/api";
import { closeBoxWindow, openBoxWindow, showBoxWindow } from "@/entities/desktopBox/windows";
import { SETTINGS_WINDOW_SYNC_TIMING } from "@/entities/desktopBox/layout";
import {
  listenAutostartChanged,
  listenTrayCreateBox,
} from "@/shared/ipc/appTray";
import {
  createDesktopStartupSnapshotServer,
  listenBoxWindowReady,
} from "@/shared/ipc/desktop";
import type { Component } from "vue";
import type { DesktopBox } from "@/entities/desktopBox/types";

/**
 * 设置页的导航键值统一收口，避免侧栏、标题和内容区域各自维护字符串
 */
type SettingsSection = "boxes" | "boxDisplay" | "appearance" | "about";

/**
 * 设置页左侧导航项，icon 使用组件类型以便保持 Lucide 图标风格一致
 */
interface SettingsNavItem {
  key: SettingsSection;
  label: string;
  icon: Component;
}

/**
 * 设置页内容宽度由父窗口统一定义，再下发给各面板以保持视觉密度一致
 */
const SETTINGS_PANEL_WIDTH = {
  default: "max-w-[780px]",
  wide: "max-w-[820px]",
} as const;

const currentWindow = getCurrentWindow();
const desktopStore = useDesktopStore();
const activeSection = ref<SettingsSection>("boxes");
const startupError = ref("");
const autostartEnabled = ref(false);
const isMigratingCollectionRoot = ref(false);
const unlistenFns: UnlistenFn[] = [];
/**
 * 设置窗获得焦点时不立刻刷新桌面快照，避免 Shell 缩略图扫描卡住标题栏拖动首帧
 */
const FOCUS_SYNC_DELAY_MS = SETTINGS_WINDOW_SYNC_TIMING.focusSyncDelayMs;
const DRAG_RELEASE_FALLBACK_MS = SETTINGS_WINDOW_SYNC_TIMING.dragReleaseFallbackMs;
const BOX_STARTUP_READY_WAIT_MS = 8_000;
let focusSyncTimer: ReturnType<typeof window.setTimeout> | null = null;
let isDraggingSettingsWindow = false;
let dragReleaseCleanup: (() => void) | null = null;

/**
 * 设置页只保留当前真实可用的配置入口，旧占位导航不再兼容
 */
const sections: SettingsNavItem[] = [
  { key: "boxes", label: "Box", icon: Boxes },
  { key: "boxDisplay", label: "设置", icon: MonitorCog },
  { key: "appearance", label: "外观", icon: Palette },
  { key: "about", label: "关于", icon: Info },
];

const activeTitle = computed(
  () => sections.find((section) => section.key === activeSection.value)?.label ?? "设置",
);
const migratableBoxCount = computed(() => desktopStore.getMigratableBoxes().length);
onMounted(async () => {
  await desktopStore.initialize();
  await syncAutostartEnabled();
  unlistenFns.push(
    await listenAutostartChanged(({ payload }) => {
      autostartEnabled.value = payload;
    }),
  );
  unlistenFns.push(
    await listenTrayCreateBox(async () => {
      await createAndOpenBox();
    }),
  );
  unlistenFns.push(
    await currentWindow.onFocusChanged(({ payload }) => {
      if (!payload) {
        clearFocusSyncTimer();
        return;
      }

      scheduleFocusSync();
    }),
  );

  if (await openAllBoxes()) {
    await currentWindow.hide();
  }
});

/**
 * 组件销毁时注销窗口事件，避免开发热更新后重复刷新状态
 */
onUnmounted(() => {
  clearFocusSyncTimer();
  clearDragReleaseListeners();
  for (const unlisten of unlistenFns) {
    unlisten();
  }
});

/**
 * 设置页启动时批量打开现有 Box，避免窗口数量多时串行等待拖慢桌面恢复
 */
async function openAllBoxes(): Promise<boolean> {
  startupError.value = "";
  const startupBoxes = [...desktopStore.boxes];
  if (startupBoxes.length === 0) {
    return true;
  }

  try {
    const [readyTracker, startupSnapshotServer] = await Promise.all([
      createBoxWindowReadyTracker(),
      createDesktopStartupSnapshotServer(desktopStore.createStartupSnapshot()),
    ]);

    try {
      const openResults = await Promise.allSettled(
        startupBoxes.map((box) =>
          openBoxWindow(box, {
            focus: false,
            startupSnapshotToken: startupSnapshotServer.token,
            /**
             * WebView 文件区首帧依赖窗口完成布局，启动恢复先显示再等待 ready 可减少空白闪烁。
             */
            visible: true,
          }),
        ),
      );
      const openedBoxes = startupBoxes.filter(
        (_, index) => openResults[index]?.status === "fulfilled",
      );
      const failedOpenResults = openResults.filter(
        (result): result is PromiseRejectedResult => result.status === "rejected",
      );
      await readyTracker.waitFor(
        openedBoxes.map((box) => box.id),
        BOX_STARTUP_READY_WAIT_MS,
      );
      const showResults = await Promise.allSettled(
        openedBoxes.map((box) => showBoxWindow(box, { focus: false })),
      );
      const failedShowResults = showResults.filter(
        (result): result is PromiseRejectedResult => result.status === "rejected",
      );
      const failedResults = [...failedOpenResults, ...failedShowResults];

      if (failedResults.length === 0) {
        return true;
      }

      startupError.value = formatBatchOpenBoxError(failedResults);
      return false;
    } finally {
      readyTracker.dispose();
      startupSnapshotServer.dispose();
    }
  } catch (error) {
    startupError.value = error instanceof Error ? error.message : String(error);
    return false;
  }
}

/**
 * 启动恢复时先监听 ready，再创建隐藏 Box，避免 ready 事件早于监听注册导致统一显示卡住
 */
async function createBoxWindowReadyTracker(): Promise<{
  dispose: () => void;
  waitFor: (boxIds: string[], timeoutMs: number) => Promise<boolean>;
}> {
  const readyBoxIds = new Set<string>();
  const waiters: Array<() => void> = [];
  const unlisten = await listenBoxWindowReady(({ payload }) => {
    readyBoxIds.add(payload.boxId);
    for (const waiter of [...waiters]) {
      waiter();
    }
  });

  return {
    dispose: () => {
      unlisten();
      waiters.splice(0);
    },
    waitFor: (boxIds, timeoutMs) =>
      waitForReadyBoxIds(boxIds, timeoutMs, readyBoxIds, waiters),
  };
}

/**
 * 等待目标 Box 首帧准备完成；超时后继续展示窗口，防止异常窗口永久隐藏
 */
function waitForReadyBoxIds(
  boxIds: string[],
  timeoutMs: number,
  readyBoxIds: Set<string>,
  waiters: Array<() => void>,
): Promise<boolean> {
  const targetBoxIds = Array.from(new Set(boxIds));
  if (areAllBoxesReady(targetBoxIds, readyBoxIds)) {
    return Promise.resolve(true);
  }

  return new Promise<boolean>((resolve) => {
    let isSettled = false;
    const settle = (isReady: boolean): void => {
      if (isSettled) {
        return;
      }

      isSettled = true;
      window.clearTimeout(timeoutTimer);
      const waiterIndex = waiters.indexOf(waiter);
      if (waiterIndex >= 0) {
        waiters.splice(waiterIndex, 1);
      }
      resolve(isReady);
    };
    const waiter = (): void => {
      if (areAllBoxesReady(targetBoxIds, readyBoxIds)) {
        settle(true);
      }
    };
    const timeoutTimer = window.setTimeout(() => settle(false), timeoutMs);

    waiters.push(waiter);
    waiter();
  });
}

/**
 * 空列表视为已准备，方便没有成功打开窗口时直接进入错误处理分支
 */
function areAllBoxesReady(boxIds: string[], readyBoxIds: Set<string>): boolean {
  return boxIds.every((boxId) => readyBoxIds.has(boxId));
}

/**
 * 批量打开失败时保留第一条真实错误，并提示失败数量，避免大量窗口错误淹没设置页
 */
function formatBatchOpenBoxError(failedResults: PromiseRejectedResult[]): string {
  const [firstFailure] = failedResults;
  const firstMessage =
    firstFailure.reason instanceof Error ? firstFailure.reason.message : String(firstFailure.reason);

  return `${failedResults.length} 个 Box 启动失败：${firstMessage}`;
}

/**
 * 新增 Box 后立即打开独立窗口，确保用户看到的是桌面扩展本体
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
 * 设置页里的锁定入口与 Box 更多菜单共用同一 Store 字段，确保窗口拖动和缩放行为一致
 */
async function toggleBoxLockedFromSettings(box: DesktopBox): Promise<void> {
  await desktopStore.updateBoxLocked(box.id, !box.locked);
}

/**
 * 设置页收到的删除事件已经由按钮完成二段式确认，这里只执行真实删除和窗口关闭
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
 * 开机自启以系统启动项为准，设置页每次需要展示时都重新读取，避免外部修改后状态滞后
 */
async function syncAutostartEnabled(): Promise<void> {
  try {
    autostartEnabled.value = await isAutostartEnabled();
  } catch (error) {
    desktopStore.lastError = error instanceof Error ? error.message : String(error);
  }
}

/**
 * 设置页切换自启后交给 Rust 同步托盘勾选状态，前端只保存后端确认后的真实结果
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
 * 设置页选择收纳根目录只影响后续新建 Box，不移动已有真实文件夹
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

/**
 * 打开 Box 对应的真实文件夹，便于用户确认原生 Explorer 视图背后的磁盘位置
 */
async function openBoxFolderFromSettings(box: DesktopBox): Promise<void> {
  try {
    await openBoxFolder(box.folderPath);
  } catch (error) {
    desktopStore.lastError = error instanceof Error ? error.message : String(error);
  }
}

/**
 * 自定义标题栏通过 Tauri 转交拖动，保持无边框窗口仍可移动
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
 * 焦点同步延迟执行，让用户点击标题栏拖动时先进入系统拖动流程，再刷新 SQLite 和桌面快照
 */
function scheduleFocusSync(): void {
  clearFocusSyncTimer();
  focusSyncTimer = window.setTimeout(() => {
    focusSyncTimer = null;
    void syncSettingsWindowState();
  }, FOCUS_SYNC_DELAY_MS);
}

/**
 * 聚焦后同步持久化状态和桌面快照；拖动中收到兜底触发时继续后延，避免刷新抢占拖动
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
 * 清理焦点同步计时器，避免隐藏或拖动窗口时仍然启动一次昂贵的桌面扫描
 */
function clearFocusSyncTimer(): void {
  if (!focusSyncTimer) {
    return;
  }

  window.clearTimeout(focusSyncTimer);
  focusSyncTimer = null;
}

/**
 * 监听拖动释放后再恢复设置同步；原生拖动吞掉释放事件时用短兜底恢复
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
 * 拖动释放监听每次只保留一组，防止多次按住标题栏后重复安排同步
 */
function clearDragReleaseListeners(): void {
  dragReleaseCleanup?.();
  dragReleaseCleanup = null;
}

/**
 * 最小化只作用于设置窗口，桌面上的 Box 会继续保持显示
 */
function minimizeSettings(): void {
  void currentWindow.minimize();
}

/**
 * 最大化只作用于设置窗口，不改变任何 Box 的桌面位置和尺寸
 */
function toggleSettingsMaximize(): void {
  void currentWindow.toggleMaximize();
}

/**
 * 关闭设置页时隐藏主窗口，Box 右键菜单仍可重新唤起设置
 */
function closeSettings(): void {
  void currentWindow.hide();
}
</script>

<template>
  <main class="h-screen w-screen overflow-hidden bg-transparent text-[#17181c] dark:text-[#f4f4f5]">
    <section class="flex h-full w-full overflow-hidden bg-[#f7f7f9] shadow-[0_16px_36px_rgba(20,24,32,0.16)] ring-1 ring-inset ring-[#d7dae2] dark:bg-[#15161b] dark:shadow-[0_16px_36px_rgba(0,0,0,0.34)] dark:ring-[#2b2e36]">
      <SettingsSidebar
        :active-section="activeSection"
        :sections="sections"
        @section-change="activeSection = $event"
      />

      <section class="flex min-w-0 flex-1 flex-col bg-[#fbfbfd] dark:bg-[#101116]">
        <SettingsHeader
          :title="activeTitle"
          @close="closeSettings"
          @drag-start="startDragging"
          @minimize="minimizeSettings"
          @toggle-maximize="toggleSettingsMaximize"
        />

        <div class="dasktop-scrollarea min-h-0 flex-1 overflow-auto">
          <div
            v-if="startupError"
            class="mx-8 mt-6 rounded-[12px] border border-[#f0c7c7] bg-[#fff6f6] px-4 py-3 text-[13px] text-[#9f1d1d] dark:border-[#5b2a2d] dark:bg-[#281619] dark:text-[#ffb4b4]"
          >
            Box 启动失败：{{ startupError }}
          </div>

          <AppearancePanel
            v-if="activeSection === 'appearance'"
            :panel-width="SETTINGS_PANEL_WIDTH.default"
            :settings="desktopStore.settings"
            @box-visual-setting-change="desktopStore.updateNumberSetting"
            @box-theme-change="desktopStore.updateBoxTheme"
            @settings-theme-change="desktopStore.updateSettingsTheme"
          />
          <BoxDisplayPanel
            v-else-if="activeSection === 'boxDisplay'"
            :autostart-enabled="autostartEnabled"
            :migratable-box-count="migratableBoxCount"
            :migration-busy="isMigratingCollectionRoot"
            :panel-width="SETTINGS_PANEL_WIDTH.default"
            :settings="desktopStore.settings"
            @autostart-enabled-change="updateAutostartEnabled"
            @box-boolean-setting-change="desktopStore.updateBooleanSetting"
            @box-conflict-policy-change="desktopStore.updateBoxConflictPolicy"
            @box-delete-policy-change="desktopStore.updateBoxDeletePolicy"
            @box-drag-out-action-change="desktopStore.updateBoxDragOutAction"
            @box-drop-action-change="desktopStore.updateBoxDropAction"
            @collection-root-choose="chooseCollectionRootFromSettings"
            @collection-root-migrate="migrateCollectionRootFromSettings"
            @collection-root-open="openCollectionRootFromSettings"
            @name-display-mode-change="desktopStore.updateNameDisplayMode"
            @snap-threshold-change="desktopStore.updateSnapThreshold"
            @snap-to-edges-change="desktopStore.updateSnapToEdges"
          />
          <BoxesPanel
            v-else-if="activeSection === 'boxes'"
            :boxes="desktopStore.boxes"
            :collection-root-path="desktopStore.settings.collectionRootPath"
            :panel-width="SETTINGS_PANEL_WIDTH.wide"
            @create-box="createAndOpenBox"
            @delete-box="deleteBoxFromSettings"
            @open-box="openBoxWindow"
            @open-folder="openBoxFolderFromSettings"
            @refresh="desktopStore.refreshSnapshot"
            @toggle-box-locked="toggleBoxLockedFromSettings"
          />
          <AboutPanel
            v-else-if="activeSection === 'about'"
            :desktop-path="desktopStore.desktopPath"
            :panel-width="SETTINGS_PANEL_WIDTH.default"
          />
        </div>
      </section>
    </section>
  </main>
</template>
