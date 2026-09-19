<script setup lang="ts">
import type { ResizeDirection } from "@/windows/desktop/composables/useBoxWindowFrame";

/**
 * 无边框 Box 通过显式热区提供缩放入口，组件只负责渲染和派发鼠标事件。
 * 热区保持较大命中范围但完全透明；悬停时在边缘中央显示一个小胶囊把手，
 * 代替整条白色边框提示，视觉更轻。
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

/**
 * 每条边/角的把手胶囊样式：居中悬浮的圆角小条，颜色随明暗主题适配。
 */
const HANDLE_PILL_CLASS =
  "pointer-events-none absolute rounded-full bg-slate-500/70 opacity-0 shadow-[0_1px_3px_rgba(15,23,42,0.35)] transition-opacity duration-150 group/resize-handle-hover:opacity-100 dark:bg-slate-300/70";
const HANDLE_PILL_HORIZONTAL = "h-1 w-8 left-1/2 -translate-x-1/2";
const HANDLE_PILL_VERTICAL = "h-8 w-1 top-1/2 -translate-y-1/2";

const handlePillClasses: Record<ResizeDirection, string> = {
  North: `top-0.5 ${HANDLE_PILL_HORIZONTAL}`,
  South: `bottom-0.5 ${HANDLE_PILL_HORIZONTAL}`,
  West: `left-0.5 ${HANDLE_PILL_VERTICAL}`,
  East: `right-0.5 ${HANDLE_PILL_VERTICAL}`,
  NorthWest: "top-1 left-1 h-1.5 w-1.5 rounded-full",
  NorthEast: "top-1 right-1 h-1.5 w-1.5 rounded-full",
  SouthWest: "bottom-1 left-1 h-1.5 w-1.5 rounded-full",
  SouthEast: "bottom-1 right-1 h-1.5 w-1.5 rounded-full",
};
</script>

<template>
  <template v-if="canResizeBox">
    <span
      v-for="handle in resizeHandles"
      :key="handle.direction"
      class="group/resize-handle-hover absolute z-40"
      :class="handle.className"
      @mousedown.stop.prevent="$emit('startResizing', handle.direction, $event)"
      @mouseenter="$emit('resizeHandleMouseEnter')"
      @mouseleave="$emit('resizeHandleMouseLeave')"
    >
      <span aria-hidden="true" :class="[HANDLE_PILL_CLASS, handlePillClasses[handle.direction]]" />
    </span>
  </template>
</template>
