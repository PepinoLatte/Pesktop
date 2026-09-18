<script setup lang="ts">
/**
 * 设置面板外壳统一标题、说明和内容宽度，避免每个菜单重复维护页面间距。
 */
defineProps<{
  description: string;
  panelWidth: string;
  title: string;
}>();
</script>

<template>
  <section class="mx-auto w-full px-8 py-7" :class="panelWidth">
    <div class="mb-6">
      <h1 class="text-[24px] font-semibold tracking-[0] text-[#17181c] dark:text-[#f4f4f5]">
        {{ title }}
      </h1>
      <p class="mt-1 text-[13px] text-[#6f7480] dark:text-[#a7abb5]">{{ description }}</p>
    </div>

    <slot />
  </section>
</template>
