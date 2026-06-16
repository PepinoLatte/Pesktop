<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { Boxes, Info, Palette, SlidersHorizontal } from "@lucide/vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import AboutPanel from "./components/AboutPanel.vue";
import AppearancePanel from "./components/AppearancePanel.vue";
import BehaviorPanel from "./components/BehaviorPanel.vue";
import BoxesPanel from "./components/BoxesPanel.vue";
import SettingsHeader from "./components/SettingsHeader.vue";
import SettingsSidebar from "./components/SettingsSidebar.vue";
import { useDesktopStore } from "../desktop/store/desktopStore";
import { openBoxWindow } from "../../shared/window/boxWindows";
import type { SettingsNavItem, SettingsSection } from "./types";

const currentWindow = getCurrentWindow();
const desktopStore = useDesktopStore();
const activeSection = ref<SettingsSection>("boxes");
const startupError = ref("");
const unlistenFns: UnlistenFn[] = [];

/**
 * 设置页只保留当前真实可用的配置入口，旧占位导航不再兼容。
 */
const sections: SettingsNavItem[] = [
  { key: "boxes", label: "Box", icon: Boxes },
  { key: "appearance", label: "外观", icon: Palette },
  { key: "behavior", label: "行为", icon: SlidersHorizontal },
  { key: "about", label: "关于", icon: Info },
];

const activeTitle = computed(
  () => sections.find((section) => section.key === activeSection.value)?.label ?? "设置",
);
const totalItems = computed(() =>
  desktopStore.boxes.reduce((count, box) => count + box.itemPaths.length, 0),
);

onMounted(async () => {
  await desktopStore.initialize();
  unlistenFns.push(
    await currentWindow.onFocusChanged(async ({ payload }) => {
      if (!payload) {
        return;
      }

      await desktopStore.reloadPersistedState();
      await desktopStore.refreshSnapshot(false);
    }),
  );

  if (await openAllBoxes()) {
    await currentWindow.hide();
  }
});

/**
 * 组件销毁时注销窗口事件，避免开发热更新后重复刷新状态。
 */
onUnmounted(() => {
  for (const unlisten of unlistenFns) {
    unlisten();
  }
});

/**
 * 设置页启动时打开现有 Box，随后隐藏设置窗，让桌面扩展本体先出现。
 */
async function openAllBoxes(): Promise<boolean> {
  startupError.value = "";

  try {
    for (const box of desktopStore.boxes) {
      await openBoxWindow(box, { focus: false });
    }
    return true;
  } catch (error) {
    startupError.value = error instanceof Error ? error.message : String(error);
    return false;
  }
}

/**
 * 新增 Box 后立即打开独立窗口，确保用户看到的是桌面扩展本体。
 */
async function createAndOpenBox(): Promise<void> {
  const box = await desktopStore.createBox();
  await openBoxWindow(box, { focus: true });
}

/**
 * 自定义标题栏通过 Tauri 转交拖动，保持无边框窗口仍可移动。
 */
function startDragging(event: MouseEvent): void {
  if (event.button !== 0 || event.detail > 1) {
    return;
  }

  void currentWindow.startDragging();
}

/**
 * 最小化只作用于设置窗口，桌面上的 Box 会继续保持显示。
 */
function minimizeSettings(): void {
  void currentWindow.minimize();
}

/**
 * 最大化只作用于设置窗口，不改变任何 Box 的桌面位置和尺寸。
 */
function toggleSettingsMaximize(): void {
  void currentWindow.toggleMaximize();
}

/**
 * 关闭设置页时隐藏主窗口，Box 右键菜单仍可重新唤起设置。
 */
function closeSettings(): void {
  void currentWindow.hide();
}
</script>

<template>
  <main class="h-screen w-screen overflow-hidden bg-transparent text-[#17181c] dark:text-[#f4f4f5]">
    <section class="flex h-full w-full overflow-hidden border border-[#d7dae2] bg-[#f7f7f9] shadow-[0_18px_42px_rgba(20,24,32,0.18)] dark:border-[#2b2e36] dark:bg-[#15161b] dark:shadow-[0_18px_42px_rgba(0,0,0,0.38)]">
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
            :settings="desktopStore.settings"
            @item-labels-change="desktopStore.updateShowItemLabels"
            @theme-change="desktopStore.updateTheme"
          />
          <BoxesPanel
            v-else-if="activeSection === 'boxes'"
            :boxes="desktopStore.boxes"
            :total-items="totalItems"
            :unassigned-items="desktopStore.unassignedItems.length"
            @create-box="createAndOpenBox"
            @open-box="openBoxWindow"
            @refresh="desktopStore.refreshSnapshot"
          />
          <BehaviorPanel
            v-else-if="activeSection === 'behavior'"
            :settings="desktopStore.settings"
            @snap-threshold-change="desktopStore.updateSnapThreshold"
            @snap-to-edges-change="desktopStore.updateSnapToEdges"
          />
          <AboutPanel
            v-else-if="activeSection === 'about'"
            :desktop-path="desktopStore.desktopPath"
          />
        </div>
      </section>
    </section>
  </main>
</template>
