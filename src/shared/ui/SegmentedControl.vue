<script setup lang="ts" generic="Value extends string">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import type { Component } from "vue";
import { animate } from "motion";

/**
 * 分段控件只负责通用选项展示和选择事件，具体业务文案与枚举由调用方维护
 */
const props = withDefaults(
  defineProps<{
    /**
     * 密度只影响控件自身高度和内边距，业务侧仍通过 optionWidthPx 控制横向容量
     */
    density?: "compact" | "default";
    modelValue: Value;
    optionWidthPx?: number;
    options: Array<{
      icon?: Component;
      label: string;
      value: Value;
    }>;
  }>(),
  {
    density: "default",
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
 * 分段项使用固定基础宽度，保证带图标的主题选项和纯文本选项都能居中且不换行
 */
const SEGMENTED_CONTROL_LAYOUT = {
  defaultPaddingWidthPx: 8,
  compactPaddingWidthPx: 4,
} as const;
const optionCount = computed(() => Math.max(props.options.length, 1));
const isCompact = computed(() => props.density === "compact");
const horizontalPaddingWidth = computed(() =>
  isCompact.value
    ? SEGMENTED_CONTROL_LAYOUT.compactPaddingWidthPx
    : SEGMENTED_CONTROL_LAYOUT.defaultPaddingWidthPx,
);
const segmentWidth = computed(
  () => `calc((100% - ${horizontalPaddingWidth.value}px) / ${optionCount.value})`,
);
const controlWidth = computed(
  () => `${optionCount.value * props.optionWidthPx + horizontalPaddingWidth.value}px`,
);
const indicatorTranslateX = computed(() => activeIndex.value * props.optionWidthPx);
const indicatorRef = ref<HTMLElement | null>(null);
let indicatorAnimation: ReturnType<typeof animate> | null = null;

watch(
  indicatorTranslateX,
  (nextTranslateX) => {
    animateSegmentIndicator(nextTranslateX);
  },
  { flush: "post" },
);

onMounted(() => {
  applySegmentIndicatorTransform(indicatorTranslateX.value);
});

onUnmounted(() => {
  indicatorAnimation?.stop();
  indicatorAnimation = null;
});

/**
 * 分段控件滑块统一由 motion 驱动，避免不同调用方各写一套 CSS 动画导致节奏不一致
 */
function animateSegmentIndicator(nextTranslateX: number): void {
  const indicatorElement = indicatorRef.value;
  if (!indicatorElement) {
    return;
  }

  indicatorAnimation?.stop();
  if (shouldReduceMotion()) {
    applySegmentIndicatorTransform(nextTranslateX);
    return;
  }

  indicatorAnimation = animate(
    indicatorElement,
    {
      transform: `translateX(${nextTranslateX}px)`,
    },
    {
      duration: 0.2,
      ease: [0.2, 0.8, 0.2, 1],
    },
  );
  const activeAnimation = indicatorAnimation;
  void activeAnimation.finished
    .catch(() => undefined)
    .finally(() => {
      if (indicatorAnimation === activeAnimation) {
        indicatorAnimation = null;
      }
    });
}

/**
 * 初始渲染和减少动态效果时直接写入滑块位置，避免首帧出现无意义位移
 */
function applySegmentIndicatorTransform(nextTranslateX: number): void {
  if (!indicatorRef.value) {
    return;
  }

  indicatorRef.value.style.transform = `translateX(${nextTranslateX}px)`;
}

/**
 * 系统减少动态效果时，分段控件保持即时切换，不额外制造横移动画
 */
function shouldReduceMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}
</script>

<template>
  <div
    class="relative grid max-w-full overflow-hidden bg-[#eceef3] dark:bg-[#242730]"
    :class="isCompact ? 'rounded-[8px] p-0.5' : 'rounded-[10px] p-1'"
    :style="{
      '--segment-width': segmentWidth,
      gridTemplateColumns: `repeat(${optionCount}, minmax(0, 1fr))`,
      width: controlWidth,
    }"
  >
    <span
      ref="indicatorRef"
      class="dasktop-segment-indicator pointer-events-none absolute bg-[#ffffff] shadow-[0_4px_12px_rgba(20,24,32,0.10)] dark:bg-[#f4f4f5]"
      :class="isCompact ? 'bottom-0.5 left-0.5 top-0.5 rounded-[6px]' : 'bottom-1 left-1 top-1 rounded-[8px]'"
      :style="{
        width: 'var(--segment-width)',
      }"
    />
    <button
      v-for="option in options"
      :key="option.value"
      class="relative z-10 flex min-w-0 items-center justify-center font-medium transition-colors duration-150"
      :class="[
        isCompact ? 'h-7 gap-1.5 rounded-[6px] px-2 text-[11px]' : 'h-8 gap-2 rounded-[8px] px-3 text-[12px]',
        modelValue === option.value
          ? 'text-[#17181c] dark:text-[#17181c]'
          : 'text-[#686e7b] hover:text-[#17181c] dark:text-[#a7abb5] dark:hover:text-[#f4f4f5]',
      ]"
      type="button"
      @click="emit('change', option.value)"
    >
      <component :is="option.icon" v-if="option.icon" :size="15" />
      <span class="whitespace-nowrap">{{ option.label }}</span>
    </button>
  </div>
</template>
