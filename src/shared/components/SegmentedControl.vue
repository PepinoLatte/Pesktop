<script setup lang="ts" generic="Value extends string">
import { computed } from "vue";
import type { Component } from "vue";

/**
 * 分段控件只负责通用选项展示和选择事件，具体业务文案与枚举由调用方维护。
 */
const props = withDefaults(
  defineProps<{
    modelValue: Value;
    optionWidthPx?: number;
    options: Array<{
      icon?: Component;
      label: string;
      value: Value;
    }>;
  }>(),
  {
    optionWidthPx: 108,
  },
);

const emit = defineEmits<{
  change: [value: Value];
}>();

const activeIndex = computed(() =>
  Math.max(
    props.options.findIndex((option) => option.value === props.modelValue),
    0,
  ),
);
/**
 * 分段项使用固定基础宽度，保证带图标的主题选项和纯文本选项都能居中且不换行。
 */
const SEGMENTED_CONTROL_LAYOUT = {
  paddingWidthPx: 8,
} as const;
const optionCount = computed(() => Math.max(props.options.length, 1));
const segmentWidth = computed(() => `calc((100% - 0.5rem) / ${optionCount.value})`);
const segmentIndex = computed(() => String(activeIndex.value));
const controlWidth = computed(
  () =>
    `${optionCount.value * props.optionWidthPx + SEGMENTED_CONTROL_LAYOUT.paddingWidthPx}px`,
);
</script>

<template>
  <div
    class="relative grid max-w-full overflow-hidden rounded-[10px] bg-[#eceef3] p-1 dark:bg-[#242730]"
    :style="{
      '--segment-width': segmentWidth,
      '--segment-index': segmentIndex,
      gridTemplateColumns: `repeat(${optionCount}, minmax(0, 1fr))`,
      width: controlWidth,
    }"
  >
    <span
      class="dasktop-segment-indicator pointer-events-none absolute bottom-1 left-1 top-1 rounded-[8px] bg-[#ffffff] shadow-[0_4px_12px_rgba(20,24,32,0.10)] transition-transform duration-200 ease-out motion-reduce:transition-none dark:bg-[#f4f4f5]"
      :style="{
        width: 'var(--segment-width)',
        transform: `translateX(calc(var(--segment-index) * 100%))`,
      }"
    />
    <button
      v-for="option in options"
      :key="option.value"
      class="relative z-10 flex h-8 min-w-0 items-center justify-center gap-2 rounded-[8px] px-3 text-[12px] font-medium transition-colors duration-150"
      :class="
        modelValue === option.value
          ? 'text-[#17181c] dark:text-[#17181c]'
          : 'text-[#686e7b] hover:text-[#17181c] dark:text-[#a7abb5] dark:hover:text-[#f4f4f5]'
      "
      type="button"
      @click="emit('change', option.value)"
    >
      <component :is="option.icon" v-if="option.icon" :size="15" />
      <span class="whitespace-nowrap">{{ option.label }}</span>
    </button>
  </div>
</template>
