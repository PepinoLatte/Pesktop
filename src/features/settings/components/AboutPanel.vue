<script setup lang="ts">
import { SETTINGS_PANEL_WIDTH } from "../config/settingsUi";

/**
 * 关于面板只展示运行信息，避免把设置页做成产品宣传页。
 */
defineProps<{
  desktopPath: string;
}>();
</script>

<template>
  <section class="mx-auto w-full px-8 py-7" :class="SETTINGS_PANEL_WIDTH.default">
    <div class="mb-6">
      <h1 class="text-[24px] font-semibold tracking-[0] text-[#17181c] dark:text-[#f4f4f5]">关于</h1>
      <p class="mt-1 text-[13px] text-[#6f7480] dark:text-[#a7abb5]">当前桌面扩展的运行信息。</p>
    </div>

    <div class="overflow-hidden rounded-[14px] border border-[#dfe2e8] bg-[#ffffff] shadow-[0_10px_28px_rgba(20,24,32,0.06)] dark:border-[#292c34] dark:bg-[#181a20] dark:shadow-none">
      <div class="grid min-h-[68px] grid-cols-[150px_1fr] items-center gap-6 px-5">
        <span class="text-[13px] text-[#707684] dark:text-[#9ca0aa]">应用名称</span>
        <strong class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">Dasktop</strong>
      </div>
      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />
      <div class="grid min-h-[68px] grid-cols-[150px_1fr] items-center gap-6 px-5">
        <span class="text-[13px] text-[#707684] dark:text-[#9ca0aa]">桌面路径</span>
        <strong class="truncate text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">
          {{ desktopPath || "读取中" }}
        </strong>
      </div>
    </div>
  </section>
</template>
