<script setup lang="ts">
import { Monitor, Moon, Sun } from "@lucide/vue";
import type { Component } from "vue";
import type { AppSettings, ThemeMode } from "@/entities/appSettings/types";
import SegmentedControl from "@/shared/ui/SegmentedControl.vue";

/**
 * 主题分段选项只服务当前外观面板，内联后避免为单一展示数据单独跳转文件。
 */
const THEME_SEGMENT_OPTIONS: Array<{
  icon: Component;
  label: string;
  value: ThemeMode;
}> = [
  { icon: Sun, label: "浅色", value: "light" },
  { icon: Monitor, label: "跟随系统", value: "system" },
  { icon: Moon, label: "深色", value: "dark" },
];

/**
 * 外观面板承载主题和 Box 视觉密度设置，文件名显示规则交给设置页集中管理。
 */
const props = defineProps<{
  /**
   * 设置页父级统一下发内容宽度，避免各面板各自维护页面密度。
   */
  panelWidth: string;
  settings: AppSettings;
}>();

const emit = defineEmits<{
  boxThemeChange: [theme: ThemeMode];
  settingsThemeChange: [theme: ThemeMode];
}>();
</script>

<template>
  <section class="mx-auto w-full px-8 py-7" :class="panelWidth">
    <div class="mb-6">
      <h1 class="text-[24px] font-semibold tracking-[0] text-[#17181c] dark:text-[#f4f4f5]">外观</h1>
      <p class="mt-1 text-[13px] text-[#6f7480] dark:text-[#a7abb5]">调整设置窗和 Box 的显示偏好。</p>
    </div>

    <div class="overflow-hidden rounded-[8px] border border-[#dfe2e8] bg-[#ffffff] shadow-[0_10px_28px_rgba(20,24,32,0.06)] dark:border-[#292c34] dark:bg-[#181a20] dark:shadow-none">
      <div class="grid min-h-[76px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">设置页主题</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">只影响当前设置窗口的明暗显示。</p>
        </div>

        <SegmentedControl
          :model-value="props.settings.settingsTheme"
          :options="THEME_SEGMENT_OPTIONS"
          @change="emit('settingsThemeChange', $event)"
        />
      </div>

      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />

      <div class="grid min-h-[76px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">Box 窗口主题</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">只影响桌面上的 Box 窗口。</p>
        </div>

        <SegmentedControl
          :model-value="props.settings.boxTheme"
          :options="THEME_SEGMENT_OPTIONS"
          @change="emit('boxThemeChange', $event)"
        />
      </div>
    </div>

  </section>
</template>
