<script setup lang="ts">
import { RotateCcw } from "@lucide/vue";
import {
  APP_SETTING_NUMBER_LIMITS,
  DEFAULT_APP_SETTINGS,
  type AppSettingNumberKey,
} from "@/entities/appSettings/defaults";
import type { AppSettings } from "@/entities/appSettings/types";
import type {
  DesktopItem,
  DesktopNameDisplayMode,
} from "@/entities/desktopItem/types";
import SegmentedControl from "@/shared/ui/SegmentedControl.vue";

/**
 * Box 外观数值项限定在当前设置页展示的字段内，吸附距离继续归属窗口行为配置。
 */
type BoxVisualSettingKey = Exclude<AppSettingNumberKey, "snapThreshold">;

/**
 * 文件名样式选项按信息量递减排列，让用户从完整到简洁逐步选择。
 */
const NAME_DISPLAY_SEGMENT_OPTIONS: Array<{
  label: string;
  value: DesktopNameDisplayMode;
}> = [
  { label: "完整名称", value: "full" },
  { label: "隐藏.lnk", value: "hideShortcutExtension" },
  { label: "简洁名称", value: "hideAllExtensions" },
];

/**
 * 吸附距离滑块范围直接跟随应用设置范围，避免模板限制和持久化校验脱节。
 */
const SNAP_THRESHOLD_INPUT = {
  ...APP_SETTING_NUMBER_LIMITS.snapThreshold,
} as const;

/**
 * Box 外观调节项是当前面板的展示描述表，内联后控件文案与渲染位置保持内聚。
 */
const BOX_VISUAL_SETTING_CONTROLS: Array<{
  description: string;
  key: BoxVisualSettingKey;
  label: string;
  max: number;
  min: number;
  step: number;
  unit: string;
}> = [
  {
    key: "boxBackgroundOpacity",
    label: "背景透明度",
    description: "控制 Box 背景与桌面壁纸的融合程度。",
    ...APP_SETTING_NUMBER_LIMITS.boxBackgroundOpacity,
  },
  {
    key: "boxCollapseAnimationMs",
    label: "收缩动画速度",
    description: "调整 Box 自动收起和展开的动画时长。",
    ...APP_SETTING_NUMBER_LIMITS.boxCollapseAnimationMs,
  },
  {
    key: "boxIconSize",
    label: "图标大小",
    description: "调整 Box 内项目图标的显示尺寸。",
    ...APP_SETTING_NUMBER_LIMITS.boxIconSize,
  },
  {
    key: "boxLabelTextSize",
    label: "文字大小",
    description: "调整文件名文字大小，适配不同分辨率。",
    ...APP_SETTING_NUMBER_LIMITS.boxLabelTextSize,
  },
  {
    key: "boxIconGapX",
    label: "横向间距",
    description: "调整图标列之间的水平距离。",
    ...APP_SETTING_NUMBER_LIMITS.boxIconGapX,
  },
  {
    key: "boxIconGapY",
    label: "纵向间距",
    description: "调整图标行之间的垂直距离。",
    ...APP_SETTING_NUMBER_LIMITS.boxIconGapY,
  },
  {
    key: "boxFilenameWidth",
    label: "文件名宽度",
    description: "控制文件名换行宽度，长名称会在此范围内显示。",
    ...APP_SETTING_NUMBER_LIMITS.boxFilenameWidth,
  },
  {
    key: "boxCornerRadius",
    label: "圆角大小",
    description: "调整 Box 面板和图标悬停区域的圆角。",
    ...APP_SETTING_NUMBER_LIMITS.boxCornerRadius,
  },
];

/**
 * Box 显示面板维护所有 Box 共享的图标呈现规则，避免同一项目在不同窗口表现不一致。
 */
const props = defineProps<{
  desktopItems: DesktopItem[];
  /**
   * 设置页父级统一下发内容宽度，避免各面板各自维护页面密度。
   */
  panelWidth: string;
  settings: AppSettings;
}>();

const emit = defineEmits<{
  boxVisualSettingChange: [key: BoxVisualSettingKey, value: number];
  doubleClickOpenItemsChange: [value: boolean];
  itemLabelsChange: [value: boolean];
  nameDisplayModeChange: [value: DesktopNameDisplayMode];
  nativeDesktopIconIgnorePathsChange: [paths: string[]];
  nativeDesktopIconsHiddenChange: [value: boolean];
  showShortcutArrowChange: [value: boolean];
  snapThresholdChange: [value: number];
  snapToEdgesChange: [value: boolean];
}>();

/**
 * Box 外观滑块统一通过数值设置事件保存，避免面板直接了解数据库细节。
 */
function emitBoxVisualSettingChange(key: BoxVisualSettingKey, event: Event): void {
  emit("boxVisualSettingChange", key, Number((event.target as HTMLInputElement).value));
}

/**
 * 单行重置只恢复当前设置项，避免用户调整多个视觉参数后被整体覆盖。
 */
function resetBoxVisualSetting(key: BoxVisualSettingKey): void {
  emit("boxVisualSettingChange", key, DEFAULT_APP_SETTINGS[key]);
}

/**
 * 吸附距离沿用窗口行为设置，但入口已经合并到当前设置页。
 */
function emitSnapThresholdChange(event: Event): void {
  emit("snapThresholdChange", Number((event.target as HTMLInputElement).value));
}

/**
 * 吸附距离重置到默认阈值，方便用户调乱后快速回到稳定拖动手感。
 */
function resetSnapThreshold(): void {
  emit("snapThresholdChange", DEFAULT_APP_SETTINGS.snapThreshold);
}

/**
 * 忽略列表用真实路径做稳定键，避免重名桌面文件勾选状态互相覆盖。
 */
function isNativeDesktopIconIgnored(itemPath: string): boolean {
  return props.settings.nativeDesktopIconIgnorePaths.includes(itemPath);
}

/**
 * 切换单个忽略项时重新生成数组，保持 Pinia 和 SQLite 都能识别到设置变化。
 */
function toggleNativeDesktopIconIgnorePath(itemPath: string, checked: boolean): void {
  const nextPaths = checked
    ? [...props.settings.nativeDesktopIconIgnorePaths, itemPath]
    : props.settings.nativeDesktopIconIgnorePaths.filter((path) => path !== itemPath);

  emit("nativeDesktopIconIgnorePathsChange", Array.from(new Set(nextPaths)));
}
</script>

<template>
  <section class="mx-auto w-full px-8 py-7" :class="panelWidth">
    <div class="mb-6">
      <h1 class="text-[24px] font-semibold tracking-[0] text-[#17181c] dark:text-[#f4f4f5]">设置</h1>
      <p class="mt-1 text-[13px] text-[#6f7480] dark:text-[#a7abb5]">调整 Box 内项目的展示和打开方式。</p>
    </div>

    <div class="mt-4 overflow-hidden rounded-[8px] border border-[#dfe2e8] bg-[#ffffff] shadow-[0_10px_28px_rgba(20,24,32,0.06)] dark:border-[#292c34] dark:bg-[#181a20] dark:shadow-none">
      <div
        v-for="(control, index) in BOX_VISUAL_SETTING_CONTROLS"
        :key="control.key"
      >
        <div class="grid min-h-[84px] grid-cols-[1fr_300px] items-center gap-6 px-5">
          <div class="min-w-0 pr-4">
            <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">{{ control.label }}</h2>
            <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">{{ control.description }}</p>
          </div>

          <div class="flex items-center gap-3">
            <input
              class="h-1.5 w-full cursor-pointer appearance-none rounded-full bg-[#d8dbe3] accent-[#ff5c5c] dark:bg-[#333640] dark:accent-[#ff6b6b]"
              :max="control.max"
              :min="control.min"
              :step="control.step"
              type="range"
              :value="props.settings[control.key]"
              @input="emitBoxVisualSettingChange(control.key, $event)"
            />
            <span class="w-[70px] shrink-0 whitespace-nowrap text-right text-[13px] font-medium text-[#555b66] dark:text-[#c7cad1]">
              {{ props.settings[control.key] }} {{ control.unit }}
            </span>
            <button
              :aria-label="`重置${control.label}`"
              class="grid size-8 shrink-0 place-items-center rounded-[6px] text-[#68707d] transition-colors hover:bg-[#eef0f4] hover:text-[#17181c] disabled:cursor-default disabled:opacity-35 disabled:hover:bg-transparent disabled:hover:text-[#68707d] dark:text-[#a7abb5] dark:hover:bg-[#242730] dark:hover:text-[#f4f4f5] dark:disabled:hover:bg-transparent dark:disabled:hover:text-[#a7abb5]"
              :disabled="props.settings[control.key] === DEFAULT_APP_SETTINGS[control.key]"
              :title="`重置${control.label}`"
              type="button"
              @click="resetBoxVisualSetting(control.key)"
            >
              <RotateCcw :size="15" />
            </button>
          </div>
        </div>

        <div
          v-if="index < BOX_VISUAL_SETTING_CONTROLS.length - 1"
          class="h-px bg-[#e7e9ee] dark:bg-[#292c34]"
        />
      </div>
    </div>

    <div class="overflow-hidden rounded-[8px] border border-[#dfe2e8] bg-[#ffffff] shadow-[0_10px_28px_rgba(20,24,32,0.06)] dark:border-[#292c34] dark:bg-[#181a20] dark:shadow-none">
      <div class="grid min-h-[72px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">快捷方式标记</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">在快捷方式图标左下角显示 Windows 风格箭头。</p>
        </div>

        <label class="relative block h-7 w-12">
          <input
            class="peer sr-only"
            :checked="props.settings.showShortcutArrow"
            type="checkbox"
            @change="emit('showShortcutArrowChange', ($event.target as HTMLInputElement).checked)"
          />
          <span
            class="absolute inset-0 rounded-full bg-[#c9cdd6] transition-colors peer-checked:bg-[#ff5c5c] dark:bg-[#3a3d46] dark:peer-checked:bg-[#ff6b6b]"
          />
          <span
            class="absolute left-1 top-1 size-5 rounded-full bg-white shadow-[0_2px_7px_rgba(20,24,32,0.25)] transition-transform peer-checked:translate-x-5"
          />
        </label>
      </div>

      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />

      <div class="grid min-h-[76px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">文件名样式</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">控制 Box 中项目名称的后缀显示策略。</p>
        </div>

        <SegmentedControl
          :model-value="props.settings.nameDisplayMode"
          :options="NAME_DISPLAY_SEGMENT_OPTIONS"
          @change="emit('nameDisplayModeChange', $event)"
        />
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

      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />

      <div class="grid min-h-[72px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">双击打开项目</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">开启后需要双击图标才会打开文件或文件夹。</p>
        </div>

        <label class="relative block h-7 w-12">
          <input
            class="peer sr-only"
            :checked="props.settings.doubleClickOpenItems"
            type="checkbox"
            @change="emit('doubleClickOpenItemsChange', ($event.target as HTMLInputElement).checked)"
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

    <div class="mt-4 overflow-hidden rounded-[8px] border border-[#dfe2e8] bg-[#ffffff] shadow-[0_10px_28px_rgba(20,24,32,0.06)] dark:border-[#292c34] dark:bg-[#181a20] dark:shadow-none">
      <div class="grid min-h-[72px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">边缘吸附</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">拖动 Box 靠近屏幕边缘或其他 Box 时自动贴齐。</p>
        </div>

        <label class="relative block h-7 w-12">
          <input
            class="peer sr-only"
            :checked="props.settings.snapToEdges"
            type="checkbox"
            @change="emit('snapToEdgesChange', ($event.target as HTMLInputElement).checked)"
          />
          <span
            class="absolute inset-0 rounded-full bg-[#c9cdd6] transition-colors peer-checked:bg-[#ff5c5c] dark:bg-[#3a3d46] dark:peer-checked:bg-[#ff6b6b]"
          />
          <span
            class="absolute left-1 top-1 size-5 rounded-full bg-white shadow-[0_2px_7px_rgba(20,24,32,0.25)] transition-transform peer-checked:translate-x-5"
          />
        </label>
      </div>

      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />

      <div class="grid min-h-[84px] grid-cols-[1fr_300px] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">吸附距离</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">数值越大，拖动时越容易贴齐边缘。</p>
        </div>

        <div class="flex items-center gap-3">
          <input
            class="h-1.5 w-full cursor-pointer appearance-none rounded-full bg-[#d8dbe3] accent-[#ff5c5c] dark:bg-[#333640] dark:accent-[#ff6b6b]"
            :max="SNAP_THRESHOLD_INPUT.max"
            :min="SNAP_THRESHOLD_INPUT.min"
            :step="SNAP_THRESHOLD_INPUT.step"
            type="range"
            :value="props.settings.snapThreshold"
            @input="emitSnapThresholdChange"
          />
          <span class="w-[70px] shrink-0 whitespace-nowrap text-right text-[13px] font-medium text-[#555b66] dark:text-[#c7cad1]">
            {{ props.settings.snapThreshold }} {{ SNAP_THRESHOLD_INPUT.unit }}
          </span>
          <button
            aria-label="重置吸附距离"
            class="grid size-8 shrink-0 place-items-center rounded-[6px] text-[#68707d] transition-colors hover:bg-[#eef0f4] hover:text-[#17181c] disabled:cursor-default disabled:opacity-35 disabled:hover:bg-transparent disabled:hover:text-[#68707d] dark:text-[#a7abb5] dark:hover:bg-[#242730] dark:hover:text-[#f4f4f5] dark:disabled:hover:bg-transparent dark:disabled:hover:text-[#a7abb5]"
            :disabled="props.settings.snapThreshold === DEFAULT_APP_SETTINGS.snapThreshold"
            title="重置吸附距离"
            type="button"
            @click="resetSnapThreshold"
          >
            <RotateCcw :size="15" />
          </button>
        </div>
      </div>
    </div>

    <div class="mt-4 overflow-hidden rounded-[8px] border border-[#dfe2e8] bg-[#ffffff] shadow-[0_10px_28px_rgba(20,24,32,0.06)] dark:border-[#292c34] dark:bg-[#181a20] dark:shadow-none">
      <div class="grid min-h-[72px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">Windows 桌面图标</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">使用 Windows 隐藏属性隐藏原生桌面文件，Box 中的项目不受影响。</p>
        </div>

        <label class="relative block h-7 w-12">
          <input
            class="peer sr-only"
            :checked="props.settings.nativeDesktopIconsHidden"
            type="checkbox"
            @change="emit('nativeDesktopIconsHiddenChange', ($event.target as HTMLInputElement).checked)"
          />
          <span
            class="absolute inset-0 rounded-full bg-[#c9cdd6] transition-colors peer-checked:bg-[#ff5c5c] dark:bg-[#3a3d46] dark:peer-checked:bg-[#ff6b6b]"
          />
          <span
            class="absolute left-1 top-1 size-5 rounded-full bg-white shadow-[0_2px_7px_rgba(20,24,32,0.25)] transition-transform peer-checked:translate-x-5"
          />
        </label>
      </div>

      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />

      <div class="px-5 py-4">
        <div class="mb-3 flex items-center justify-between gap-4">
          <div class="min-w-0">
            <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">保留在系统桌面</h2>
            <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">勾选后，即使开启隐藏也不会隐藏这些原生桌面图标。</p>
          </div>
          <span class="shrink-0 text-[12px] font-medium text-[#707684] dark:text-[#9ca0aa]">
            {{ props.settings.nativeDesktopIconIgnorePaths.length }} / {{ props.desktopItems.length }}
          </span>
        </div>

        <div class="dasktop-scrollarea max-h-[220px] overflow-auto rounded-[8px] bg-[#f7f8fb] p-1 dark:bg-[#121419]">
          <label
            v-for="item in props.desktopItems"
            :key="item.path"
            class="flex min-h-9 cursor-pointer items-center gap-3 rounded-[7px] px-3 text-[12px] text-[#343842] transition-colors hover:bg-white dark:text-[#dfe2e8] dark:hover:bg-[#20232b]"
          >
            <input
              class="size-4 shrink-0 accent-[#ff5c5c] dark:accent-[#ff6b6b]"
              :checked="isNativeDesktopIconIgnored(item.path)"
              type="checkbox"
              @change="toggleNativeDesktopIconIgnorePath(item.path, ($event.target as HTMLInputElement).checked)"
            />
            <span class="min-w-0 flex-1 truncate" :title="item.path">{{ item.name }}</span>
          </label>

          <div
            v-if="props.desktopItems.length === 0"
            class="px-3 py-8 text-center text-[12px] text-[#707684] dark:text-[#9ca0aa]"
          >
            当前没有可配置的桌面项目。
          </div>
        </div>
      </div>
    </div>
  </section>
</template>
