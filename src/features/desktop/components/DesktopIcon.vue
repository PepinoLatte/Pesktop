<script setup lang="ts">
import { computed, onUnmounted, ref } from "vue";
import type { CSSProperties } from "vue";
import { openDesktopItem } from "../../../shared/api/desktop";
import { DEFAULT_APP_SETTINGS } from "../../../shared/config/appSettings";
import type { DesktopItem, DesktopNameDisplayMode } from "../../../shared/types/desktop";
import { DESKTOP_ICON_VIEW } from "../config/desktopIcon";
import { formatDesktopItemDisplayName } from "../utils/desktopItemName";
import DesktopIconGlyph from "./DesktopIconGlyph.vue";

/**
 * Box 内的图标是桌面文件映射视图，优先复用系统原生图标以保持拖入前后的视觉一致性。
 */
const props = defineProps<{
  doubleClickOpen?: boolean;
  /**
   * Box 图标拖拽期间关闭其他图标的 pointer 命中，避免 hover 背景和拖拽排序反馈互相干扰。
   */
  dragInteractionDisabled?: boolean;
  /**
   * 当前正在被拖动的图标保留原位置，但不显示 hover 背景，避免和拖影窗口形成双重高亮。
   */
  dragging?: boolean;
  iconSize?: number;
  item: DesktopItem;
  labelTextSize?: number;
  labelWidth?: number;
  nameDisplayMode: DesktopNameDisplayMode;
  radiusSize?: number;
  showLabel?: boolean;
  showShortcutArrow?: boolean;
}>();

const emit = defineEmits<{
  boxPointerDragEnd: [event: PointerEvent, itemPath: string];
  boxPointerDragStart: [event: PointerEvent, itemPath: string];
  nativeContextMenu: [event: MouseEvent, item: DesktopItem];
}>();

/**
 * pointer 拖拽状态只用于 Box 内排序，避免浏览器原生 DnD 在透明窗口里显示禁用光标。
 */
interface PointerDragState {
  dragging: boolean;
  itemPath: string;
  startX: number;
  startY: number;
}

const displayName = computed(() =>
  formatDesktopItemDisplayName(props.item, props.nameDisplayMode),
);
const resolvedIconSize = computed(() => props.iconSize ?? DEFAULT_APP_SETTINGS.boxIconSize);
const resolvedLabelTextSize = computed(
  () => props.labelTextSize ?? DEFAULT_APP_SETTINGS.boxLabelTextSize,
);
const resolvedLabelWidth = computed(() => props.labelWidth ?? DEFAULT_APP_SETTINGS.boxFilenameWidth);
const resolvedItemWidth = computed(() =>
  Math.max(
    resolvedLabelWidth.value,
    resolvedIconSize.value + DESKTOP_ICON_VIEW.itemInlinePadding * 2,
  ),
);
const resolvedRadius = computed(() => props.radiusSize ?? DEFAULT_APP_SETTINGS.boxCornerRadius);
/**
 * Windows 桌面标签会给字母下探部位留空间；这里多留 2px，避免 p/g/y 被两行截断裁掉。
 */
const labelLineHeight = computed(() => Math.max(14, Math.ceil(resolvedLabelTextSize.value * 1.32)));
const labelBlockHeight = computed(() => labelLineHeight.value * 2 + 2);
const suppressNextClick = ref(false);
const isPointerDragging = ref(false);
let pointerDragState: PointerDragState | null = null;

onUnmounted(() => {
  cleanupPointerDrag();
});

const iconButtonStyle = computed(
  () =>
    ({
      borderRadius: `${resolvedRadius.value}px`,
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
      fontSize: `${resolvedLabelTextSize.value}px`,
      lineHeight: `${labelLineHeight.value}px`,
      maxHeight: `${labelBlockHeight.value}px`,
      paddingBottom: "2px",
      textOverflow: "ellipsis",
      width: `${resolvedLabelWidth.value}px`,
    }) as CSSProperties,
);
/**
 * pointer 按下只记录候选拖拽，移动距离超过阈值后才进入排序拖动状态。
 */
function onPointerDown(event: PointerEvent, item: DesktopItem): void {
  if (event.button !== 0 || event.detail > 1) {
    return;
  }

  pointerDragState = {
    dragging: false,
    itemPath: item.path,
    startX: event.clientX,
    startY: event.clientY,
  };
  window.addEventListener("pointermove", onPointerMove, { capture: true });
  window.addEventListener("pointerup", onPointerRelease, { capture: true, once: true });
  window.addEventListener("pointercancel", onPointerRelease, { capture: true, once: true });
  window.addEventListener("blur", onPointerWindowBlur, { capture: true, once: true });
}

/**
 * pointer 移动只负责跨过阈值后启动全局拖拽；插入线统一由窗口级拖拽事件计算，避免本地 hover 抖动。
 */
function onPointerMove(event: PointerEvent): void {
  const dragState = pointerDragState;
  if (!dragState) {
    return;
  }

  if (!dragState.dragging && !hasPointerExceededDragThreshold(event, dragState)) {
    return;
  }

  if (!dragState.dragging) {
    dragState.dragging = true;
    isPointerDragging.value = true;
    suppressNextClick.value = true;
    emit("boxPointerDragStart", event, dragState.itemPath);
  }

  event.preventDefault();
}

/**
 * pointerup 才主动结束拖拽；pointercancel 只清理本地监听，真实释放由全局轮询兜底处理。
 */
function onPointerRelease(event: PointerEvent): void {
  const dragState = pointerDragState;
  const shouldEmitDragEnd = dragState?.dragging === true && event.type === "pointerup";

  cleanupPointerDrag();
  if (dragState?.dragging) {
    if (shouldEmitDragEnd) {
      emit("boxPointerDragEnd", event, dragState.itemPath);
    }
    window.setTimeout(() => {
      suppressNextClick.value = false;
    }, 0);
  }
}

/**
 * 清理 pointer 拖拽监听，避免多次按下后产生重复 move/up 回调。
 */
function cleanupPointerDrag(): void {
  window.removeEventListener("pointermove", onPointerMove, { capture: true });
  window.removeEventListener("pointerup", onPointerRelease, { capture: true });
  window.removeEventListener("pointercancel", onPointerRelease, { capture: true });
  window.removeEventListener("blur", onPointerWindowBlur, { capture: true });
  pointerDragState = null;
  isPointerDragging.value = false;
}

/**
 * 拖出窗口时全局拖拽轮询仍会继续，组件这里只清理本地 pointer 状态和点击抑制。
 */
function onPointerWindowBlur(): void {
  cleanupPointerDrag();
  window.setTimeout(() => {
    suppressNextClick.value = false;
  }, 0);
}

/**
 * 拖拽阈值使用欧氏距离，斜向移动和横向移动都有一致的触发手感。
 */
function hasPointerExceededDragThreshold(
  event: PointerEvent,
  dragState: PointerDragState,
): boolean {
  const deltaX = event.clientX - dragState.startX;
  const deltaY = event.clientY - dragState.startY;

  return Math.hypot(deltaX, deltaY) >= DESKTOP_ICON_VIEW.dragStartThreshold;
}

/**
 * 右键菜单由父级桥接到 Windows Shell，组件自身不展示浏览器菜单。
 */
function onContextMenu(event: MouseEvent): void {
  emit("nativeContextMenu", event, props.item);
}

/**
 * 点击 Box 内图标时交给系统默认程序打开，保持与 Windows 桌面双击一致。
 */
async function openItem(): Promise<void> {
  if (suppressNextClick.value) {
    return;
  }

  await openDesktopItem(props.item.path);
}

/**
 * 默认使用双击打开，保留单击选择/拖动的空间；用户关闭该设置后单击直接打开。
 */
function handleClick(event: MouseEvent): void {
  if (props.doubleClickOpen === false && event.detail === DESKTOP_ICON_VIEW.openClickDetail) {
    void openItem();
  }
}

/**
 * 双击打开时只在双击事件中触发，避免第一次单击误启动文件。
 */
function handleDoubleClick(): void {
  if (props.doubleClickOpen !== false) {
    void openItem();
  }
}

</script>

<template>
  <button
    class="dasktop-icon-button relative flex min-w-0 select-none flex-col items-center justify-start self-start bg-transparent text-center text-slate-900 transition-colors hover:bg-white/55 active:bg-white/75 dark:text-white dark:hover:bg-white/10 dark:active:bg-white/20"
    :class="[
      dragging || isPointerDragging ? 'dasktop-icon-button--dragging opacity-60' : '',
      dragInteractionDisabled ? 'dasktop-icon-button--drag-interaction-disabled' : '',
    ]"
    :data-box-item-path="item.path"
    :style="iconButtonStyle"
    type="button"
    :title="item.path"
    @contextmenu.prevent="onContextMenu"
    @click="handleClick"
    @dblclick.prevent="handleDoubleClick"
    @pointerdown="onPointerDown($event, item)"
  >
    <DesktopIconGlyph
      :icon-size="resolvedIconSize"
      :item="item"
      :radius-size="resolvedRadius"
      :show-shortcut-arrow="showShortcutArrow"
    />
    <span
      v-if="showLabel !== false"
      class="overflow-hidden [overflow-wrap:anywhere] text-slate-700 dark:text-slate-200"
      :style="labelStyle"
    >
      {{ displayName }}
    </span>
  </button>
</template>
