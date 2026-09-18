<script setup lang="ts">
import type { ResizeDirection } from "@/windows/desktop/composables/useBoxWindowFrame";

/**
 * Box 缩放把手组件：
 * 覆盖 Box 四方边缘（上、下、左、右）及四个拐角。
 * 静默或鼠标不移到把手周围时完全隐藏（opacity-0），视点移近把手时平滑浮现指示条，
 * 按住即可自由拖动调整 Box 大小。
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
 * 根据把手方向计算对应边缘的高亮指示条样式
 */
function resolveIndicatorClass(direction: ResizeDirection): string {
  switch (direction) {
    case "North":
      return "h-1 w-14 top-0.5";
    case "South":
      return "h-1 w-14 bottom-0.5";
    case "West":
      return "w-1 h-14 left-0.5";
    case "East":
      return "w-1 h-14 right-0.5";
    case "NorthWest":
      return "size-2 top-1 left-1 rounded-sm";
    case "NorthEast":
      return "size-2 top-1 right-1 rounded-sm";
    case "SouthWest":
      return "size-2 bottom-1 left-1 rounded-sm";
    case "SouthEast":
      return "size-2 bottom-1 right-1 rounded-sm";
  }
}
</script>

<template>
  <template v-if="canResizeBox">
    <div
      v-for="handle in resizeHandles"
      :key="handle.direction"
      class="group/handle absolute z-40 flex items-center justify-center transition-all duration-150"
      :class="handle.className"
      title="拖动调整大小"
      @mousedown.stop.prevent="$emit('startResizing', handle.direction, $event)"
      @mouseenter="$emit('resizeHandleMouseEnter')"
      @mouseleave="$emit('resizeHandleMouseLeave')"
    >
      <!-- 视觉把手指示器：静默状态下完全隐藏，鼠标挪到把手周围时平滑浮现 -->
      <span
        class="pointer-events-none absolute rounded-full bg-[#2f6bff] shadow-[0_0_10px_rgba(47,107,255,0.7)] opacity-0 transition-all duration-200 group-hover/handle:opacity-100"
        :class="resolveIndicatorClass(handle.direction)"
      />
    </div>
  </template>
</template>
