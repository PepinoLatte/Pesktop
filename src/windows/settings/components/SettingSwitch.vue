<script setup lang="ts">
/**
 * 设置页开关统一视觉和交互尺寸，避免不同面板出现不等高开关。
 */
defineProps<{
  ariaLabel: string;
  modelValue: boolean;
}>();

const emit = defineEmits<{
  change: [value: boolean];
}>();
</script>

<template>
  <label class="relative block h-7 w-12">
    <input
      class="peer sr-only"
      :aria-label="ariaLabel"
      :checked="modelValue"
      type="checkbox"
      @change="emit('change', ($event.target as HTMLInputElement).checked)"
    />
    <span
      class="absolute inset-0 rounded-full bg-[#c9cdd6] transition-colors peer-checked:bg-[#ff5c5c] dark:bg-[#3a3d46] dark:peer-checked:bg-[#ff6b6b]"
    />
    <span
      class="absolute left-1 top-1 size-5 rounded-full bg-white shadow-[0_2px_7px_rgba(20,24,32,0.25)] transition-transform peer-checked:translate-x-5"
    />
  </label>
</template>
