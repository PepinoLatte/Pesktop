<script setup lang="ts">
import { Trash2 } from "@lucide/vue";
import type { DesktopBox } from "@/entities/desktopBox/types";

/**
 * 删除按钮只展示二次确认态，真实删除和窗口关闭由外层动作统一处理。
 */
defineProps<{
  box: DesktopBox;
  isConfirming: boolean;
}>();

defineEmits<{
  deleteBox: [];
}>();
</script>

<template>
  <button
    :aria-label="isConfirming ? '确认删除 Box' : '删除 Box'"
    class="flex items-center rounded-[7px] px-2.5 py-1 text-left text-[12px] transition-colors"
    :class="
      isConfirming
        ? 'bg-red-600 text-white hover:bg-red-700 dark:bg-red-500 dark:text-white dark:hover:bg-red-600'
        : 'text-red-600 hover:bg-[#fff0f0] dark:text-red-400 dark:hover:bg-[#3a2528]'
    "
    :title="isConfirming ? '再次点击确认删除并执行当前文件夹处理策略' : '删除 Box'"
    type="button"
    @click="$emit('deleteBox')"
  >
    <Trash2 class="mr-2" :size="14" />
    {{ isConfirming ? "确认删除" : "删除 Box" }}
  </button>
</template>
