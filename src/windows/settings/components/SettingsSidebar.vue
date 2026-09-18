<script setup lang="ts">
import { computed } from "vue";
import logoUrl from "@/assets/logo.png";
import { APP_META } from "@/shared/config/appMeta";
import type { SettingsNavItem, SettingsSection } from "../model/navigation";

/**
 * 设置侧栏只渲染当前版本的设置入口，导航文案由 settings model 统一维护。
 */
const props = defineProps<{
  activeSection: SettingsSection;
  sections: readonly SettingsNavItem[];
}>();

const emit = defineEmits<{
  sectionChange: [section: SettingsSection];
}>();

/**
 * 侧栏品牌区直接使用应用正式 logo，避免设置页与安装包图标分开维护视觉资产
 */
const settingsLogoUrl = logoUrl;

/**
 * 版本和作者来自统一应用元信息，发布时只需要维护包版本来源。
 */
const appInfo = {
  version: APP_META.version,
  author: APP_META.author,
} as const;

const NAV_ITEM_MOTION_OFFSET_PX = 44;

/**
 * 当前菜单索引用于驱动选中背景层位移，找不到时回落到首项避免动画落到空位置。
 */
const activeSectionIndex = computed(() => {
  const matchedIndex = props.sections.findIndex((section) => section.key === props.activeSection);
  return Math.max(matchedIndex, 0);
});

/**
 * 选中背景位移只由 Vue 状态驱动，避免 imperative motion 与 style transform 不同步。
 */
const activeIndicatorStyle = computed(() => ({
  transform: `translateY(${activeSectionIndex.value * NAV_ITEM_MOTION_OFFSET_PX}px)`,
}));
</script>

<template>
  <aside class="flex h-full w-[264px] shrink-0 flex-col border-r border-[#dfe2e8] bg-[#f2f3f6] dark:border-[#292c34] dark:bg-[#17181e]">
    <div class="flex items-center gap-3 px-5 pb-5 pt-6">
      <img
        :src="settingsLogoUrl"
        alt="Dasktop"
        class="size-10 shrink-0 rounded-[10px] object-contain"
      />
      <div class="min-w-0">
        <div class="truncate text-[17px] font-semibold tracking-[0] text-[#17181c] dark:text-[#f4f4f5]">{{ APP_META.displayName }}</div>
        <div class="mt-0.5 text-[12px] text-[#6f7480] dark:text-[#9ca0aa]">像 Box 一样管理桌面</div>
      </div>
    </div>

    <nav class="flex-1 px-3 pb-4">
      <div class="relative grid gap-1">
        <div
          aria-hidden="true"
          class="settings-sidebar-active-indicator pointer-events-none absolute left-0 top-0 h-10 w-full rounded-[9px] bg-[#ffffff] shadow-[0_5px_16px_rgba(20,24,32,0.08)] will-change-transform dark:bg-[#24262e] dark:shadow-none"
          :style="activeIndicatorStyle"
        />
        <button
          v-for="section in sections"
          :key="section.key"
          class="relative z-10 flex h-10 items-center gap-3 rounded-[9px] px-3 text-left text-[13px] font-medium transition-colors duration-200"
          :data-theme-instant="activeSection === section.key ? 'true' : undefined"
          :class="
            activeSection === section.key
              ? 'text-[#17181c] dark:text-[#f4f4f5]'
              : 'text-[#606672] hover:bg-[#ffffff] hover:text-[#17181c] dark:text-[#9ca0aa] dark:hover:bg-[#202229] dark:hover:text-[#f4f4f5]'
          "
          type="button"
          @click="emit('sectionChange', section.key)"
        >
          <span
            class="grid size-7 shrink-0 place-items-center rounded-[8px] transition-[background-color,color,transform] duration-200"
            :class="
              activeSection === section.key
                ? 'scale-[1.04] bg-[#ff5c5c] text-white'
                : 'bg-[#e3e5eb] text-[#68707d] dark:bg-[#2a2d35] dark:text-[#a7abb5]'
            "
          >
            <component :is="section.icon" :size="15" />
          </span>
          <span>{{ section.label }}</span>
        </button>
      </div>
    </nav>

    <div class="border-t border-[#dfe2e8] px-5 py-4 text-[12px] text-[#7a7f8b] dark:border-[#292c34] dark:text-[#858a95]">
      <div>版本 v{{ appInfo.version }}</div>
      <div class="mt-1">作者 {{ appInfo.author }}</div>
    </div>
  </aside>
</template>

<style scoped>
/**
 * 侧栏选中背景只做轻量位移动画，减少快速切换菜单时的状态不同步风险。
 */
.settings-sidebar-active-indicator {
  transition: transform 200ms cubic-bezier(0.22, 1, 0.36, 1);
}

@media (prefers-reduced-motion: reduce) {
  .settings-sidebar-active-indicator {
    transition: none;
  }
}
</style>
