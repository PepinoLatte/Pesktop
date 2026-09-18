<script setup lang="ts">
import DragPreviewItem from "@/windows/dragPreview/components/DragPreviewItem.vue";
import { useDragPreviewWindow } from "@/windows/dragPreview/composables/useDragPreviewWindow";

/**
 * 拖影窗口只负责承载透明预览层，拖拽数据和窗口位置均由 composable 管理。
 */
const { item, itemCount, preview } = useDragPreviewWindow();
</script>

<template>
  <main class="grid h-screen w-screen place-items-start overflow-hidden bg-transparent p-1">
    <DragPreviewItem
      v-if="item && preview"
      :item="item"
      :item-count="itemCount"
      :preview="preview"
    />
  </main>
</template>
