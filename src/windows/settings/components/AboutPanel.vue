<script setup lang="ts">
/**
 * 关于面板只展示可核对的产品元信息，避免把设置页做成完整宣传页
 */
defineProps<{
  desktopPath: string;
  /**
   * 设置页父级统一下发内容宽度，避免各面板各自维护页面密度
   */
  panelWidth: string;
}>();

/**
 * 应用元信息集中维护在关于页，避免展示文案散落在模板中难以同步
 */
const APP_META = {
  author: "Aug_Sakura",
  name: "Dasktop",
  poweredBy: "Codex",
  version: "0.1.0",
} as const;
</script>

<template>
  <section class="mx-auto w-full px-8 py-7" :class="panelWidth">
    <div class="mb-6">
      <h1 class="text-[24px] font-semibold tracking-[0] text-[#17181c] dark:text-[#f4f4f5]">关于</h1>
      <p class="mt-1 text-[13px] text-[#6f7480] dark:text-[#a7abb5]">当前桌面扩展的运行信息</p>
    </div>

    <div class="overflow-hidden rounded-[14px] border border-[#dfe2e8] bg-[#ffffff] shadow-[0_10px_28px_rgba(20,24,32,0.06)] dark:border-[#292c34] dark:bg-[#181a20] dark:shadow-none">
      <div class="grid min-h-[68px] grid-cols-[150px_1fr] items-center gap-6 px-5">
        <span class="text-[13px] text-[#707684] dark:text-[#9ca0aa]">应用名称</span>
        <div class="min-w-0 py-3">
          <strong class="block text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">{{ APP_META.name }}</strong>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">
            名字里藏着作者的 Da，也落在每天打开的 Desktop 上
          </p>
        </div>
      </div>
      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />
      <div class="grid min-h-[68px] grid-cols-[150px_1fr] items-center gap-6 px-5">
        <span class="text-[13px] text-[#707684] dark:text-[#9ca0aa]">作者</span>
        <div class="min-w-0 py-3">
          <strong class="block text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">{{ APP_META.author }}</strong>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">产品设计与开发</p>
        </div>
      </div>
      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />
      <div class="grid min-h-[68px] grid-cols-[150px_1fr] items-center gap-6 px-5">
        <span class="text-[13px] text-[#707684] dark:text-[#9ca0aa]">版本</span>
        <div class="min-w-0 py-3">
          <strong class="block text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">v{{ APP_META.version }}</strong>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">当前演示与试用版本</p>
        </div>
      </div>
      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />
      <div class="grid min-h-[68px] grid-cols-[150px_1fr] items-center gap-6 px-5">
        <span class="text-[13px] text-[#707684] dark:text-[#9ca0aa]">Powered by</span>
        <div class="min-w-0 py-3">
          <strong class="block text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">{{ APP_META.poweredBy }}</strong>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">参与产品构建与实现协作</p>
        </div>
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
