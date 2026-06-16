<script setup lang="ts">
import { File, FileText, Folder, Link } from "@lucide/vue";
import type { DesktopItem } from "../../../shared/types/desktop";

/**
 * Box 内的图标是桌面文件映射视图，点击和拖拽都不改变真实文件位置。
 */
defineProps<{
  item: DesktopItem;
  showLabel?: boolean;
}>();

/**
 * 拖拽只传递文件路径，真实桌面文件仍然交给系统管理。
 */
function onDragStart(event: DragEvent, item: DesktopItem): void {
  event.dataTransfer?.setData("text/plain", item.path);
  event.dataTransfer?.setDragImage(event.currentTarget as Element, 36, 36);
}
</script>

<template>
  <button
    class="flex min-w-0 select-none flex-col items-center justify-center gap-1.5 rounded-[10px] bg-transparent p-1.5 text-center text-slate-900 transition-colors hover:bg-white/60 active:bg-white/80 dark:text-white dark:hover:bg-white/10 dark:active:bg-white/20"
    :class="showLabel === false ? 'h-14' : 'h-[78px]'"
    draggable="true"
    type="button"
    :title="item.path"
    @dragstart="onDragStart($event, item)"
  >
    <span
      class="grid size-9 place-items-center rounded-[10px] border border-slate-200 bg-white text-slate-700 shadow-[0_8px_18px_rgba(15,23,42,0.08)] dark:border-white/10 dark:bg-white/10 dark:text-slate-100 dark:shadow-none"
    >
      <Folder v-if="item.kind === 'folder'" :size="20" />
      <Link v-else-if="item.kind === 'shortcut'" :size="20" />
      <FileText v-else-if="item.extension" :size="20" />
      <File v-else :size="20" />
    </span>
    <span
      v-if="showLabel !== false"
      class="line-clamp-2 w-full overflow-hidden [overflow-wrap:anywhere] text-[10.5px] leading-[1.18] text-slate-700 dark:text-slate-200"
    >
      {{ item.name }}
    </span>
  </button>
</template>
