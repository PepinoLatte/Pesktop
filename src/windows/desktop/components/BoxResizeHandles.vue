<script setup lang="ts">
import type { ResizeDirection } from "@/windows/desktop/composables/useBoxWindowFrame";

/**
 * 无边框 Box 通过显式热区提供缩放入口，组件只负责渲染和派发鼠标事件。
 * 热区保持较大命中范围但完全透明；悬停时在边缘中央显示气泡胶囊小短条，
 * 四角显示精致圆角微空间把手，美观、轻量、不遮挡内容。
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

const EDGE_PILL_BASE =
  "pointer-events-none absolute rounded-full bg-white/80 dark:bg-white/70 opacity-0 shadow-[0_2px_6px_rgba(0,0,0,0.25)] ring-1 ring-black/10 backdrop-blur-sm transition-all duration-200 group-hover/resize:opacity-100";

const CORNER_SPACE_BASE =
  "pointer-events-none absolute size-2.5 rounded-full bg-white/85 dark:bg-white/75 opacity-0 shadow-[0_2px_6px_rgba(0,0,0,0.3)] ring-1 ring-black/10 backdrop-blur-sm transition-all duration-200 group-hover/resize:opacity-100 group-hover/resize:scale-110";

const handleIndicatorClasses: Record<ResizeDirection, string> = {
  North: `${EDGE_PILL_BASE} top-1 left-1/2 -translate-x-1/2 h-1 w-8`,
  South: `${EDGE_PILL_BASE} bottom-1 left-1/2 -translate-x-1/2 h-1 w-8`,
  West: `${EDGE_PILL_BASE} left-1 top-1/2 -translate-y-1/2 w-1 h-8`,
  East: `${EDGE_PILL_BASE} right-1 top-1/2 -translate-y-1/2 w-1 h-8`,
  NorthWest: `${CORNER_SPACE_BASE} top-1.5 left-1.5`,
  NorthEast: `${CORNER_SPACE_BASE} top-1.5 right-1.5`,
  SouthWest: `${CORNER_SPACE_BASE} bottom-1.5 left-1.5`,
  SouthEast: `${CORNER_SPACE_BASE} bottom-1.5 right-1.5`,
};
</script>

<template>
  <template v-if="canResizeBox">
    <span
      v-for="handle in resizeHandles"
      :key="handle.direction"
      class="group/resize absolute z-40 bg-transparent"
      :class="handle.className"
      @mousedown.stop.prevent="$emit('startResizing', handle.direction, $event)"
      @mouseenter="$emit('resizeHandleMouseEnter')"
      @mouseleave="$emit('resizeHandleMouseLeave')"
    >
      <span aria-hidden="true" :class="handleIndicatorClasses[handle.direction]" />
    </span>
  </template>
</template>
