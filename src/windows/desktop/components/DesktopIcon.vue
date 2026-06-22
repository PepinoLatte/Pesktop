<script setup lang="ts">
import { computed } from "vue";
import type { CSSProperties } from "vue";
import { formatDesktopItemDisplayName } from "@/entities/desktopItem/displayName";
import type { DesktopItem, DesktopNameDisplayMode } from "@/entities/desktopItem/types";
import { DESKTOP_ICON_VIEW } from "../config/desktopIcon";
import DesktopIconGlyph from "./DesktopIconGlyph.vue";

const props = defineProps<{
  editing?: boolean;
  dragging?: boolean;
  dragInteractionDisabled?: boolean;
  iconSize: number;
  item: DesktopItem;
  labelTextSize: number;
  labelWidth: number;
  nameDisplayMode: DesktopNameDisplayMode;
  radiusSize: number;
  renameDraft?: string;
  selected?: boolean;
  showLabel?: boolean;
  showShortcutArrow?: boolean;
}>();

const emit = defineEmits<{
  cancelRename: [];
  commitRename: [];
  itemClick: [event: MouseEvent, item: DesktopItem];
  itemContextMenu: [event: MouseEvent, item: DesktopItem];
  itemDoubleClick: [item: DesktopItem];
  itemPointerDown: [event: PointerEvent, item: DesktopItem];
  renameDraftChange: [value: string];
}>();

const displayName = computed(() =>
  formatDesktopItemDisplayName(props.item, props.nameDisplayMode),
);
const resolvedItemWidth = computed(() =>
  Math.max(
    props.labelWidth,
    props.iconSize + DESKTOP_ICON_VIEW.itemInlinePadding * 2,
  ),
);
const labelLineHeight = computed(() => Math.max(14, Math.ceil(props.labelTextSize * 1.32)));
const labelBlockHeight = computed(() => labelLineHeight.value * 2 + 2);
const iconButtonStyle = computed(
  () =>
    ({
      borderRadius: `${props.radiusSize}px`,
      gap: props.showLabel === false ? "0px" : `${DESKTOP_ICON_VIEW.labelGap}px`,
      padding: `${DESKTOP_ICON_VIEW.itemBlockPadding}px ${DESKTOP_ICON_VIEW.itemInlinePadding}px`,
      width: `${resolvedItemWidth.value}px`,
    }) as CSSProperties,
);
const labelStyle = computed(
  () =>
    ({
      WebkitBoxOrient: "vertical",
      WebkitLineClamp: "2",
      display: "-webkit-box",
      fontSize: `${props.labelTextSize}px`,
      lineHeight: `${labelLineHeight.value}px`,
      maxHeight: `${labelBlockHeight.value}px`,
      paddingBottom: "2px",
      textOverflow: "ellipsis",
      width: `${props.labelWidth}px`,
    }) as CSSProperties,
);

/**
 * 文件重命名输入需要阻止事件冒泡，否则 F2/Enter 会继续触发窗口级快捷键。
 */
function updateRenameDraft(event: Event): void {
  const target = event.target;
  if (target instanceof HTMLInputElement) {
    emit("renameDraftChange", target.value);
  }
}
</script>

<template>
  <button
    class="dasktop-icon-button relative flex min-w-0 cursor-default select-none flex-col items-center justify-start self-start bg-transparent text-center text-slate-900 transition-colors hover:bg-white/55 active:bg-white/75 dark:text-white dark:hover:bg-white/10 dark:active:bg-white/20"
    :class="[
      selected ? 'bg-white/70 ring-1 ring-[#2f6bff]/70 dark:bg-white/15' : '',
      dragging ? 'opacity-60' : '',
      dragInteractionDisabled ? 'pointer-events-none' : '',
    ]"
    :data-box-item-path="item.path"
    :style="iconButtonStyle"
    type="button"
    :title="item.path"
    @click.stop="emit('itemClick', $event, item)"
    @contextmenu.prevent.stop="emit('itemContextMenu', $event, item)"
    @dblclick.prevent.stop="emit('itemDoubleClick', item)"
    @pointerdown="emit('itemPointerDown', $event, item)"
  >
    <DesktopIconGlyph
      :icon-size="iconSize"
      :item="item"
      :radius-size="radiusSize"
      :show-shortcut-arrow="showShortcutArrow"
    />
    <input
      v-if="editing"
      data-rename-input="true"
      :value="renameDraft"
      class="w-full rounded-[5px] border border-[#2f6bff]/60 bg-white/95 px-1 text-center text-slate-950 outline-none dark:bg-slate-950 dark:text-white"
      :style="{ fontSize: `${labelTextSize}px`, lineHeight: `${labelLineHeight}px` }"
      @blur="emit('commitRename')"
      @input="updateRenameDraft"
      @keydown.enter.prevent.stop="emit('commitRename')"
      @keydown.esc.prevent.stop="emit('cancelRename')"
      @mousedown.stop
    />
    <span
      v-else-if="showLabel !== false"
      class="overflow-hidden [overflow-wrap:anywhere] text-slate-700 dark:text-slate-200"
      :style="labelStyle"
    >
      {{ displayName }}
    </span>
  </button>
</template>
