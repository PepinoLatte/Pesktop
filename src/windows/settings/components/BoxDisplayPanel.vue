<script setup lang="ts">
import { ExternalLink, FolderOpen, MoveRight, RotateCcw } from "@lucide/vue";
import {
  APP_SETTING_NUMBER_LIMITS,
  APP_SETTING_KEYS,
  DEFAULT_APP_SETTINGS,
  type AppSettingBooleanKey,
} from "@/entities/appSettings/defaults";
import type {
  AppSettings,
  BoxConflictPolicy,
  BoxDeletePolicy,
  BoxDropAction,
} from "@/entities/appSettings/types";
import type { DesktopNameDisplayMode } from "@/entities/desktopItem/types";
import SegmentedControl from "@/shared/ui/SegmentedControl.vue";

/**
 * 删除策略文案按风险从低到高排列，让用户清楚删除 Box 时真实文件的去向
 */
const BOX_DELETE_POLICY_OPTIONS: Array<{
  label: string;
  value: BoxDeletePolicy;
}> = [
  { label: "移回桌面", value: "moveContentsToDesktop" },
  { label: "保留文件夹", value: "keepFolder" },
  { label: "进回收站", value: "recycleFolder" },
];

/**
 * 拖入策略文案从“是否改变原文件”切入，帮助用户理解复制、移动和映射的真实差异
 */
const BOX_DROP_ACTION_OPTIONS: Array<{
  label: string;
  value: BoxDropAction;
}> = [
  { label: "移动", value: "move" },
  { label: "复制", value: "copy" },
  { label: "映射", value: "map" },
];

/**
 * 同名策略默认使用自动重命名，既保留已有文件，也避免 Windows 冲突弹窗被 Box 遮挡。
 */
const BOX_CONFLICT_POLICY_OPTIONS: Array<{
  label: string;
  value: BoxConflictPolicy;
}> = [
  { label: "自动重命名", value: "rename" },
  { label: "跳过同名", value: "skip" },
  { label: "替换已有", value: "replace" },
];

/**
 * 文件名显示选项只改变 Box 标签文本，不改变真实文件名和磁盘路径。
 */
const NAME_DISPLAY_MODE_OPTIONS: Array<{
  label: string;
  value: DesktopNameDisplayMode;
}> = [
  { label: "完整", value: "full" },
  { label: "隐藏快捷方式", value: "hideShortcutExtension" },
  { label: "隐藏全部后缀", value: "hideAllExtensions" },
];

/**
 * 文件显示与打开方式属于 Box 使用行为，集中放在设置页避免和纯外观滑块混杂。
 */
const FILE_BEHAVIOR_SETTING_CONTROLS: Array<{
  description: string;
  key: AppSettingBooleanKey;
  label: string;
}> = [
  {
    key: APP_SETTING_KEYS.showItemLabels,
    label: "显示文件名",
    description: "关闭后 Box 内只显示图标，适合极简桌面",
  },
  {
    key: APP_SETTING_KEYS.showShortcutArrow,
    label: "显示快捷方式角标",
    description: "只影响 `.lnk` 的视觉角标，不改变快捷方式文件",
  },
  {
    key: APP_SETTING_KEYS.doubleClickOpenItems,
    label: "双击打开",
    description: "关闭后单击即可打开文件，拖拽仍需要先移动超过阈值",
  },
];

/**
 * 窗口行为开关与吸附距离相邻展示，便于用户理解 resize 和拖动都属于窗口手感设置。
 */
const WINDOW_BEHAVIOR_SETTING_CONTROLS: Array<{
  description: string;
  key: AppSettingBooleanKey;
  label: string;
}> = [
  {
    key: APP_SETTING_KEYS.boxResizeGridEnabled,
    label: "按网格调整大小",
    description: "拖拽窗口边缘时吸附到完整图标行列，减少半截空位",
  },
];

/**
 * 吸附距离滑块范围直接跟随应用设置范围，避免模板限制和持久化校验脱节
 */
const SNAP_THRESHOLD_INPUT = {
  ...APP_SETTING_NUMBER_LIMITS.snapThreshold,
} as const;

/**
 * 文件夹型 Box 设置面板维护真实收纳目录、删除策略和窗口行为
 */
const props = defineProps<{
  /**
   * 开机自启状态来自系统启动项，和 SQLite 中的渲染偏好分开维护
   */
  autostartEnabled: boolean;
  /**
   * 设置页父级统一下发内容宽度，避免各面板各自维护页面密度
   */
  panelWidth: string;
  settings: AppSettings;
  /**
   * 迁移按钮由父级计算真实待迁移数量，避免设置面板直接理解 Box 路径规则。
   */
  migratableBoxCount: number;
  /**
   * 迁移涉及真实文件移动，执行期间禁用相关按钮防止重复提交。
   */
  migrationBusy: boolean;
}>();

const emit = defineEmits<{
  autostartEnabledChange: [value: boolean];
  boxConflictPolicyChange: [value: BoxConflictPolicy];
  boxDeletePolicyChange: [value: BoxDeletePolicy];
  boxDragOutActionChange: [value: BoxDropAction];
  boxDropActionChange: [value: BoxDropAction];
  boxBooleanSettingChange: [key: AppSettingBooleanKey, value: boolean];
  collectionRootChoose: [];
  collectionRootOpen: [];
  collectionRootMigrate: [];
  nameDisplayModeChange: [mode: DesktopNameDisplayMode];
  snapThresholdChange: [value: number];
  snapToEdgesChange: [value: boolean];
}>();

/**
 * 吸附距离沿用窗口行为设置，但入口已经合并到当前设置页
 */
function emitSnapThresholdChange(event: Event): void {
  emit("snapThresholdChange", Number((event.target as HTMLInputElement).value));
}

/**
 * 吸附距离重置到默认阈值，方便用户调乱后快速回到稳定拖动手感
 */
function resetSnapThreshold(): void {
  emit("snapThresholdChange", DEFAULT_APP_SETTINGS.snapThreshold);
}

/**
 * 收纳位置操作按钮共享同一视觉语言，避免同一设置行出现多个不等高按钮。
 */
const COLLECTION_ROOT_ACTION_BUTTON_CLASS =
  "inline-flex h-9 items-center gap-2 rounded-[9px] border border-[#dfe2e8] bg-[#ffffff] px-3 text-[13px] font-medium text-[#555b66] transition-colors hover:bg-[#f6f7fa] hover:text-[#17181c] disabled:cursor-not-allowed disabled:opacity-45 dark:border-[#292c34] dark:bg-[#181a20] dark:text-[#a7abb5] dark:hover:bg-[#202229] dark:hover:text-[#f4f4f5]";
</script>

<template>
  <section class="mx-auto w-full px-8 py-7" :class="panelWidth">
    <div class="mb-6">
      <h1 class="text-[24px] font-semibold tracking-[0] text-[#17181c] dark:text-[#f4f4f5]">设置</h1>
      <p class="mt-1 text-[13px] text-[#6f7480] dark:text-[#a7abb5]">调整真实文件夹 Box 的文件位置和窗口行为</p>
    </div>

    <div class="overflow-hidden rounded-[8px] border border-[#dfe2e8] bg-[#ffffff] shadow-[0_10px_28px_rgba(20,24,32,0.06)] dark:border-[#292c34] dark:bg-[#181a20] dark:shadow-none">
      <div class="grid min-h-[84px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">收纳位置</h2>
          <p class="mt-1 truncate text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">
            {{ props.settings.collectionRootPath || "尚未选择" }}
          </p>
        </div>

        <div class="flex items-center gap-2">
          <button
            :class="COLLECTION_ROOT_ACTION_BUTTON_CLASS"
            :disabled="!props.settings.collectionRootPath || props.migrationBusy"
            type="button"
            @click="emit('collectionRootOpen')"
          >
            <ExternalLink :size="15" />
            打开
          </button>
          <button
            :class="COLLECTION_ROOT_ACTION_BUTTON_CLASS"
            :disabled="props.migrationBusy"
            type="button"
            @click="emit('collectionRootChoose')"
          >
            <FolderOpen :size="15" />
            选择
          </button>
          <button
            :class="COLLECTION_ROOT_ACTION_BUTTON_CLASS"
            :disabled="!props.settings.collectionRootPath || props.migrationBusy || props.migratableBoxCount === 0"
            type="button"
            @click="emit('collectionRootMigrate')"
          >
            <MoveRight :size="15" />
            {{ props.migrationBusy ? "迁移中" : props.migratableBoxCount > 0 ? `迁移 ${props.migratableBoxCount}` : "已迁移" }}
          </button>
        </div>
      </div>

      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />

      <div class="grid min-h-[76px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">拖入 Box</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">控制桌面或 Explorer 文件拖入后的处理方式</p>
        </div>

        <SegmentedControl
          :model-value="props.settings.boxDropAction"
          :options="BOX_DROP_ACTION_OPTIONS"
          @change="emit('boxDropActionChange', $event)"
        />
      </div>

      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />

      <div class="grid min-h-[76px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">同名处理</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">控制 Box 传输遇到同名文件时的落盘方式</p>
        </div>

        <SegmentedControl
          :model-value="props.settings.boxConflictPolicy"
          :options="BOX_CONFLICT_POLICY_OPTIONS"
          @change="emit('boxConflictPolicyChange', $event)"
        />
      </div>

      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />

      <div class="grid min-h-[76px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">拖出到桌面</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">控制 Box 内文件拖到桌面后的处理方式</p>
        </div>

        <SegmentedControl
          :model-value="props.settings.boxDragOutAction"
          :options="BOX_DROP_ACTION_OPTIONS"
          @change="emit('boxDragOutActionChange', $event)"
        />
      </div>

      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />

      <div class="grid min-h-[76px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">删除 Box</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">控制删除 Box 时真实收纳文件夹的处理方式</p>
        </div>

        <SegmentedControl
          :model-value="props.settings.boxDeletePolicy"
          :options="BOX_DELETE_POLICY_OPTIONS"
          @change="emit('boxDeletePolicyChange', $event)"
        />
      </div>
    </div>

    <div class="mt-4 overflow-hidden rounded-[8px] border border-[#dfe2e8] bg-[#ffffff] shadow-[0_10px_28px_rgba(20,24,32,0.06)] dark:border-[#292c34] dark:bg-[#181a20] dark:shadow-none">
      <div class="grid min-h-[76px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">文件名显示</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">控制 Box 内文件标签的后缀展示方式</p>
        </div>

        <SegmentedControl
          :model-value="props.settings.nameDisplayMode"
          :options="NAME_DISPLAY_MODE_OPTIONS"
          @change="emit('nameDisplayModeChange', $event)"
        />
      </div>

      <div class="h-px bg-[#e7e9ee] dark:bg-[#292c34]" />

      <div
        v-for="(control, index) in FILE_BEHAVIOR_SETTING_CONTROLS"
        :key="control.key"
      >
        <div class="grid min-h-[72px] grid-cols-[1fr_auto] items-center gap-6 px-5">
          <div class="min-w-0 pr-4">
            <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">{{ control.label }}</h2>
            <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">{{ control.description }}</p>
          </div>

          <label class="relative block h-7 w-12">
            <input
              class="peer sr-only"
              :checked="props.settings[control.key]"
              type="checkbox"
              @change="emit('boxBooleanSettingChange', control.key, ($event.target as HTMLInputElement).checked)"
            />
            <span
              class="absolute inset-0 rounded-full bg-[#c9cdd6] transition-colors peer-checked:bg-[#ff5c5c] dark:bg-[#3a3d46] dark:peer-checked:bg-[#ff6b6b]"
            />
            <span
              class="absolute left-1 top-1 size-5 rounded-full bg-white shadow-[0_2px_7px_rgba(20,24,32,0.25)] transition-transform peer-checked:translate-x-5"
            />
          </label>
        </div>

        <div
          v-if="index < FILE_BEHAVIOR_SETTING_CONTROLS.length - 1"
          class="h-px bg-[#e7e9ee] dark:bg-[#292c34]"
        />
      </div>
    </div>

    <div class="mt-4 overflow-hidden rounded-[8px] border border-[#dfe2e8] bg-[#ffffff] shadow-[0_10px_28px_rgba(20,24,32,0.06)] dark:border-[#292c34] dark:bg-[#181a20] dark:shadow-none">
      <div class="grid min-h-[72px] grid-cols-[1fr_auto] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">边缘吸附</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">拖动 Box 靠近屏幕边缘或其他 Box 时自动贴齐</p>
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

      <div
        v-for="control in WINDOW_BEHAVIOR_SETTING_CONTROLS"
        :key="control.key"
      >
        <div class="grid min-h-[72px] grid-cols-[1fr_auto] items-center gap-6 px-5">
          <div class="min-w-0 pr-4">
            <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">{{ control.label }}</h2>
            <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">{{ control.description }}</p>
          </div>

          <label class="relative block h-7 w-12">
            <input
              class="peer sr-only"
              :checked="props.settings[control.key]"
              type="checkbox"
              @change="emit('boxBooleanSettingChange', control.key, ($event.target as HTMLInputElement).checked)"
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
      </div>

      <div class="grid min-h-[84px] grid-cols-[1fr_300px] items-center gap-6 px-5">
        <div class="min-w-0 pr-4">
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">吸附距离</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">数值越大，拖动时越容易贴齐边缘</p>
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
          <h2 class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">开机自启</h2>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">登录 Windows 后自动启动 Dasktop，并恢复 Box 窗口</p>
        </div>

        <label class="relative block h-7 w-12">
          <input
            class="peer sr-only"
            :checked="props.autostartEnabled"
            type="checkbox"
            @change="emit('autostartEnabledChange', ($event.target as HTMLInputElement).checked)"
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
