<script setup lang="ts">
import type { AppSettings, DesktopNameDisplayMode } from "../../../shared/types/desktop";
import { NAME_DISPLAY_SEGMENT_OPTIONS, SETTINGS_PANEL_WIDTH } from "../config/settingsUi";
import SegmentedControl from "./SegmentedControl.vue";

/**
 * Box 显示面板维护所有 Box 共享的图标呈现规则，避免同一项目在不同窗口表现不一致。
 */
const props = defineProps<{
  settings: AppSettings;
}>();

const emit = defineEmits<{
  doubleClickOpenItemsChange: [value: boolean];
  nameDisplayModeChange: [value: DesktopNameDisplayMode];
  showShortcutArrowChange: [value: boolean];
}>();

</script>

<template>
  <section class="mx-auto w-full px-8 py-7" :class="SETTINGS_PANEL_WIDTH.default">
    <div class="mb-6">
      <h1 class="text-[24px] font-semibold tracking-[0] text-[#17181c] dark:text-[#f4f4f5]">设置</h1>
      <p class="mt-1 text-[13px] text-[#6f7480] dark:text-[#a7abb5]">调整 Box 内项目的展示和打开方式。</p>
    </div>

    <div class="overflow-hidden rounded-[14px] border border-[#dfe2e8] bg-[#ffffff] shadow-[0_10px_28px_rgba(20,24,32,0.06)] dark:border-[#292c34] dark:bg-[#181a20] dark:shadow-none">
      <div class="grid min-h-[72px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">快捷方式标记</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">在快捷方式图标左下角显示 Windows 风格箭头。</p>
        </div>

        <label class="relative block h-7 w-12">
          <input
            class="peer sr-only"
            :checked="props.settings.showShortcutArrow"
            type="checkbox"
            @change="emit('showShortcutArrowChange', ($event.target as HTMLInputElement).checked)"
          />
          <span
            class="absolute inset-0 rounded-full bg-[#c9cdd6] transition-colors peer-checked:bg-[#ff5c5c] dark:bg-[#3a3d46] dark:peer-checked:bg-[#ff6b6b]"
          />
          <span
            class="absolute left-1 top-1 size-5 rounded-full bg-white shadow-[0_2px_7px_rgba(20,24,32,0.25)] transition-transform peer-checked:translate-x-5"
          />
        </label>
      </div>

      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />

      <div class="grid min-h-[76px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">文件名样式</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">控制 Box 中项目名称的后缀显示策略。</p>
        </div>

        <SegmentedControl
          :model-value="props.settings.nameDisplayMode"
          :options="NAME_DISPLAY_SEGMENT_OPTIONS"
          @change="emit('nameDisplayModeChange', $event)"
        />
      </div>

      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />

      <div class="grid min-h-[72px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">双击打开项目</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">开启后需要双击图标才会打开文件或文件夹。</p>
        </div>

        <label class="relative block h-7 w-12">
          <input
            class="peer sr-only"
            :checked="props.settings.doubleClickOpenItems"
            type="checkbox"
            @change="emit('doubleClickOpenItemsChange', ($event.target as HTMLInputElement).checked)"
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
