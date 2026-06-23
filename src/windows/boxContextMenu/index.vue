<script setup lang="ts">
import type { ComponentPublicInstance } from "vue";
import { useDesktopBoxDeleteConfirmation } from "@/entities/desktopBox/deleteConfirmation";
import MenuActionRows from "@/windows/boxContextMenu/components/MenuActionRows.vue";
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
  updateIdleOpacityFromMenu,
  updateTitlePositionFromMenu,
} = useBoxContextMenuActions({
  box,
  clearBoxDeleteConfirmation,
  closeAnimated,
  requestBoxDeleteConfirmation,
});

/**
 * 函数 ref 显式把菜单 DOM 交给窗口动画 composable，避免字符串 ref 在类型检查中被误判未使用。
 */
function bindMenuRef(element: Element | ComponentPublicInstance | null): void {
  menuRef.value = element instanceof HTMLElement ? element : null;
}
</script>

<template>
  <main class="h-screen w-screen overflow-hidden bg-transparent p-0">
    <nav
      v-if="box && isMenuRendered"
      :ref="bindMenuRef"
      aria-label="Box 更多菜单"
      class="dasktop-box-menu grid h-full w-full gap-1 overflow-x-hidden overflow-y-auto rounded-[10px] border border-[#d9dce3] bg-[#fbfbfd] p-1.5 text-slate-800 shadow-[0_18px_45px_rgba(15,23,42,0.24)] dark:border-[#30333c] dark:bg-[#202228] dark:text-slate-100"
      @click.stop
    >
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
        @update-title-position="updateTitlePositionFromMenu"
      />
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
    </nav>
  </main>
</template>
