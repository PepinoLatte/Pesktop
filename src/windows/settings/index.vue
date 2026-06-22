<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useDesktopStore } from "@/entities/desktopBox/store";
import AboutPanel from "./panels/AboutPanel.vue";
import AppearancePanel from "./panels/AppearancePanel.vue";
import BoxesPanel from "./panels/BoxesPanel.vue";
import FilePanel from "./panels/FilePanel.vue";
import SettingsHeader from "./components/SettingsHeader.vue";
import SettingsSidebar from "./components/SettingsSidebar.vue";
import WindowPanel from "./panels/WindowPanel.vue";
import { useSettingsActions } from "./composables/useSettingsActions";
import { useSettingsBoxes } from "./composables/useSettingsBoxes";
import { useSettingsStartup } from "./composables/useSettingsStartup";
import { useSettingsWindow } from "./composables/useSettingsWindow";
import {
  SETTINGS_PANEL_WIDTH,
  SETTINGS_SECTIONS,
  resolveSettingsSectionTitle,
  type SettingsSection,
} from "./model/navigation";

const desktopStore = useDesktopStore();
const activeSection = ref<SettingsSection>("boxes");
const activeTitle = computed(() => resolveSettingsSectionTitle(activeSection.value));
const settingsActions = useSettingsActions(desktopStore);
const settingsBoxes = useSettingsBoxes(desktopStore);
const settingsWindow = useSettingsWindow(desktopStore, settingsActions.syncAutostartEnabled);
const settingsStartup = useSettingsStartup(
  desktopStore,
  settingsActions.syncAutostartEnabled,
  settingsBoxes.createAndOpenBox,
  (enabled) => {
    settingsActions.autostartEnabled.value = enabled;
  },
);

/**
 * 设置页启动时注册窗口同步和托盘事件，再恢复桌面上的 Box 窗口。
 */
onMounted(async () => {
  await settingsWindow.bindFocusSync();
  await settingsStartup.initializeSettingsStartup();
});

/**
 * 组件销毁时注销窗口和托盘事件，避免开发热更新后重复刷新状态。
 */
onUnmounted(() => {
  settingsWindow.disposeSettingsWindow();
  settingsStartup.disposeSettingsStartup();
});
</script>

<template>
  <main class="settings-window h-screen w-screen overflow-hidden bg-transparent text-[#17181c] dark:text-[#f4f4f5]">
    <section class="flex h-full w-full overflow-hidden bg-[#f7f7f9] shadow-[0_16px_36px_rgba(20,24,32,0.16)] ring-1 ring-inset ring-[#d7dae2] dark:bg-[#15161b] dark:shadow-[0_16px_36px_rgba(0,0,0,0.34)] dark:ring-[#2b2e36]">
      <SettingsSidebar
        :active-section="activeSection"
        :sections="SETTINGS_SECTIONS"
        @section-change="activeSection = $event"
      />

      <section class="flex min-w-0 flex-1 flex-col bg-[#fbfbfd] dark:bg-[#101116]">
        <SettingsHeader
          :title="activeTitle"
          @close="settingsWindow.closeSettings"
          @drag-start="settingsWindow.startDragging"
          @minimize="settingsWindow.minimizeSettings"
          @toggle-maximize="settingsWindow.toggleSettingsMaximize"
        />

        <div class="dasktop-scrollarea min-h-0 flex-1 overflow-auto">
          <div
            v-if="settingsStartup.startupError.value"
            class="mx-8 mt-6 rounded-[12px] border border-[#f0c7c7] bg-[#fff6f6] px-4 py-3 text-[13px] text-[#9f1d1d] dark:border-[#5b2a2d] dark:bg-[#281619] dark:text-[#ffb4b4]"
          >
            Box 启动失败：{{ settingsStartup.startupError.value }}
          </div>

          <BoxesPanel
            v-if="activeSection === 'boxes'"
            :boxes="desktopStore.boxes"
            :collection-root-path="desktopStore.settings.collectionRootPath"
            :migratable-box-count="settingsActions.migratableBoxCount.value"
            :migration-busy="settingsActions.isMigratingCollectionRoot.value"
            :panel-width="SETTINGS_PANEL_WIDTH.default"
            @collection-root-choose="settingsActions.chooseCollectionRootFromSettings"
            @collection-root-migrate="settingsActions.migrateCollectionRootFromSettings"
            @collection-root-open="settingsActions.openCollectionRootFromSettings"
            @create-box="settingsBoxes.createAndOpenBox"
            @delete-box="settingsBoxes.deleteBoxFromSettings"
            @open-box="settingsBoxes.openBoxWindow"
            @open-folder="settingsBoxes.openBoxFolderFromSettings"
            @refresh="desktopStore.refreshSnapshot"
            @toggle-box-locked="settingsBoxes.toggleBoxLockedFromSettings"
          />
          <FilePanel
            v-else-if="activeSection === 'file'"
            :panel-width="SETTINGS_PANEL_WIDTH.default"
            :settings="desktopStore.settings"
            @box-boolean-setting-change="desktopStore.updateBooleanSetting"
            @box-conflict-policy-change="desktopStore.updateBoxConflictPolicy"
            @box-delete-policy-change="desktopStore.updateBoxDeletePolicy"
            @box-drag-out-action-change="desktopStore.updateBoxDragOutAction"
            @box-drop-action-change="desktopStore.updateBoxDropAction"
            @name-display-mode-change="desktopStore.updateNameDisplayMode"
          />
          <WindowPanel
            v-else-if="activeSection === 'window'"
            :autostart-enabled="settingsActions.autostartEnabled.value"
            :panel-width="SETTINGS_PANEL_WIDTH.default"
            :settings="desktopStore.settings"
            @autostart-enabled-change="settingsActions.updateAutostartEnabled"
            @box-boolean-setting-change="desktopStore.updateBooleanSetting"
            @snap-threshold-change="desktopStore.updateSnapThreshold"
            @snap-to-edges-change="desktopStore.updateSnapToEdges"
          />
          <AppearancePanel
            v-else-if="activeSection === 'appearance'"
            :panel-width="SETTINGS_PANEL_WIDTH.default"
            :settings="desktopStore.settings"
            @box-theme-change="desktopStore.updateBoxTheme"
            @box-visual-setting-change="desktopStore.updateNumberSetting"
            @settings-theme-change="desktopStore.updateSettingsTheme"
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

<style scoped>
/**
 * 设置页是桌面工具面板，禁用文本框选可以避免拖拽窗口和点击控件时误选中文案。
 */
.settings-window,
.settings-window * {
  user-select: none;
  -webkit-user-select: none;
}
</style>
