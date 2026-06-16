<script setup lang="ts">
import type { AppSettings, ThemeMode } from "../../../shared/types/desktop";
import { SETTINGS_PANEL_WIDTH, THEME_SEGMENT_OPTIONS } from "../config/settingsUi";
import SegmentedControl from "./SegmentedControl.vue";

/**
 * 外观面板只暴露当前实现的主题和图标名称设置，避免旧视觉参数回流。
 */
const props = defineProps<{
  settings: AppSettings;
}>();

const emit = defineEmits<{
  itemLabelsChange: [value: boolean];
  themeChange: [theme: ThemeMode];
}>();

</script>

<template>
  <section class="mx-auto w-full px-8 py-7" :class="SETTINGS_PANEL_WIDTH.default">
    <div class="mb-6">
      <h1 class="text-[24px] font-semibold tracking-[0] text-[#17181c] dark:text-[#f4f4f5]">外观</h1>
      <p class="mt-1 text-[13px] text-[#6f7480] dark:text-[#a7abb5]">调整设置窗和 Box 的显示偏好。</p>
    </div>

    <div class="overflow-hidden rounded-[14px] border border-[#dfe2e8] bg-[#ffffff] shadow-[0_10px_28px_rgba(20,24,32,0.06)] dark:border-[#292c34] dark:bg-[#181a20] dark:shadow-none">
      <div class="grid min-h-[76px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">界面主题</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">用于设置页和 Box 窗口的明暗切换。</p>
        </div>

        <SegmentedControl
          :model-value="props.settings.theme"
          :options="THEME_SEGMENT_OPTIONS"
          @change="emit('themeChange', $event)"
        />
      </div>

      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />

      <div class="grid min-h-[72px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">显示项目名称</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">在 Box 图标下方展示文件名。</p>
        </div>

        <label class="relative block h-7 w-12">
          <input
            class="peer sr-only"
            :checked="props.settings.showItemLabels"
            type="checkbox"
            @change="emit('itemLabelsChange', ($event.target as HTMLInputElement).checked)"
          />
          <span
            class="absolute inset-0 rounded-full bg-[#c9cdd6] transition-colors peer-checked:bg-[#ff5c5c] dark:bg-[#3a3d46] dark:peer-checked:bg-[#ff6b6b]"
          />
          <span
            class="absolute left-1 top-1 size-5 rounded-full bg-white shadow-[0_2px_7px_rgba(20,24,32,0.25)] transition-transform peer-checked:translate-x-5"
          />
        </label>
      </div>
    </div>
  </section>
</template>
