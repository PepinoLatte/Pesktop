<script setup lang="ts">
import type { ComponentPublicInstance, CSSProperties } from "vue";
import type { AppSettings } from "@/entities/appSettings/types";
import type { DesktopItem } from "@/entities/desktopItem/types";
import type {
  BoxSortInsertionPlacement,
  BoxSortInsertionPreview,
} from "@/windows/desktop/model/fileDrag";
import DesktopIcon from "@/windows/desktop/components/DesktopIcon.vue";

/**
 * Box 文件网格只负责渲染文件项、空状态和框选层，文件动作通过事件交给外层组合式逻辑。
 */
defineProps<{
  boxBodyStyle: CSSProperties;
  boxGridOverflowClass: string;
  boxGridStyle: CSSProperties;
  boxItems: DesktopItem[];
  editingPath: string | null;
  isBoxFileDragActive: boolean;
  isItemDragging: (item: DesktopItem) => boolean;
  isItemSelected: (item: DesktopItem) => boolean;
  isSelecting: boolean;
  renameDraft: string;
  selectionRectStyle: CSSProperties;
  setGridRef: (element: Element | ComponentPublicInstance | null) => void;
  sortInsertionPreview: BoxSortInsertionPreview | null;
  settings: AppSettings;
}>();

defineEmits<{
  "cancel-rename": [];
  "commit-rename": [];
  "file-view-keydown": [event: KeyboardEvent];
  "grid-pointer-down": [event: PointerEvent];
  "item-click": [event: MouseEvent, item: DesktopItem];
  "item-context-menu": [event: MouseEvent, item: DesktopItem];
  "item-double-click": [item: DesktopItem];
  "item-pointer-down": [event: PointerEvent, item: DesktopItem];
  "rename-draft-change": [value: string];
}>();

/**
 * 单个图标只关心插入线是否贴在自身左右侧，末尾语义由外层排序逻辑转换成最后一个目标图标。
 */
function resolveItemSortInsertionPlacement(
  item: DesktopItem,
  preview: BoxSortInsertionPreview | null,
): Exclude<BoxSortInsertionPlacement, "end"> | null {
  if (!preview || preview.targetPath !== item.path || preview.placement === "end") {
    return null;
  }

  return preview.placement;
}
</script>

<template>
  <div
    :ref="setGridRef"
    class="dasktop-scrollarea dasktop-box-scrollarea relative grid min-h-0 flex-1 p-2.5 outline-none"
    :class="[
      boxGridOverflowClass,
      boxItems.length === 0
        ? 'content-center place-items-center justify-center'
        : 'content-start items-start justify-start',
    ]"
    :style="[boxGridStyle, boxBodyStyle]"
    tabindex="0"
    @keydown="$emit('file-view-keydown', $event)"
    @pointerdown="$emit('grid-pointer-down', $event)"
  >
    <DesktopIcon
      v-for="item in boxItems"
      :key="item.path"
      :drag-interaction-disabled="isBoxFileDragActive"
      :dragging="isItemDragging(item)"
      :editing="editingPath === item.path"
      :icon-size="settings.boxIconSize"
      :item="item"
      :label-text-size="settings.boxLabelTextSize"
      :label-width="settings.boxFilenameWidth"
      :name-display-mode="settings.nameDisplayMode"
      :radius-size="settings.boxIconBorderRadius"
      :rename-draft="renameDraft"
      :selected="isItemSelected(item)"
      :show-label="settings.showItemLabels"
      :show-shortcut-arrow="settings.showShortcutArrow"
      :sort-insertion-placement="resolveItemSortInsertionPlacement(item, sortInsertionPreview)"
      @commit-rename="$emit('commit-rename')"
      @cancel-rename="$emit('cancel-rename')"
      @item-click="$emit('item-click', $event, item)"
      @item-context-menu="$emit('item-context-menu', $event, item)"
      @item-double-click="$emit('item-double-click', item)"
      @item-pointer-down="$emit('item-pointer-down', $event, item)"
      @rename-draft-change="$emit('rename-draft-change', $event)"
    />

    <div
      v-if="boxItems.length === 0"
      class="col-span-full flex min-h-[120px] w-full flex-col items-center justify-center px-5 text-center"
    >
      <strong class="block text-center text-[13px] font-semibold text-slate-900 dark:text-white">这个 Box 还是空的</strong>
      <p class="mt-1 max-w-[180px] text-center text-[12px] leading-5 text-slate-600 dark:text-slate-300">把桌面文件拖进来就能开始整理</p>
    </div>

    <div
      v-if="isSelecting"
      class="pointer-events-none absolute z-20 border border-[#2f6bff]/70 bg-[#2f6bff]/15"
      :style="selectionRectStyle"
    />
  </div>
</template>
