<script setup lang="ts">
import { computed } from "vue";
import type { CSSProperties } from "vue";
import { File, FileText, Folder, Link } from "@lucide/vue";
import type { DesktopItem } from "../../../shared/types/desktop";
import { DESKTOP_ICON_VIEW, WINDOWS_SHORTCUT_BADGE } from "../config/desktopIcon";

/**
 * 桌面项目图标主体在 Box 和拖影窗口中复用，保证用户调节图标大小后两处视觉一致。
 */
const props = defineProps<{
  iconSize: number;
  item: DesktopItem;
  radiusSize: number;
  showShortcutArrow?: boolean;
}>();

const resolvedFallbackIconSize = computed(() =>
  Math.max(
    DESKTOP_ICON_VIEW.fallbackIconSize,
    Math.round(props.iconSize * DESKTOP_ICON_VIEW.fallbackIconScale),
  ),
);
const shortcutBadgeScale = computed(
  () => props.iconSize / WINDOWS_SHORTCUT_BADGE.referenceIconSize,
);
const iconFrameStyle = computed(
  () =>
    ({
      borderRadius: `${props.radiusSize}px`,
      height: `${props.iconSize}px`,
      width: `${props.iconSize}px`,
    }) as CSSProperties,
);
const iconImageStyle = computed(
  () =>
    ({
      maxHeight: `${props.iconSize}px`,
      maxWidth: `${props.iconSize}px`,
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
 * 快捷方式角标按参考图标尺寸等比缩放，避免拖影和 Box 图标出现比例差。
 */
function scaleShortcutBadgeValue(value: number): number {
  const scale = Math.min(Math.max(shortcutBadgeScale.value, 0.72), 1.25);

  return Math.round(value * scale);
}
</script>

<template>
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
</template>
