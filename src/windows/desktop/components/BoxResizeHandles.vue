<script setup lang="ts">
import type { ResizeDirection } from "@/windows/desktop/composables/useBoxWindowFrame";

/**
 * 无边框 Box 通过显式热区提供缩放入口，组件只负责渲染和派发鼠标事件。
 */
defineProps<{
  canResizeBox: boolean;
  resizeHandles: Array<{
    className: string;
    direction: ResizeDirection;
  }>;
}>();

defineEmits<{
  resizeHandleMouseEnter: [];
  resizeHandleMouseLeave: [];
  startResizing: [direction: ResizeDirection, event: MouseEvent];
}>();
</script>

<template>
  <template v-if="canResizeBox">
    <span
      v-for="handle in resizeHandles"
      :key="handle.direction"
      class="absolute z-40"
      :class="handle.className"
      @mousedown.stop.prevent="$emit('startResizing', handle.direction, $event)"
      @mouseenter="$emit('resizeHandleMouseEnter')"
      @mouseleave="$emit('resizeHandleMouseLeave')"
    />
  </template>
</template>
