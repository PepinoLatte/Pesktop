<script setup lang="ts">
import type { ComponentPublicInstance, CSSProperties } from "vue";
import { MoreHorizontal } from "@lucide/vue";
import type { DesktopBox } from "@/entities/desktopBox/types";

/**
 * Box 标题栏负责展示名称、进入标题编辑、打开更多菜单和派发窗口拖动事件。
 */
defineProps<{
  box: DesktopBox;
  boxTitleAreaStyle: CSSProperties;
  boxTitleOrderClass: string;
  isEditingTitle: boolean;
  setTitleInputRef: (element: Element | ComponentPublicInstance | null) => void;
  titleDraft: string;
  titleFontSize?: number;
}>();

defineEmits<{
  cancelTitleEditing: [];
  commitTitleEditing: [];
  startDragging: [event: MouseEvent];
  startTitleEditing: [event: MouseEvent];
  titleMouseEnter: [];
  titleMouseLeave: [];
  toggleContextMenu: [];
  "update:titleDraft": [value: string];
}>();
</script>

<template>
  <header
    class="group/header relative z-30 flex h-10 shrink-0 select-none items-center justify-center px-3 transition-opacity duration-150 ease-out"
    :class="boxTitleOrderClass"
    :style="boxTitleAreaStyle"
    @mousedown.left="$emit('startDragging', $event)"
    @dblclick.stop.prevent
    @mouseenter="$emit('titleMouseEnter')"
    @mouseleave="$emit('titleMouseLeave')"
  >
    <input
      v-if="isEditingTitle"
      :ref="setTitleInputRef"
      :value="titleDraft"
      aria-label="编辑 Box 名称"
      class="h-7 w-[68%] max-w-[220px] rounded-[6px] bg-white/60 px-2 text-center font-semibold text-slate-900 outline-none transition-colors placeholder:text-slate-400 focus:bg-white/85 dark:bg-white/10 dark:text-white dark:focus:bg-white/15"
      :style="{ fontSize: `${titleFontSize || 13}px` }"
      maxlength="32"
      type="text"
      @blur="$emit('commitTitleEditing')"
      @input="$emit('update:titleDraft', ($event.target as HTMLInputElement).value)"
      @keydown.enter.prevent="$emit('commitTitleEditing')"
      @keydown.esc.prevent="$emit('cancelTitleEditing')"
      @mousedown.stop
      @dblclick.stop
    />
    <span
      v-else
      class="h-7 min-w-12 max-w-[68%] truncate text-center font-semibold leading-7 text-slate-900 dark:text-white"
      :style="{ fontSize: `${titleFontSize || 13}px` }"
      title="双击编辑 Box 名称"
      @dblclick="$emit('startTitleEditing', $event)"
      @mousedown.stop
    >
      {{ box.title }}
    </span>
    <button
      aria-label="打开 Box 菜单"
      class="absolute right-2 top-1/2 grid size-7 -translate-y-1/2 place-items-center rounded-[6px] text-slate-600 transition-all duration-200 hover:bg-white/50 hover:text-slate-950 dark:text-slate-300 dark:hover:bg-white/10 dark:hover:text-white opacity-0 group-hover/header:opacity-100 focus-visible:opacity-100"
      type="button"
      @click.stop="$emit('toggleContextMenu')"
      @mousedown.stop
    >
      <MoreHorizontal :size="18" />
    </button>
  </header>
</template>
