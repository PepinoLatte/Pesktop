<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
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
  sortInsertionPlacement?: "after" | "before" | null;
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
const renameTextareaRef = ref<HTMLTextAreaElement | null>(null);
const renameTextareaHeight = ref(0);
/**
 * 外层网格的行高由窗口缩放逻辑统一控制，图标按钮本身只按内容收缩，避免预留两行时把一行背景也铺满。
 */
const iconButtonStyle = computed(
  () =>
    ({
      borderRadius: `${props.radiusSize}px`,
      gap: props.showLabel === false ? "0px" : `${DESKTOP_ICON_VIEW.labelGap}px`,
      padding: `${DESKTOP_ICON_VIEW.itemBlockPadding}px ${DESKTOP_ICON_VIEW.itemInlinePadding}px`,
      width: `${resolvedItemWidth.value}px`,
    }) as CSSProperties,
);
/**
 * 重命名时只保留输入框自身的焦点样式，避免父级图标的选中态和按压态干扰 Windows 式重命名体验。
 */
const iconStateClasses = computed(() => [
  !props.editing ? "active:bg-white/75 dark:active:bg-white/20" : "",
  props.selected && !props.editing
    ? "z-30 bg-white/70 ring-1 ring-[#2f6bff]/70 dark:bg-white/15"
    : "",
  props.dragging ? "opacity-60" : "",
  props.dragInteractionDisabled ? "pointer-events-none" : "",
]);
/**
 * 排序插入线贴在图标外缘展示目标位置，使用绝对定位避免改变网格测量和真实落点。
 */
const sortInsertionIndicatorClasses = computed(() =>
  props.sortInsertionPlacement === "before" ? "-left-1" : "-right-1",
);
const labelStyle = computed(
  () => {
    const baseStyle = {
      fontSize: `${props.labelTextSize}px`,
      lineHeight: `${labelLineHeight.value}px`,
      paddingBottom: "2px",
      width: `${props.labelWidth}px`,
    } satisfies CSSProperties;
    if (props.selected) {
      return {
        ...baseStyle,
        display: "block",
        maxHeight: "none",
        overflow: "visible",
        textOverflow: "clip",
      } as CSSProperties;
    }

    return {
      ...baseStyle,
      WebkitBoxOrient: "vertical",
      WebkitLineClamp: "2",
      display: "-webkit-box",
      maxHeight: `${labelBlockHeight.value}px`,
      textOverflow: "ellipsis",
    } as CSSProperties;
  },
);
const renameInputStyle = computed(
  () =>
    ({
      fontSize: `${props.labelTextSize}px`,
      height: `${Math.max(renameTextareaHeight.value, labelBlockHeight.value)}px`,
      lineHeight: `${labelLineHeight.value}px`,
      minHeight: `${labelBlockHeight.value}px`,
      width: `${props.labelWidth}px`,
    }) as CSSProperties,
);

watch(
  () => [props.editing, props.renameDraft, props.labelTextSize, props.labelWidth],
  () => {
    void nextTick(syncRenameTextareaHeight);
  },
  { immediate: true },
);

/**
 * 重命名输入框高度必须以浏览器真实换行结果为准，避免中英文混排或长文件名被字符数估算截断。
 */
function syncRenameTextareaHeight(): void {
  const textarea = renameTextareaRef.value;
  if (!props.editing || !textarea) {
    renameTextareaHeight.value = 0;
    return;
  }

  // 先释放旧高度再测量，随后同一轮同步写回 DOM，避免输入时短暂停留在默认两行高度。
  textarea.style.height = "auto";
  const nextHeight = Math.max(textarea.scrollHeight + 2, labelBlockHeight.value);
  textarea.style.height = `${nextHeight}px`;
  renameTextareaHeight.value = nextHeight;
}

/**
 * 文件重命名输入需要阻止事件冒泡，否则 F2/Enter 会继续触发窗口级快捷键。
 */
function updateRenameDraft(event: Event): void {
  const target = event.target;
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
    emit("renameDraftChange", target.value);
    void nextTick(syncRenameTextareaHeight);
  }
}
</script>

<template>
  <div
    class="dasktop-icon-button relative flex h-fit min-w-0 cursor-default select-none flex-col items-center justify-start self-start bg-transparent text-center text-slate-900 transition-colors hover:bg-white/55 dark:text-white dark:hover:bg-white/10"
    :class="iconStateClasses"
    :data-box-item-path="item.path"
    role="button"
    :style="iconButtonStyle"
    tabindex="-1"
    :title="item.path"
    @click.stop="emit('itemClick', $event, item)"
    @contextmenu.prevent.stop="emit('itemContextMenu', $event, item)"
    @dblclick.prevent.stop="emit('itemDoubleClick', item)"
    @pointerdown="emit('itemPointerDown', $event, item)"
  >
    <span
      v-if="sortInsertionPlacement"
      aria-hidden="true"
      class="pointer-events-none absolute bottom-1 top-1 z-50 w-[3px] rounded-full bg-[#2f6bff] shadow-[0_0_0_2px_rgba(47,107,255,0.2),0_4px_10px_rgba(47,107,255,0.34)]"
      :class="sortInsertionIndicatorClasses"
    />
    <DesktopIconGlyph
      :icon-size="iconSize"
      :item="item"
      :radius-size="radiusSize"
      :show-shortcut-arrow="showShortcutArrow"
    />
    <textarea
      v-if="editing"
      ref="renameTextareaRef"
      data-rename-input="true"
      :value="renameDraft"
      class="z-40 box-border flex-none resize-none overflow-hidden rounded-[5px] border border-[#2f6bff]/60 bg-white/95 px-1 text-center text-slate-950 outline-none [overflow-wrap:anywhere] dark:bg-slate-950 dark:text-white"
      :style="renameInputStyle"
      wrap="soft"
      @blur="emit('commitRename')"
      @click.stop
      @input="updateRenameDraft"
      @keydown.enter.prevent.stop="emit('commitRename')"
      @keydown.esc.prevent.stop="emit('cancelRename')"
      @mousedown.stop
      @pointerdown.stop
    />
    <span
      v-else-if="showLabel !== false"
      class="overflow-hidden [overflow-wrap:anywhere] text-slate-700 dark:text-slate-200"
      :style="labelStyle"
    >
      {{ displayName }}
    </span>
  </div>
</template>
