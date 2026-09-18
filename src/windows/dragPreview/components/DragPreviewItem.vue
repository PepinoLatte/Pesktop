<script setup lang="ts">
import { computed } from "vue";
import { formatDesktopItemDisplayName } from "@/entities/desktopItem/displayName";
import type { DesktopItem } from "@/entities/desktopItem/types";
import type { BoxFileDragPreviewOptions } from "@/shared/ipc/boxFileDrag";
import DesktopIconGlyph from "@/windows/desktop/components/DesktopIconGlyph.vue";
import {
  resolvePreviewLabelStyle,
  resolvePreviewSurfaceStyle,
} from "@/windows/dragPreview/model/previewLayout";

/**
 * 拖影项只展示当前拖拽会话的代表文件，数量角标用于提示多选拖拽规模。
 */
const props = defineProps<{
  item: DesktopItem;
  itemCount: number;
  preview: BoxFileDragPreviewOptions;
}>();

const displayName = computed(() =>
  formatDesktopItemDisplayName(props.item, props.preview.nameDisplayMode),
);
const previewSurfaceStyle = computed(() => resolvePreviewSurfaceStyle(props.preview));
const previewLabelStyle = computed(() => resolvePreviewLabelStyle(props.preview));
</script>

<template>
  <div
    class="pointer-events-none relative flex min-w-0 select-none flex-col items-center justify-start bg-white/55 text-center text-slate-900 opacity-90 shadow-[0_8px_20px_rgba(15,23,42,0.14)] dark:bg-white/10 dark:text-white"
    :style="previewSurfaceStyle"
  >
    <DesktopIconGlyph
      :icon-size="preview.iconSize"
      :item="item"
      :radius-size="preview.radiusSize"
      :show-shortcut-arrow="preview.showShortcutArrow"
    />
    <span
      v-if="itemCount > 1"
      class="absolute right-1 top-1 grid min-w-5 place-items-center rounded-full bg-[#2f6bff] px-1.5 text-[11px] font-semibold leading-5 text-white shadow-[0_4px_10px_rgba(47,107,255,0.35)]"
    >
      {{ itemCount }}
    </span>
    <span
      v-if="preview.showItemLabels"
      class="overflow-hidden [overflow-wrap:anywhere]"
      :style="previewLabelStyle"
    >
      {{ displayName }}
    </span>
  </div>
</template>
