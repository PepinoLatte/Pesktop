<script setup lang="ts">
import { ref, watch } from "vue";
import type { ComponentPublicInstance } from "vue";
import { ImagePlus } from "@lucide/vue";
import { useDesktopBoxDeleteConfirmation } from "@/entities/desktopBox/deleteConfirmation";
import MenuActionRows from "@/windows/boxContextMenu/components/MenuActionRows.vue";
import MenuCoverPicker from "@/windows/boxContextMenu/components/MenuCoverPicker.vue";
import MenuDeleteButton from "@/windows/boxContextMenu/components/MenuDeleteButton.vue";
import MenuSettingControls from "@/windows/boxContextMenu/components/MenuSettingControls.vue";
import MenuTopActions from "@/windows/boxContextMenu/components/MenuTopActions.vue";
import { useBoxContextMenuActions } from "@/windows/boxContextMenu/composables/useBoxContextMenuActions";
import { useBoxContextMenuWindow } from "@/windows/boxContextMenu/composables/useBoxContextMenuWindow";

const {
  clearBoxDeleteConfirmation,
  isConfirmingBoxDelete,
  requestBoxDeleteConfirmation,
} = useDesktopBoxDeleteConfirmation();

/**
 * 菜单内视图切换：封面选择器内容较高，嵌入独立视图避免撑破主菜单布局；
 * 激活 Box 变化（重新打开菜单）时回到主视图
 */
const menuView = ref<"main" | "cover">("main");

/**
 * Box 更多菜单壳组件负责绑定当前激活 Box，并把窗口生命周期与业务动作分派到 composables。
 */
const { box, closeAnimated, isMenuRendered, menuRef } = useBoxContextMenuWindow({
  clearBoxDeleteConfirmation,
});
const {
  boxAutoCollapseMode,
  createBoxFromMenu,
  deleteCurrentBox,
  openFolderFromMenu,
  openSettingsFromMenu,
  refreshDesktopFromMenu,
  toggleBoxLockedFromMenu,
  updateBoxAutoCollapseFromMenu,
  updateBoxCollapseModeFromMenu,
  updateBoxCoverFromMenu,
  updateIdleOpacityFromMenu,
  updateTitlePositionFromMenu,
} = useBoxContextMenuActions({
  box,
  clearBoxDeleteConfirmation,
  closeAnimated,
  requestBoxDeleteConfirmation,
});

watch(
  () => box.value?.id,
  () => {
    menuView.value = "main";
  },
);

/**
 * 函数 ref 显式把菜单 DOM 交给窗口动画 composable，避免字符串 ref 在类型检查中被误判未使用。
 */
function bindMenuRef(element: Element | ComponentPublicInstance | null): void {
  menuRef.value = element instanceof HTMLElement ? element : null;
}
</script>

<template>
  <main class="box-context-menu-window h-screen w-screen overflow-hidden bg-transparent p-0">
    <nav
      v-if="box && isMenuRendered"
      :ref="bindMenuRef"
      aria-label="Box 更多菜单"
      class="dasktop-box-menu grid h-full w-full gap-1 overflow-x-hidden overflow-y-auto rounded-[10px] border border-[#d9dce3] bg-[#fbfbfd] p-1.5 text-slate-800 shadow-[0_18px_45px_rgba(15,23,42,0.24)] dark:border-[#30333c] dark:bg-[#202228] dark:text-slate-100"
      @click.stop
    >
      <MenuCoverPicker
        v-if="menuView === 'cover'"
        :box="box"
        @back="menuView = 'main'"
        @select="
          (coverIcon) => {
            menuView = 'main';
            void updateBoxCoverFromMenu(coverIcon);
          }
        "
      />
      <template v-else>
        <MenuTopActions
          @create-box="createBoxFromMenu"
          @open-settings="openSettingsFromMenu"
          @refresh-desktop="refreshDesktopFromMenu"
        />
        <span class="my-0.5 h-px bg-[#e4e6eb] dark:bg-[#30333c]" />
        <MenuSettingControls
          :box="box"
          :box-auto-collapse-mode="boxAutoCollapseMode"
          @update-auto-collapse="updateBoxAutoCollapseFromMenu"
          @update-collapse-mode="updateBoxCollapseModeFromMenu"
          @update-title-position="updateTitlePositionFromMenu"
        />
        <button
          class="flex min-h-[36px] w-full items-center gap-2 rounded-[8px] px-2 text-left text-[12px] transition-colors hover:bg-[#eef1f6] focus-visible:bg-[#eef1f6] dark:hover:bg-[#2a2d36] dark:focus-visible:bg-[#2a2d36]"
          type="button"
          @click="menuView = 'cover'"
        >
          <span class="grid size-7 shrink-0 place-items-center rounded-[7px] bg-[#e9edf5] text-slate-600 dark:bg-[#30343e] dark:text-slate-300">
            <ImagePlus :size="15" />
          </span>
          <span class="grid min-w-0 flex-1 gap-0.5">
            <span class="truncate text-[12px] font-semibold text-slate-800 dark:text-slate-100">设置封面</span>
            <span class="truncate text-[10px] leading-3 text-slate-500 dark:text-slate-400">
              {{ box.coverIcon ? "已选自定义封面（点击更换或恢复）" : "当前使用默认图标" }}
            </span>
          </span>
          <span
            class="rounded-[6px] px-1.5 py-0.5 text-[10px] font-medium"
            :class="box.coverIcon ? 'bg-[#e0edff] text-[#1c64f2] dark:bg-[#1e2e4a] dark:text-[#60a5fa]' : 'bg-[#e9edf5] text-slate-500 dark:bg-[#2b2e37] dark:text-slate-400'"
          >
            {{ box.coverIcon ? "自定义" : "默认图标" }}
          </span>
        </button>
        <span class="my-0.5 h-px bg-[#e4e6eb] dark:bg-[#30333c]" />
        <MenuActionRows
          :box="box"
          @open-folder="openFolderFromMenu"
          @toggle-locked="toggleBoxLockedFromMenu"
          @update-idle-opacity="updateIdleOpacityFromMenu"
        />
        <span class="my-0.5 h-px bg-[#e4e6eb] dark:bg-[#30333c]" />
        <MenuDeleteButton
          :box="box"
          :is-confirming="isConfirmingBoxDelete(box)"
          @delete-box="deleteCurrentBox"
        />
      </template>
    </nav>
  </main>
</template>

<style scoped>
/**
 * Box 更多菜单是工具型弹窗，禁用文本框选可以避免快速点击菜单项时误选中文案。
 * 这里与设置页保持一致，同时覆盖 WebView2 旧版内核的前缀属性。
 */
.box-context-menu-window,
.box-context-menu-window * {
  user-select: none;
  -webkit-user-select: none;
}
</style>
