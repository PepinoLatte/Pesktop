<script setup lang="ts">
import { Minus, Square, X } from "@lucide/vue";

/**
 * 设置窗口头部只负责拖动和 Windows 风格窗口操作，页面标题交给右侧内容区展示。
 */
defineProps<{
  title: string;
}>();

const emit = defineEmits<{
  close: [];
  dragStart: [event: MouseEvent];
  minimize: [];
  toggleMaximize: [];
}>();
</script>

<template>
  <header
    class="flex h-11 shrink-0 select-none items-stretch border-b border-[#dfe2e8] bg-[#fbfbfd] dark:border-[#292c34] dark:bg-[#101116]"
  >
    <div
      class="flex min-w-0 flex-1 items-center px-5 text-left"
      @mousedown.left="emit('dragStart', $event)"
      @dblclick.stop.prevent="emit('toggleMaximize')"
    ></div>

    <div class="flex items-stretch">
      <button
        aria-label="最小化"
        class="grid h-11 w-12 place-items-center text-[#626773] transition-colors hover:bg-[#e9ebf0] hover:text-[#17181c] dark:text-[#a7abb5] dark:hover:bg-[#242730] dark:hover:text-white"
        type="button"
        @click.stop="emit('minimize')"
        @mousedown.stop
      >
        <Minus :size="16" />
      </button>
      <button
        aria-label="最大化"
        class="grid h-11 w-12 place-items-center text-[#626773] transition-colors hover:bg-[#e9ebf0] hover:text-[#17181c] dark:text-[#a7abb5] dark:hover:bg-[#242730] dark:hover:text-white"
        type="button"
        @click.stop="emit('toggleMaximize')"
        @mousedown.stop
      >
        <Square :size="14" />
      </button>
      <button
        aria-label="关闭"
        class="grid h-11 w-12 place-items-center text-[#626773] transition-colors hover:bg-[#e81123] hover:text-white dark:text-[#a7abb5]"
        type="button"
        @click.stop="emit('close')"
        @mousedown.stop
      >
        <X :size="17" />
      </button>
    </div>
  </header>
</template>
