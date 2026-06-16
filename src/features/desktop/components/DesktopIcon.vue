<script setup lang="ts">
import { computed, ref } from "vue";
import { File, FileText, Folder, Link } from "@lucide/vue";
import { openDesktopItem } from "../../../shared/api/desktop";
import type { DesktopItem, DesktopNameDisplayMode } from "../../../shared/types/desktop";
import { DESKTOP_ICON_VIEW, WINDOWS_SHORTCUT_BADGE } from "../config/desktopIcon";

/**
 * Box 内的图标是桌面文件映射视图，优先复用系统原生图标以保持拖入前后的视觉一致性。
 */
const props = defineProps<{
  doubleClickOpen?: boolean;
  item: DesktopItem;
  nameDisplayMode: DesktopNameDisplayMode;
  showLabel?: boolean;
  showShortcutArrow?: boolean;
}>();

const displayName = computed(() => formatDisplayName(props.item, props.nameDisplayMode));
const suppressNextClick = ref(false);

/**
 * 拖拽只传递文件路径，真实桌面文件仍然交给系统管理。
 */
function onDragStart(event: DragEvent, item: DesktopItem): void {
  suppressNextClick.value = true;
  event.dataTransfer?.setData("text/plain", item.path);
  event.dataTransfer?.setDragImage(event.currentTarget as Element, 36, 36);
}

/**
 * 拖拽结束后延迟一帧恢复点击，避免浏览器在 dragend 后补发 click 导致误打开。
 */
function onDragEnd(): void {
  window.setTimeout(() => {
    suppressNextClick.value = false;
  }, 0);
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
    class="flex min-w-0 select-none flex-col items-center justify-center gap-1.5 rounded-[8px] bg-transparent p-1.5 text-center text-slate-900 transition-colors hover:bg-white/55 active:bg-white/75 dark:text-white dark:hover:bg-white/10 dark:active:bg-white/20"
    :class="showLabel === false ? 'h-16' : 'h-[82px]'"
    draggable="true"
    type="button"
    :title="item.path"
    @dragend="onDragEnd"
    @dragstart="onDragStart($event, item)"
    @click="handleClick"
    @dblclick.prevent="handleDoubleClick"
  >
    <span
      class="relative grid size-11 place-items-center text-slate-700 dark:text-slate-100"
    >
      <img
        v-if="item.iconDataUrl"
        :alt="item.name"
        class="max-h-11 max-w-11 object-contain drop-shadow-[0_4px_8px_rgba(15,23,42,0.16)]"
        draggable="false"
        :src="item.iconDataUrl"
      />
      <Folder v-else-if="item.kind === 'folder'" :size="DESKTOP_ICON_VIEW.fallbackIconSize" />
      <Link v-else-if="item.kind === 'shortcut'" :size="DESKTOP_ICON_VIEW.fallbackIconSize" />
      <FileText v-else-if="item.extension" :size="DESKTOP_ICON_VIEW.fallbackIconSize" />
      <File v-else :size="DESKTOP_ICON_VIEW.fallbackIconSize" />
      <span
        v-if="item.kind === 'shortcut' && showShortcutArrow !== false"
        class="absolute bottom-0 left-0 grid size-4 place-items-center rounded-[2px] border border-white bg-white shadow-[0_1px_3px_rgba(15,23,42,0.22)]"
      >
        <svg aria-hidden="true" class="size-[15px]" :viewBox="WINDOWS_SHORTCUT_BADGE.viewBox">
          <path
            :d="WINDOWS_SHORTCUT_BADGE.backgroundPath"
            :fill="WINDOWS_SHORTCUT_BADGE.backgroundColor"
          />
          <path
            :d="WINDOWS_SHORTCUT_BADGE.arrowPath"
            :fill="WINDOWS_SHORTCUT_BADGE.arrowColor"
          />
        </svg>
      </span>
    </span>
    <span
      v-if="showLabel !== false"
      class="line-clamp-2 w-full overflow-hidden [overflow-wrap:anywhere] text-[10.5px] leading-[1.18] text-slate-700 dark:text-slate-200"
    >
      {{ displayName }}
    </span>
  </button>
</template>
