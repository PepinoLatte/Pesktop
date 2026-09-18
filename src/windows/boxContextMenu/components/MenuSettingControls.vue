<script setup lang="ts">
import { computed } from "vue";
import SegmentedControl from "@/shared/ui/SegmentedControl.vue";
import type { DesktopBox, DesktopBoxTitlePosition } from "@/entities/desktopBox/types";
import { BOX_PRESET_ICONS } from "@/entities/desktopBox/presetIcons";
import BoxIconGlyph from "@/windows/desktop/components/BoxIconGlyph.vue";
import {
  BOX_AUTO_COLLAPSE_OPTIONS,
  BOX_MENU_SEGMENT_WIDTH,
  BOX_TITLE_POSITION_OPTIONS,
  type BoxAutoCollapseMode,
} from "@/windows/boxContextMenu/model/menuOptions";

/**
 * 菜单设置控件承载当前 Box 的轻量布局偏好与预选图标库
 */
const props = defineProps<{
  box: DesktopBox;
  boxAutoCollapseMode: BoxAutoCollapseMode;
}>();

defineEmits<{
  updateAutoCollapse: [mode: BoxAutoCollapseMode];
  updateTitlePosition: [position: DesktopBoxTitlePosition];
  updateBoxIcon: [icon: string];
  updateBoxSizeMode: [mode: "window" | "icon"];
}>();

const isCompactMode = computed(() => props.box.width <= 110 || props.box.height <= 110);
const currentSizeMode = computed(() => (isCompactMode.value ? "icon" : "window"));

const SIZE_MODE_OPTIONS = [
  { label: "窗口", value: "window" as const },
  { label: "小图标", value: "icon" as const },
];

const currentPresetIconLabel = computed(() => {
  const currentId = props.box.icon || "Folder";
  const matched = BOX_PRESET_ICONS.find((item) => item.id === currentId);
  return matched ? matched.label : "默认";
});
</script>

<template>
  <div class="grid grid-cols-[58px_1fr] items-center gap-2 px-1 py-0.5">
    <span class="text-[11px] font-medium text-slate-500 dark:text-slate-400">窗口形态</span>
    <SegmentedControl
      density="compact"
      :model-value="currentSizeMode"
      :option-width-px="56"
      :options="SIZE_MODE_OPTIONS"
      @change="$emit('updateBoxSizeMode', $event)"
    />
  </div>
  <div v-if="!isCompactMode" class="grid grid-cols-[58px_1fr] items-center gap-2 px-1 py-0.5">
    <span class="text-[11px] font-medium text-slate-500 dark:text-slate-400">标题位置</span>
    <SegmentedControl
      density="compact"
      :model-value="box.titlePosition"
      :option-width-px="BOX_MENU_SEGMENT_WIDTH.titlePosition"
      :options="BOX_TITLE_POSITION_OPTIONS"
      @change="$emit('updateTitlePosition', $event)"
    />
  </div>
  <div v-if="!isCompactMode" class="grid grid-cols-[58px_1fr] items-center gap-2 px-1 py-0.5">
    <span class="text-[11px] font-medium text-slate-500 dark:text-slate-400">自动收起</span>
    <SegmentedControl
      density="compact"
      :model-value="boxAutoCollapseMode"
      :option-width-px="BOX_MENU_SEGMENT_WIDTH.collapse"
      :options="BOX_AUTO_COLLAPSE_OPTIONS"
      @change="$emit('updateAutoCollapse', $event)"
    />
  </div>
  <div class="grid gap-1 px-1 pt-1.5 pb-0.5">
    <div class="flex items-center justify-between text-[11px] font-medium text-slate-500 dark:text-slate-400">
      <span>预选图标与封面</span>
      <span class="text-[10px] text-[#2f6bff] font-normal">{{ currentPresetIconLabel }}</span>
    </div>
    <div class="grid grid-cols-6 gap-1 pt-1">
      <button
        v-for="item in BOX_PRESET_ICONS"
        :key="item.id"
        type="button"
        :class="box.icon === item.id || (!box.icon && item.id === 'Folder') ? 'bg-[#2f6bff] text-white shadow-sm ring-1 ring-[#2f6bff]' : 'bg-[#eef1f6] hover:bg-[#e2e6ef] text-slate-700 dark:bg-[#2c2f38] dark:hover:bg-[#383c47] dark:text-slate-200'"
        class="grid size-7 place-items-center rounded-[6px] transition-all cursor-pointer"
        :title="item.label"
        @click="$emit('updateBoxIcon', item.id)"
      >
        <BoxIconGlyph :icon="item.id" :size="14" />
      </button>
    </div>
  </div>
</template>
