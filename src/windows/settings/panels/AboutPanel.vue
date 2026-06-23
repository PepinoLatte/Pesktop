<script setup lang="ts">
import { APP_META } from "@/shared/config/appMeta";
import SettingGroup from "../components/SettingGroup.vue";
import SettingRow from "../components/SettingRow.vue";
import SettingSection from "../components/SettingSection.vue";

/**
 * 关于面板只展示可核对的产品元信息，避免把设置页做成完整宣传页。
 */
defineProps<{
  desktopPath: string;
  panelWidth: string;
}>();

</script>

<template>
  <SettingSection
    description="当前桌面扩展的运行信息"
    :panel-width="panelWidth"
    title="关于"
  >
    <SettingGroup>
      <SettingRow title="应用名称">
        <div class="min-w-0 py-3 text-right">
          <strong class="block text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">{{ APP_META.displayName }}</strong>
          <p class="mt-1 text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">
            名字里藏着作者的 Da，也落在每天打开的 Desktop 上
          </p>
        </div>
      </SettingRow>

      <SettingRow title="作者" description="产品设计与开发">
        <strong class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">{{ APP_META.author }}</strong>
      </SettingRow>

      <SettingRow title="版本" description="当前演示与试用版本">
        <strong class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">v{{ APP_META.version }}</strong>
      </SettingRow>

      <SettingRow title="Powered by" description="参与产品构建与实现协作">
        <strong class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">{{ APP_META.poweredBy }}</strong>
      </SettingRow>

      <SettingRow title="桌面路径" :divided="false">
        <strong class="max-w-[420px] truncate text-right text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">
          {{ desktopPath || "读取中" }}
        </strong>
      </SettingRow>
    </SettingGroup>
  </SettingSection>
</template>
