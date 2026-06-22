<script setup lang="ts">
import {
  BOX_CONFLICT_POLICY_OPTIONS,
  BOX_DELETE_POLICY_OPTIONS,
  BOX_DROP_ACTION_OPTIONS,
  FILE_BEHAVIOR_SETTING_CONTROLS,
  NAME_DISPLAY_MODE_OPTIONS,
} from "@/entities/appSettings/groups";
import type { AppSettingBooleanKey } from "@/entities/appSettings/defaults";
import type {
  AppSettings,
  BoxConflictPolicy,
  BoxDeletePolicy,
  BoxDropAction,
} from "@/entities/appSettings/types";
import type { DesktopNameDisplayMode } from "@/entities/desktopItem/types";
import SegmentedControl from "@/shared/ui/SegmentedControl.vue";
import SettingGroup from "../components/SettingGroup.vue";
import SettingRow from "../components/SettingRow.vue";
import SettingSection from "../components/SettingSection.vue";
import SettingSwitch from "../components/SettingSwitch.vue";

const props = defineProps<{
  panelWidth: string;
  settings: AppSettings;
}>();

const emit = defineEmits<{
  boxBooleanSettingChange: [key: AppSettingBooleanKey, value: boolean];
  boxConflictPolicyChange: [value: BoxConflictPolicy];
  boxDeletePolicyChange: [value: BoxDeletePolicy];
  boxDragOutActionChange: [value: BoxDropAction];
  boxDropActionChange: [value: BoxDropAction];
  nameDisplayModeChange: [mode: DesktopNameDisplayMode];
}>();
</script>

<template>
  <SettingSection
    description="调整文件传输策略、删除策略和文件名展示"
    :panel-width="panelWidth"
    title="规则显示"
  >
    <SettingGroup>
      <SettingRow title="拖入 Box" description="控制桌面或 Explorer 文件拖入后的处理方式">
        <SegmentedControl
          :model-value="props.settings.boxDropAction"
          :options="BOX_DROP_ACTION_OPTIONS"
          @change="emit('boxDropActionChange', $event)"
        />
      </SettingRow>

      <SettingRow title="拖出 Box" description="控制 Box 内文件拖到桌面后的处理方式">
        <SegmentedControl
          :model-value="props.settings.boxDragOutAction"
          :options="BOX_DROP_ACTION_OPTIONS"
          @change="emit('boxDragOutActionChange', $event)"
        />
      </SettingRow>

      <SettingRow title="删除 Box" description="控制删除 Box 时真实收纳文件夹的处理方式" :divided="false">
        <SegmentedControl
          :model-value="props.settings.boxDeletePolicy"
          :options="BOX_DELETE_POLICY_OPTIONS"
          @change="emit('boxDeletePolicyChange', $event)"
        />
      </SettingRow>

      <SettingRow title="同名处理" description="控制 Box 传输遇到同名文件时的落盘方式">
        <SegmentedControl
          :model-value="props.settings.boxConflictPolicy"
          :options="BOX_CONFLICT_POLICY_OPTIONS"
          @change="emit('boxConflictPolicyChange', $event)"
        />
      </SettingRow>
    </SettingGroup>

    <SettingGroup class="mt-4">
      <SettingRow title="文件名" description="控制 Box 内文件标签的后缀展示方式">
        <SegmentedControl
          :model-value="props.settings.nameDisplayMode"
          :options="NAME_DISPLAY_MODE_OPTIONS"
          @change="emit('nameDisplayModeChange', $event)"
        />
      </SettingRow>

      <SettingRow
        v-for="(control, index) in FILE_BEHAVIOR_SETTING_CONTROLS"
        :key="control.key"
        :description="control.description"
        :divided="index < FILE_BEHAVIOR_SETTING_CONTROLS.length - 1"
        :title="control.label"
      >
        <SettingSwitch
          :ariaLabel="control.label"
          :model-value="Boolean(props.settings[control.key])"
          @change="emit('boxBooleanSettingChange', control.key, $event)"
        />
      </SettingRow>
    </SettingGroup>
  </SettingSection>
</template>
