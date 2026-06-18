<script setup lang="ts">
import logoUrl from "@/assets/logo.png";
import packageInfo from "../../../../package.json";
import type { Component } from "vue";

/**
 * 侧栏只需要识别父级传入的当前设置分区，类型保持在组件内避免保留独立 types 文件
 */
type SettingsSection = "boxes" | "boxDisplay" | "appearance" | "about";

/**
 * 导航项结构与父级 sections 常量保持一致，icon 继续使用 Lucide/Vue 组件类型
 */
interface SettingsNavItem {
  key: SettingsSection;
  label: string;
  icon: Component;
}

/**
 * 设置侧栏只渲染当前版本的设置入口，旧占位菜单不再保留
 */
defineProps<{
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
 * 版本和作者来自包元数据，发布时只需要维护 package.json 这一处来源
 */
const appInfo = {
  version: packageInfo.version,
  author: packageInfo.author,
} as const;
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
        <div class="truncate text-[17px] font-semibold tracking-[0] text-[#17181c] dark:text-[#f4f4f5]">Dasktop</div>
        <div class="mt-0.5 text-[12px] text-[#6f7480] dark:text-[#9ca0aa]">像 Box 一样管理桌面</div>
      </div>
    </div>

    <nav class="flex-1 px-3 pb-4">
      <div class="grid gap-1">
        <button
          v-for="section in sections"
          :key="section.key"
          class="flex h-10 items-center gap-3 rounded-[9px] px-3 text-left text-[13px] font-medium transition-colors"
          :data-theme-instant="activeSection === section.key ? 'true' : undefined"
          :class="
            activeSection === section.key
              ? 'bg-[#ffffff] text-[#17181c] shadow-[0_5px_16px_rgba(20,24,32,0.08)] dark:bg-[#24262e] dark:text-[#f4f4f5] dark:shadow-none'
              : 'text-[#606672] hover:bg-[#ffffff] hover:text-[#17181c] dark:text-[#9ca0aa] dark:hover:bg-[#202229] dark:hover:text-[#f4f4f5]'
          "
          type="button"
          @click="emit('sectionChange', section.key)"
        >
          <span
            class="grid size-7 shrink-0 place-items-center rounded-[8px]"
            :class="
              activeSection === section.key
                ? 'bg-[#ff5c5c] text-white'
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
