<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import type { CSSProperties } from "vue";
import { emit } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalSize, PhysicalPosition } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  listenBoxFileDrag,
  type BoxFileDragPreviewOptions,
} from "@/shared/ipc/boxFileDrag";
import { formatDesktopItemDisplayName } from "@/entities/desktopItem/displayName";
import type { DesktopItem } from "@/entities/desktopItem/types";
import { DESKTOP_ICON_VIEW } from "@/windows/desktop/config/desktopIcon";
import DesktopIconGlyph from "@/windows/desktop/components/DesktopIconGlyph.vue";
import { DRAG_PREVIEW_WINDOW_READY_EVENT } from "./lifecycle";

/**
 * 拖影窗口相对鼠标保留偏移，既能看见拖动对象，又不遮挡用户判断插入位置。
 */
const DRAG_PREVIEW_OFFSET = {
  x: 6,
  y: 6,
} as const;

/**
 * 透明预览窗额外留出少量画布，避免 Shell 图标阴影和快捷方式角标被窗口边缘裁切。
 */
const DRAG_PREVIEW_WINDOW_MARGIN = 4;

/**
 * 动态窗口尺寸跟随 Box 图标配置，用户调大图标或文件名宽度时拖影也不会被固定窗口裁掉。
 */
interface DragPreviewWindowSize {
  height: number;
  width: number;
}

const currentWindow = getCurrentWindow();
const item = ref<DesktopItem | null>(null);
const itemCount = ref(0);
const preview = ref<BoxFileDragPreviewOptions | null>(null);
let unlistenDrag: UnlistenFn | null = null;
let lastPreviewWindowSize: DragPreviewWindowSize | null = null;

const displayName = computed(() => {
  const nextItem = item.value;
  const nextPreview = preview.value;
  if (!nextItem || !nextPreview) {
    return "";
  }

  return formatDesktopItemDisplayName(nextItem, nextPreview.nameDisplayMode);
});
const labelLineHeight = computed(() => {
  const labelTextSize = preview.value?.labelTextSize ?? 12;

  return Math.max(14, Math.ceil(labelTextSize * 1.32));
});
const labelBlockHeight = computed(() => labelLineHeight.value * 2 + 2);
const previewSurfaceStyle = computed(() => {
  const nextPreview = preview.value;
  if (!nextPreview) {
    return {} as CSSProperties;
  }

  return {
    borderRadius: `${nextPreview.radiusSize}px`,
    gap: nextPreview.showItemLabels ? `${DESKTOP_ICON_VIEW.labelGap}px` : "0px",
    padding: `${DESKTOP_ICON_VIEW.itemBlockPadding}px ${DESKTOP_ICON_VIEW.itemInlinePadding}px`,
    width: `${resolvePreviewItemWidth(nextPreview)}px`,
  } as CSSProperties;
});
const previewLabelStyle = computed(() => {
  const nextPreview = preview.value;
  if (!nextPreview) {
    return {} as CSSProperties;
  }

  return {
    WebkitBoxOrient: "vertical",
    WebkitLineClamp: "2",
    display: "-webkit-box",
    fontSize: `${nextPreview.labelTextSize}px`,
    lineHeight: `${labelLineHeight.value}px`,
    maxHeight: `${labelBlockHeight.value}px`,
    paddingBottom: "2px",
    textOverflow: "ellipsis",
    width: `${nextPreview.labelWidth}px`,
  } as CSSProperties;
});

onMounted(async () => {
  unlistenDrag = await listenBoxFileDrag(async ({ payload }) => {
    if (payload.phase === "cancel" || payload.phase === "drop") {
      item.value = null;
      itemCount.value = 0;
      preview.value = null;
      await currentWindow.hide();
      return;
    }

    item.value = payload.item;
    itemCount.value = payload.paths.length;
    preview.value = payload.preview;
    await resizePreviewWindow(payload.preview);
    await currentWindow.setPosition(
      new PhysicalPosition(
        Math.round(payload.screenX + DRAG_PREVIEW_OFFSET.x),
        Math.round(payload.screenY + DRAG_PREVIEW_OFFSET.y),
      ),
    );
    await currentWindow.show();
  });
  await emit(DRAG_PREVIEW_WINDOW_READY_EVENT);
});

onUnmounted(() => {
  unlistenDrag?.();
});

/**
 * 拖影图标宽度和 Box 内图标按钮保持同一计算方式，避免拖动中视觉密度突然变化。
 */
function resolvePreviewItemWidth(nextPreview: BoxFileDragPreviewOptions): number {
  return Math.max(
    nextPreview.labelWidth,
    nextPreview.iconSize + DESKTOP_ICON_VIEW.itemInlinePadding * 2,
  );
}

/**
 * 预览窗外壳按当前 Box 图标配置动态缩放，内部布局仍使用与 Box 图标一致的 padding/gap。
 */
function resolvePreviewWindowSize(nextPreview: BoxFileDragPreviewOptions): DragPreviewWindowSize {
  const labelHeight = nextPreview.showItemLabels
    ? DESKTOP_ICON_VIEW.labelGap + labelBlockHeight.value
    : 0;
  const contentHeight =
    DESKTOP_ICON_VIEW.itemBlockPadding * 2 + nextPreview.iconSize + labelHeight;

  return {
    height: Math.ceil(contentHeight + DRAG_PREVIEW_WINDOW_MARGIN * 2),
    width: Math.ceil(resolvePreviewItemWidth(nextPreview) + DRAG_PREVIEW_WINDOW_MARGIN * 2),
  };
}

/**
 * 仅在尺寸变化时调用原生窗口调整，减少拖动中不必要的窗口重排。
 */
async function resizePreviewWindow(nextPreview: BoxFileDragPreviewOptions): Promise<void> {
  const nextSize = resolvePreviewWindowSize(nextPreview);
  if (
    lastPreviewWindowSize &&
    lastPreviewWindowSize.height === nextSize.height &&
    lastPreviewWindowSize.width === nextSize.width
  ) {
    return;
  }

  lastPreviewWindowSize = nextSize;
  await currentWindow.setSize(new LogicalSize(nextSize.width, nextSize.height));
}
</script>

<template>
  <main class="grid h-screen w-screen place-items-start overflow-hidden bg-transparent p-1">
    <div
      v-if="item && preview"
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
  </main>
</template>
