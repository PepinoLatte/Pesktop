<script setup lang="ts">
import { Eye, FolderOpen, Lock, Unlock } from "@lucide/vue";
import { BOX_TITLE_OPACITY } from "@/entities/desktopBox/layout";
import type { DesktopBox } from "@/entities/desktopBox/types";
import { BOX_MENU_ACTION_ROW_CLASS } from "@/windows/boxContextMenu/model/menuOptions";

/**
 * 菜单操作行负责展示当前 Box 的快捷动作和实时状态，业务提交由外层 action composable 处理。
 */
defineProps<{
  box: DesktopBox;
}>();

const emit = defineEmits<{
  openFolder: [];
  toggleLocked: [];
  updateIdleOpacity: [nextOpacity: number];
}>();

/**
 * range 事件在组件内转换为 number，避免业务层依赖 DOM Event 结构。
 */
function handleIdleOpacityInput(event: Event): void {
  emit("updateIdleOpacity", Number((event.target as HTMLInputElement).value));
}
</script>

<template>
  <div class="grid gap-1 px-1">
    <button
      :class="BOX_MENU_ACTION_ROW_CLASS"
      type="button"
      @click="$emit('openFolder')"
    >
      <span class="grid size-7 shrink-0 place-items-center rounded-[7px] bg-[#e9edf5] text-slate-600 dark:bg-[#30343e] dark:text-slate-300">
        <FolderOpen :size="15" />
      </span>
      <span class="grid min-w-0 flex-1 gap-0.5">
        <span class="truncate text-[12px] font-semibold text-slate-800 dark:text-slate-100">打开真实文件夹</span>
        <span class="truncate text-[10px] leading-3 text-slate-500 dark:text-slate-400">在 Explorer 中查看内容</span>
      </span>
      <span class="rounded-[6px] bg-white px-1.5 py-0.5 text-[10px] font-medium text-slate-500 shadow-[inset_0_0_0_1px_rgba(148,163,184,0.35)] dark:bg-[#242730] dark:text-slate-300 dark:shadow-[inset_0_0_0_1px_rgba(100,116,139,0.35)]">打开</span>
    </button>
    <button
      :class="BOX_MENU_ACTION_ROW_CLASS"
      type="button"
      @click="$emit('toggleLocked')"
    >
      <span class="grid size-7 shrink-0 place-items-center rounded-[7px] bg-[#e9edf5] text-slate-600 dark:bg-[#30343e] dark:text-slate-300">
        <Lock v-if="box.locked" :size="15" />
        <Unlock v-else :size="15" />
      </span>
      <span class="grid min-w-0 flex-1 gap-0.5">
        <span class="truncate text-[12px] font-semibold text-slate-800 dark:text-slate-100">{{ box.locked ? "解除锁定" : "锁定布局" }}</span>
        <span class="truncate text-[10px] leading-3 text-slate-500 dark:text-slate-400">
          {{ box.locked ? "当前位置不可移动缩放" : "允许移动和缩放" }}
        </span>
      </span>
      <span
        class="rounded-[6px] px-1.5 py-0.5 text-[10px] font-medium"
        :class="box.locked ? 'bg-[#fff2d8] text-[#8a5a00] dark:bg-[#3a2f1f] dark:text-[#f5c76b]' : 'bg-[#e9f8ef] text-[#237447] dark:bg-[#1f3528] dark:text-[#7bd59f]'"
      >
        {{ box.locked ? "已锁定" : "可移动" }}
      </span>
    </button>
    <div class="rounded-[8px] px-2 py-2 transition-colors hover:bg-[#eef1f6] dark:hover:bg-[#2a2d36]">
      <div class="flex items-center gap-2">
        <span class="grid size-7 shrink-0 place-items-center rounded-[7px] bg-[#e9edf5] text-slate-600 dark:bg-[#30343e] dark:text-slate-300">
          <Eye :size="15" />
        </span>
        <span class="grid min-w-0 flex-1 gap-0.5">
          <span class="truncate text-[12px] font-semibold text-slate-800 dark:text-slate-100">闲置可见度</span>
          <span class="truncate text-[10px] leading-3 text-slate-500 dark:text-slate-400">鼠标离开后的 Box 透明度</span>
        </span>
        <span class="rounded-[6px] bg-white px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-slate-500 shadow-[inset_0_0_0_1px_rgba(148,163,184,0.35)] dark:bg-[#242730] dark:text-slate-300 dark:shadow-[inset_0_0_0_1px_rgba(100,116,139,0.35)]">{{ box.titleOpacity }}%</span>
      </div>
      <input
        class="mt-2 h-4 w-full accent-[#2f6bff]"
        :max="BOX_TITLE_OPACITY.max"
        :min="BOX_TITLE_OPACITY.min"
        :step="BOX_TITLE_OPACITY.step"
        type="range"
        :value="box.titleOpacity"
        @input="handleIdleOpacityInput"
      />
    </div>
  </div>
</template>
