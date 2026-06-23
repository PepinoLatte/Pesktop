<script setup lang="ts">
import {
  BOX_COLLAPSE_TIMING_SETTING_CONTROLS,
  BOX_VISUAL_SETTING_CONTROLS,
  THEME_SEGMENT_OPTIONS,
  type BoxAppearanceNumberSettingKey,
} from "@/entities/appSettings/groups";
import { DEFAULT_APP_SETTINGS } from "@/entities/appSettings/defaults";
import type { AppSettings, ThemeMode } from "@/entities/appSettings/types";
import SegmentedControl from "@/shared/ui/SegmentedControl.vue";
import SettingGroup from "../components/SettingGroup.vue";
import SettingRow from "../components/SettingRow.vue";
import SettingSection from "../components/SettingSection.vue";
import SettingSlider from "../components/SettingSlider.vue";

const props = defineProps<{
  panelWidth: string;
  settings: AppSettings;
}>();

const emit = defineEmits<{
  boxThemeChange: [theme: ThemeMode];
  boxVisualSettingChange: [key: BoxAppearanceNumberSettingKey, value: number];
  settingsThemeChange: [theme: ThemeMode];
}>();
</script>

<template>
  <SettingSection
    description="调整设置窗和 Box 的显示偏好"
    :panel-width="panelWidth"
    title="外观"
  >
    <SettingGroup>
      <SettingRow title="设置主题" description="只影响当前设置窗口的明暗显示">
        <SegmentedControl
          :model-value="props.settings.settingsTheme"
          :options="THEME_SEGMENT_OPTIONS"
          @change="emit('settingsThemeChange', $event)"
        />
      </SettingRow>

      <SettingRow title="Box 主题" description="只影响桌面上的 Box 窗口" :divided="false">
        <SegmentedControl
          :model-value="props.settings.boxTheme"
          :options="THEME_SEGMENT_OPTIONS"
          @change="emit('boxThemeChange', $event)"
        />
      </SettingRow>
    </SettingGroup>

    <SettingGroup class="mt-4">
      <SettingRow
        v-for="(control, index) in BOX_COLLAPSE_TIMING_SETTING_CONTROLS"
        :key="control.key"
        :description="control.description"
        :divided="index < BOX_COLLAPSE_TIMING_SETTING_CONTROLS.length - 1"
        grid-class="grid-cols-[1fr_300px]"
        :title="control.label"
      >
        <SettingSlider
          :default-value="DEFAULT_APP_SETTINGS[control.key]"
          :label="control.label"
          :max="control.max"
          :min="control.min"
          :model-value="props.settings[control.key]"
          :step="control.step"
          :unit="control.unit"
          @change="emit('boxVisualSettingChange', control.key, $event)"
          @reset="emit('boxVisualSettingChange', control.key, DEFAULT_APP_SETTINGS[control.key])"
        />
      </SettingRow>
    </SettingGroup>

    <SettingGroup class="mt-4">
      <SettingRow
        v-for="(control, index) in BOX_VISUAL_SETTING_CONTROLS"
        :key="control.key"
        :description="control.description"
        :divided="index < BOX_VISUAL_SETTING_CONTROLS.length - 1"
        grid-class="grid-cols-[1fr_300px]"
        :title="control.label"
      >
        <SettingSlider
          :default-value="DEFAULT_APP_SETTINGS[control.key]"
          :label="control.label"
          :max="control.max"
          :min="control.min"
          :model-value="props.settings[control.key]"
          :step="control.step"
          :unit="control.unit"
          @change="emit('boxVisualSettingChange', control.key, $event)"
          @reset="emit('boxVisualSettingChange', control.key, DEFAULT_APP_SETTINGS[control.key])"
        />
      </SettingRow>
    </SettingGroup>
  </SettingSection>
</template>
