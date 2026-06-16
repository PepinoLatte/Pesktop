<script setup lang="ts">
import type { AppSettings } from "../../../shared/types/desktop";

/**
 * 行为面板只维护窗口交互偏好，不接管 Windows 桌面行为。
 */
defineProps<{
  settings: AppSettings;
}>();

const emit = defineEmits<{
  snapThresholdChange: [value: number];
  snapToEdgesChange: [value: boolean];
}>();
</script>

<template>
  <section class="mx-auto w-full max-w-[780px] px-8 py-7">
    <div class="mb-6">
      <h1 class="text-[24px] font-semibold tracking-[0] text-[#17181c] dark:text-[#f4f4f5]">行为</h1>
      <p class="mt-1 text-[13px] text-[#6f7480] dark:text-[#a7abb5]">控制 Box 在桌面上的移动方式。</p>
    </div>

    <div class="overflow-hidden rounded-[14px] border border-[#dfe2e8] bg-[#ffffff] shadow-[0_10px_28px_rgba(20,24,32,0.06)] dark:border-[#292c34] dark:bg-[#181a20] dark:shadow-none">
      <div class="grid min-h-[72px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div>
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">边缘吸附</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">拖动 Box 靠近屏幕边缘时自动贴齐。</p>
        </div>

        <label class="relative block h-7 w-12">
          <input
            class="peer sr-only"
            :checked="settings.snapToEdges"
            type="checkbox"
            @change="emit('snapToEdgesChange', ($event.target as HTMLInputElement).checked)"
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

      <div class="grid min-h-[84px] grid-cols-[1fr_260px] items-center gap-6 px-5">
        <div>
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">吸附距离</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">数值越大，越容易贴到屏幕边缘。</p>
        </div>

        <div class="flex items-center gap-3">
          <input
            class="h-1.5 w-full cursor-pointer appearance-none rounded-full bg-[#d8dbe3] accent-[#ff5c5c] dark:bg-[#333640] dark:accent-[#ff6b6b]"
            max="64"
            min="8"
            step="1"
            type="range"
            :value="settings.snapThreshold"
            @input="emit('snapThresholdChange', Number(($event.target as HTMLInputElement).value))"
          />
          <span class="w-12 text-right text-[13px] font-medium text-[#555b66] dark:text-[#c7cad1]">
            {{ settings.snapThreshold }} px
          </span>
        </div>
      </div>
    </div>
  </section>
</template>
