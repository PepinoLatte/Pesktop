<script setup lang="ts">
import { RotateCcw } from "@lucide/vue";

/**
 * 设置页滑块统一数值展示和单项重置入口，具体范围来自应用设置校验配置。
 */
defineProps<{
  defaultValue: number;
  label: string;
  max: number;
  min: number;
  modelValue: number;
  step: number;
  unit: string;
}>();

const emit = defineEmits<{
  change: [value: number];
  reset: [];
}>();
</script>

<template>
  <div class="flex items-center gap-3">
    <input
      class="h-1.5 w-full cursor-pointer appearance-none rounded-full bg-[#d8dbe3] accent-[#ff5c5c] dark:bg-[#333640] dark:accent-[#ff6b6b]"
      :aria-label="label"
      :max="max"
      :min="min"
      :step="step"
      type="range"
      :value="modelValue"
      @input="emit('change', Number(($event.target as HTMLInputElement).value))"
    />
    <span class="w-[70px] shrink-0 whitespace-nowrap text-right text-[13px] font-medium text-[#555b66] dark:text-[#c7cad1]">
      {{ modelValue }} {{ unit }}
    </span>
    <button
      :aria-label="`重置${label}`"
      class="grid size-8 shrink-0 place-items-center rounded-[6px] text-[#68707d] transition-colors hover:bg-[#eef0f4] hover:text-[#17181c] disabled:cursor-default disabled:opacity-35 disabled:hover:bg-transparent disabled:hover:text-[#68707d] dark:text-[#a7abb5] dark:hover:bg-[#242730] dark:hover:text-[#f4f4f5] dark:disabled:hover:bg-transparent dark:disabled:hover:text-[#a7abb5]"
      :disabled="modelValue === defaultValue"
      :title="`重置${label}`"
      type="button"
      @click="emit('reset')"
    >
      <RotateCcw :size="15" />
    </button>
  </div>
</template>
