<script setup lang="ts">
/**
 * 设置分组只负责卡片容器视觉，行间分隔交给 SettingRow 根据顺序处理。
 */
defineProps<{
  class?: string;
}>();
</script>

<template>
  <div
    class="overflow-hidden rounded-[8px] border border-[#dfe2e8] bg-[#ffffff] shadow-[0_10px_28px_rgba(20,24,32,0.06)] dark:border-[#292c34] dark:bg-[#181a20] dark:shadow-none"
    :class="$props.class"
  >
    <slot />
  </div>
</template>
