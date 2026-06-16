<script setup lang="ts">
import type { Component } from "vue";
import { Monitor, Moon, Sun } from "@lucide/vue";
import type { AppSettings, ThemeMode } from "../../../shared/types/desktop";

/**
 * 外观面板只暴露当前实现的主题和图标名称设置，避免旧视觉参数回流。
 */
const props = defineProps<{
  settings: AppSettings;
}>();

const emit = defineEmits<{
  itemLabelsChange: [value: boolean];
  themeChange: [theme: ThemeMode];
}>();

/**
 * 主题选项直接映射持久化枚举，避免 UI 和 Store 出现额外状态。
 */
const themeOptions: Array<{
  icon: Component;
  label: string;
  value: ThemeMode;
}> = [
  { icon: Sun, label: "浅色", value: "light" },
  { icon: Monitor, label: "跟随系统", value: "system" },
  { icon: Moon, label: "深色", value: "dark" },
];
</script>

<template>
  <section class="mx-auto w-full max-w-[780px] px-8 py-7">
    <div class="mb-6">
      <h1 class="text-[24px] font-semibold tracking-[0] text-[#17181c] dark:text-[#f4f4f5]">外观</h1>
      <p class="mt-1 text-[13px] text-[#6f7480] dark:text-[#a7abb5]">调整设置窗和 Box 的显示偏好。</p>
    </div>

    <div class="overflow-hidden rounded-[14px] border border-[#dfe2e8] bg-[#ffffff] shadow-[0_10px_28px_rgba(20,24,32,0.06)] dark:border-[#292c34] dark:bg-[#181a20] dark:shadow-none">
      <div class="grid min-h-[76px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">界面主题</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">用于设置页和 Box 窗口的明暗切换。</p>
        </div>

        <div class="flex rounded-[10px] bg-[#eceef3] p-1 dark:bg-[#242730]">
          <button
            v-for="option in themeOptions"
            :key="option.value"
            class="flex h-8 min-w-[92px] items-center justify-center gap-2 rounded-[8px] px-3 text-[12px] font-medium transition-colors"
            :class="
              props.settings.theme === option.value
                ? 'bg-[#ffffff] text-[#17181c] shadow-[0_4px_12px_rgba(20,24,32,0.10)] dark:bg-[#f4f4f5] dark:text-[#17181c]'
                : 'text-[#686e7b] hover:text-[#17181c] dark:text-[#a7abb5] dark:hover:text-[#f4f4f5]'
            "
            type="button"
            @click="emit('themeChange', option.value)"
          >
            <component :is="option.icon" :size="15" />
            <span>{{ option.label }}</span>
          </button>
        </div>
      </div>

      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />

      <div class="grid min-h-[72px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">显示项目名称</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">在 Box 图标下方展示文件名。</p>
        </div>

        <label class="relative block h-7 w-12">
          <input
            class="peer sr-only"
            :checked="props.settings.showItemLabels"
            type="checkbox"
            @change="emit('itemLabelsChange', ($event.target as HTMLInputElement).checked)"
          />
          <span
            class="absolute inset-0 rounded-full bg-[#c9cdd6] transition-colors peer-checked:bg-[#ff5c5c] dark:bg-[#3a3d46] dark:peer-checked:bg-[#ff6b6b]"
          />
          <span
            class="absolute left-1 top-1 size-5 rounded-full bg-white shadow-[0_2px_7px_rgba(20,24,32,0.25)] transition-transform peer-checked:translate-x-5"
          />
        </label>
      </div>
    </div>
  </section>
</template>
