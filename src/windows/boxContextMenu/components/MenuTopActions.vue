<script setup lang="ts">
import { FolderPlus, RefreshCw, Settings } from "@lucide/vue";

/**
 * 顶部三个高频入口保持等宽网格，适合鼠标从 Box 更多按钮进入后的快速操作。
 */
defineEmits<{
  createBox: [];
  openSettings: [];
  refreshDesktop: [];
}>();
</script>

<template>
  <div class="grid grid-cols-3 gap-1">
    <button
      class="flex items-center justify-center gap-1 rounded-[8px] px-1.5 py-1 text-[12px] font-medium transition-colors hover:bg-[#eceef3] dark:hover:bg-[#2b2e37]"
      type="button"
      @click="$emit('openSettings')"
    >
      <Settings class="text-slate-500 dark:text-slate-400" :size="15" />
      设置
    </button>
    <button
      aria-label="新增 Box"
      class="flex items-center justify-center gap-1 rounded-[8px] px-1.5 py-1 text-[12px] font-medium transition-colors hover:bg-[#eceef3] dark:hover:bg-[#2b2e37]"
      title="新增 Box"
      type="button"
      @click="$emit('createBox')"
    >
      <FolderPlus class="text-slate-500 dark:text-slate-400" :size="15" />
      新增
    </button>
    <button
      class="flex items-center justify-center gap-1 rounded-[8px] px-1.5 py-1 text-[12px] font-medium transition-colors hover:bg-[#eceef3] dark:hover:bg-[#2b2e37]"
      type="button"
      @click="$emit('refreshDesktop')"
    >
      <RefreshCw class="text-slate-500 dark:text-slate-400" :size="15" />
      刷新
    </button>
  </div>
</template>
