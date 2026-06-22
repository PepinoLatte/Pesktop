<script setup lang="ts">
import SegmentedControl from "@/shared/ui/SegmentedControl.vue";
import type { DesktopBox, DesktopBoxTitlePosition } from "@/entities/desktopBox/types";
import {
  BOX_AUTO_COLLAPSE_OPTIONS,
  BOX_MENU_SEGMENT_WIDTH,
  BOX_TITLE_POSITION_OPTIONS,
  type BoxAutoCollapseMode,
} from "@/windows/boxContextMenu/model/menuOptions";

/**
 * 菜单设置控件只承载当前 Box 的轻量布局偏好，复杂设置仍留在设置窗口中处理。
 */
defineProps<{
  box: DesktopBox;
  boxAutoCollapseMode: BoxAutoCollapseMode;
}>();

defineEmits<{
  updateAutoCollapse: [mode: BoxAutoCollapseMode];
  updateTitlePosition: [position: DesktopBoxTitlePosition];
}>();
</script>

<template>
  <div class="grid grid-cols-[58px_1fr] items-center gap-2 px-1 py-0.5">
    <span class="text-[11px] font-medium text-slate-500 dark:text-slate-400">标题位置</span>
    <SegmentedControl
      density="compact"
      :model-value="box.titlePosition"
      :option-width-px="BOX_MENU_SEGMENT_WIDTH.titlePosition"
      :options="BOX_TITLE_POSITION_OPTIONS"
      @change="$emit('updateTitlePosition', $event)"
    />
  </div>
  <div class="grid grid-cols-[58px_1fr] items-center gap-2 px-1 py-0.5">
    <span class="text-[11px] font-medium text-slate-500 dark:text-slate-400">自动收起</span>
    <SegmentedControl
      density="compact"
      :model-value="boxAutoCollapseMode"
      :option-width-px="BOX_MENU_SEGMENT_WIDTH.collapse"
      :options="BOX_AUTO_COLLAPSE_OPTIONS"
      @change="$emit('updateAutoCollapse', $event)"
    />
  </div>
</template>
