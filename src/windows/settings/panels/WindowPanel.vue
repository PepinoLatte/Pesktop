<script setup lang="ts">
import {
  APP_SETTING_NUMBER_LIMITS,
  DEFAULT_APP_SETTINGS,
  type AppSettingBooleanKey,
} from "@/entities/appSettings/defaults";
import {
  WINDOW_BEHAVIOR_SETTING_CONTROLS,
} from "@/entities/appSettings/groups";
import type { AppSettings } from "@/entities/appSettings/types";
import SettingGroup from "../components/SettingGroup.vue";
import SettingRow from "../components/SettingRow.vue";
import SettingSection from "../components/SettingSection.vue";
import SettingSlider from "../components/SettingSlider.vue";
import SettingSwitch from "../components/SettingSwitch.vue";

const props = defineProps<{
  /**
   * 开机自启状态来自系统启动项，和 SQLite 中的渲染偏好分开维护。
   */
  autostartEnabled: boolean;
  panelWidth: string;
  settings: AppSettings;
}>();

const emit = defineEmits<{
  autostartEnabledChange: [value: boolean];
  boxBooleanSettingChange: [key: AppSettingBooleanKey, value: boolean];
  snapThresholdChange: [value: number];
  snapToEdgesChange: [value: boolean];
}>();

const SNAP_THRESHOLD_INPUT = {
  ...APP_SETTING_NUMBER_LIMITS.snapThreshold,
} as const;
</script>

<template>
  <SettingSection
    description="调整 Box 窗口拖动手感和应用启动行为"
    :panel-width="panelWidth"
    title="窗口启动"
  >
    <SettingGroup>
      <SettingRow title="边缘吸附" description="拖动 Box 靠近屏幕边缘或其他 Box 时自动贴齐">
        <SettingSwitch
          ariaLabel="边缘吸附"
          :model-value="props.settings.snapToEdges"
          @change="emit('snapToEdgesChange', $event)"
        />
      </SettingRow>

      <SettingRow
        v-for="control in WINDOW_BEHAVIOR_SETTING_CONTROLS"
        :key="control.key"
        :description="control.description"
        :title="control.label"
      >
        <SettingSwitch
          :ariaLabel="control.label"
          :model-value="Boolean(props.settings[control.key])"
          @change="emit('boxBooleanSettingChange', control.key, $event)"
        />
      </SettingRow>

      <SettingRow
        title="吸附距离"
        description="数值越大，拖动时越容易贴齐边缘"
        grid-class="grid-cols-[1fr_300px]"
      >
        <SettingSlider
          :default-value="DEFAULT_APP_SETTINGS.snapThreshold"
          label="吸附距离"
          :max="SNAP_THRESHOLD_INPUT.max"
          :min="SNAP_THRESHOLD_INPUT.min"
          :model-value="props.settings.snapThreshold"
          :step="SNAP_THRESHOLD_INPUT.step"
          :unit="SNAP_THRESHOLD_INPUT.unit"
          @change="emit('snapThresholdChange', $event)"
          @reset="emit('snapThresholdChange', DEFAULT_APP_SETTINGS.snapThreshold)"
        />
      </SettingRow>

      <SettingRow title="开机自启" description="登录 Windows 后自动启动 Dasktop，并恢复 Box 窗口" :divided="false">
        <SettingSwitch
          ariaLabel="开机自启"
          :model-value="props.autostartEnabled"
          @change="emit('autostartEnabledChange', $event)"
        />
      </SettingRow>
    </SettingGroup>
  </SettingSection>
</template>
