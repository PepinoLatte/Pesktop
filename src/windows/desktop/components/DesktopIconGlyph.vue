<script setup lang="ts">
import { computed } from "vue";
import type { CSSProperties } from "vue";
import { File, FileText, Folder, Link, Monitor } from "@lucide/vue";
import type { DesktopItem } from "@/entities/desktopItem/types";
import { DESKTOP_ICON_VIEW } from "../config/desktopIcon";

/**
 * 快捷方式角标按白底蓝箭头绘制，让自绘图标保留 Windows 快捷方式的核心语义。
 */
const WINDOWS_SHORTCUT_BADGE = {
  arrowPaths: ["M4 12L12 4", "M7 4H12V9"],
  offset: -4,
  overlayRadius: 6,
  overlaySize: 22,
  referenceIconSize: 64,
  shadow: "0 1px 4px rgba(0,0,0,.2)",
  strokeColor: "#0964d8",
  strokeWidth: 2.3,
  svgOffsetX: -1,
  svgOffsetY: -0.5,
  svgSize: 18,
  viewBox: "0 0 16 16",
} as const;

const props = defineProps<{
  iconSize: number;
  item: DesktopItem;
  radiusSize: number;
  showShortcutArrow?: boolean;
}>();

const resolvedFallbackIconSize = computed(() =>
  Math.max(
    DESKTOP_ICON_VIEW.fallbackIconSize,
    Math.round(props.iconSize * 0.88),
  ),
);
const shortcutBadgeScale = computed(
  () => props.iconSize / WINDOWS_SHORTCUT_BADGE.referenceIconSize,
);
const iconInnerContainerStyle = computed(
  () =>
    ({
      borderRadius: `${props.radiusSize}px`,
      clipPath: props.radiusSize > 0 ? `inset(0 round ${props.radiusSize}px)` : undefined,
      height: `${props.iconSize}px`,
      maxHeight: `${props.iconSize}px`,
      maxWidth: `${props.iconSize}px`,
      width: `${props.iconSize}px`,
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
 * 角标随图标尺寸等比缩放，避免大/小图标时快捷方式标识比例失衡。
 */
function scaleShortcutBadgeValue(value: number): number {
  const scale = Math.min(Math.max(shortcutBadgeScale.value, 0.72), 1.25);

  return Math.round(value * scale);
}
</script>

<template>
  <div
    class="relative grid shrink-0 place-items-center select-none text-slate-700 dark:text-slate-100"
    :style="{
      height: `${iconSize}px`,
      width: `${iconSize}px`,
    }"
  >
    <!-- 图标主体容器：严格统一宽高、居中定位并通过 clipPath 和 overflow 实现像素级圆角裁切 -->
    <div
      class="relative flex items-center justify-center overflow-hidden transition-all duration-150"
      :style="iconInnerContainerStyle"
    >
      <img
        v-if="item.iconDataUrl"
        :alt="item.name"
        class="h-full w-full object-contain pointer-events-none select-none transition-transform duration-150 drop-shadow-[0_2px_4px_rgba(15,23,42,0.12)]"
        draggable="false"
        :src="item.iconDataUrl"
      />
      <Folder v-else-if="item.kind === 'folder'" :size="resolvedFallbackIconSize" />
      <Monitor v-else-if="item.kind === 'shell'" :size="resolvedFallbackIconSize" />
      <Link v-else-if="item.kind === 'shortcut'" :size="resolvedFallbackIconSize" />
      <FileText v-else-if="item.extension" :size="resolvedFallbackIconSize" />
      <File v-else :size="resolvedFallbackIconSize" />
    </div>

    <!-- 快捷方式小角标：贴在图标外层左下，不受内部圆角裁切影响 -->
    <span
      v-if="item.kind === 'shortcut' && showShortcutArrow !== false"
      class="pointer-events-none absolute z-20 grid place-items-center"
      :style="shortcutBadgeStyle"
    >
      <svg
        aria-hidden="true"
        fill="none"
        :stroke="WINDOWS_SHORTCUT_BADGE.strokeColor"
        stroke-linecap="round"
        stroke-linejoin="round"
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
  </div>
</template>
