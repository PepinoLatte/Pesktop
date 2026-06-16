<script setup lang="ts">
import { computed, onUnmounted, ref } from "vue";
import type { CSSProperties } from "vue";
import { File, FileText, Folder, Link } from "@lucide/vue";
import { openDesktopItem } from "../../../shared/api/desktop";
import { DEFAULT_APP_SETTINGS } from "../../../shared/config/appSettings";
import type {
  DesktopBoxItemDropPlacement,
  DesktopItem,
  DesktopNameDisplayMode,
} from "../../../shared/types/desktop";
import { DESKTOP_ICON_VIEW, WINDOWS_SHORTCUT_BADGE } from "../config/desktopIcon";

/**
 * Box 内的图标是桌面文件映射视图，优先复用系统原生图标以保持拖入前后的视觉一致性。
 */
const props = defineProps<{
  doubleClickOpen?: boolean;
  iconSize?: number;
  item: DesktopItem;
  labelTextSize?: number;
  labelWidth?: number;
  dragInsertPosition?: DesktopBoxItemDropPlacement | null;
  nameDisplayMode: DesktopNameDisplayMode;
  radiusSize?: number;
  showLabel?: boolean;
  showShortcutArrow?: boolean;
}>();

const emit = defineEmits<{
  boxPointerDragEnd: [event: PointerEvent, itemPath: string];
  boxPointerDragMove: [event: PointerEvent, itemPath: string];
  boxPointerDragStart: [itemPath: string];
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

const displayName = computed(() => formatDisplayName(props.item, props.nameDisplayMode));
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
const resolvedFallbackIconSize = computed(() =>
  Math.max(
    DESKTOP_ICON_VIEW.fallbackIconSize,
    Math.round(resolvedIconSize.value * DESKTOP_ICON_VIEW.fallbackIconScale),
  ),
);
/**
 * Windows 桌面标签会给字母下探部位留空间；这里多留 2px，避免 p/g/y 被两行截断裁掉。
 */
const labelLineHeight = computed(() => Math.max(14, Math.ceil(resolvedLabelTextSize.value * 1.32)));
const labelBlockHeight = computed(() => labelLineHeight.value * 2 + 2);
const shortcutBadgeScale = computed(() =>
  resolvedIconSize.value / WINDOWS_SHORTCUT_BADGE.referenceIconSize,
);
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
const iconFrameStyle = computed(
  () =>
    ({
      borderRadius: `${resolvedRadius.value}px`,
      height: `${resolvedIconSize.value}px`,
      width: `${resolvedIconSize.value}px`,
    }) as CSSProperties,
);
const iconImageStyle = computed(
  () =>
    ({
      maxHeight: `${resolvedIconSize.value}px`,
      maxWidth: `${resolvedIconSize.value}px`,
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
const shortcutBadgeStyle = computed(
  () =>
    ({
      background: "#ffffff",
      borderRadius: `${scaleShortcutBadgeValue(WINDOWS_SHORTCUT_BADGE.overlayRadius)}px`,
      bottom: `${scaleShortcutBadgeValue(WINDOWS_SHORTCUT_BADGE.offset)}px`,
      boxShadow: WINDOWS_SHORTCUT_BADGE.shadow,
      height: `${scaleShortcutBadgeValue(WINDOWS_SHORTCUT_BADGE.overlaySize)}px`,
      left: `${scaleShortcutBadgeValue(WINDOWS_SHORTCUT_BADGE.offset)}px`,
      width: `${scaleShortcutBadgeValue(WINDOWS_SHORTCUT_BADGE.overlaySize)}px`,
    }) as CSSProperties,
);
const shortcutBadgeSvgStyle = computed(
  () =>
    ({
      height: `${scaleShortcutBadgeValue(WINDOWS_SHORTCUT_BADGE.svgSize)}px`,
      transform: `translate(${scaleShortcutBadgeValue(WINDOWS_SHORTCUT_BADGE.svgOffsetX)}px, ${scaleShortcutBadgeValue(WINDOWS_SHORTCUT_BADGE.svgOffsetY)}px)`,
      width: `${scaleShortcutBadgeValue(WINDOWS_SHORTCUT_BADGE.svgSize)}px`,
    }) as CSSProperties,
);

/**
 * 快捷方式角标按参考 64px 图标等比缩放，同时给极小图标保留可辨识的最小尺寸。
 */
function scaleShortcutBadgeValue(value: number): number {
  const scale = Math.min(Math.max(shortcutBadgeScale.value, 0.72), 1.25);

  return Math.round(value * scale);
}

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
 * pointer 移动时实时通知父级计算插入线；未达到阈值前不影响正常点击打开。
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
    emit("boxPointerDragStart", dragState.itemPath);
  }

  event.preventDefault();
  emit("boxPointerDragMove", event, dragState.itemPath);
}

/**
 * pointer 释放时提交排序或拖出删除，并在下一帧恢复点击能力。
 */
function onPointerRelease(event: PointerEvent): void {
  const dragState = pointerDragState;
  const shouldEmitDragEnd = dragState?.dragging === true;

  cleanupPointerDrag();
  if (dragState && shouldEmitDragEnd) {
    emit("boxPointerDragEnd", event, dragState.itemPath);
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
 * 拖出窗口时父级会处理删除映射，组件这里只清理本地 pointer 状态和点击抑制。
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

/**
 * 文件名显示只处理末尾扩展名，不改真实文件名，也不影响路径映射。
 */
function formatDisplayName(item: DesktopItem, mode: DesktopNameDisplayMode): string {
  if (mode === "full" || !item.extension) {
    return item.name;
  }

  if (mode === "hideShortcutExtension" && item.kind !== "shortcut") {
    return item.name;
  }

  const extensionSuffix = `.${item.extension}`;
  return item.name.toLowerCase().endsWith(extensionSuffix.toLowerCase())
    ? item.name.slice(0, -extensionSuffix.length)
    : item.name;
}
</script>

<template>
  <button
    class="dasktop-icon-button relative flex min-w-0 select-none flex-col items-center justify-start self-start bg-transparent text-center text-slate-900 transition-colors hover:bg-white/55 active:bg-white/75 dark:text-white dark:hover:bg-white/10 dark:active:bg-white/20"
    :class="isPointerDragging ? 'opacity-60' : ''"
    :data-box-item-path="item.path"
    :style="iconButtonStyle"
    type="button"
    :title="item.path"
    @contextmenu.prevent="onContextMenu"
    @click="handleClick"
    @dblclick.prevent="handleDoubleClick"
    @pointerdown="onPointerDown($event, item)"
  >
    <span
      v-if="dragInsertPosition"
      aria-hidden="true"
      class="pointer-events-none absolute bottom-1 top-1 z-10 w-[2px] rounded-full bg-[#2f6bff] shadow-[0_0_0_1px_rgba(255,255,255,0.86),0_0_10px_rgba(47,107,255,0.48)] dark:shadow-[0_0_0_1px_rgba(15,23,42,0.9),0_0_10px_rgba(83,149,255,0.58)]"
      :class="dragInsertPosition === 'before' ? '-left-1.5' : '-right-1.5'"
    />
    <span
      class="relative grid place-items-center text-slate-700 dark:text-slate-100"
      :style="iconFrameStyle"
    >
      <img
        v-if="item.iconDataUrl"
        :alt="item.name"
        class="object-contain drop-shadow-[0_4px_8px_rgba(15,23,42,0.16)]"
        draggable="false"
        :src="item.iconDataUrl"
        :style="iconImageStyle"
      />
      <Folder v-else-if="item.kind === 'folder'" :size="resolvedFallbackIconSize" />
      <Link v-else-if="item.kind === 'shortcut'" :size="resolvedFallbackIconSize" />
      <FileText v-else-if="item.extension" :size="resolvedFallbackIconSize" />
      <File v-else :size="resolvedFallbackIconSize" />
      <span
        v-if="item.kind === 'shortcut' && showShortcutArrow !== false"
        class="absolute grid place-items-center"
        :style="shortcutBadgeStyle"
      >
        <svg
          aria-hidden="true"
          fill="none"
          :stroke="WINDOWS_SHORTCUT_BADGE.strokeColor"
          :stroke-linecap="'round'"
          :stroke-linejoin="'round'"
          :stroke-width="WINDOWS_SHORTCUT_BADGE.strokeWidth"
          :style="shortcutBadgeSvgStyle"
          :viewBox="WINDOWS_SHORTCUT_BADGE.viewBox"
        >
          <path
            v-for="path in WINDOWS_SHORTCUT_BADGE.arrowPaths"
            :key="path"
            :d="path"
          />
        </svg>
      </span>
    </span>
    <span
      v-if="showLabel !== false"
      class="overflow-hidden [overflow-wrap:anywhere] text-slate-700 dark:text-slate-200"
      :style="labelStyle"
    >
      {{ displayName }}
    </span>
  </button>
</template>
