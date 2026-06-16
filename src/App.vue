<script setup lang="ts">
import DesktopWindow from "./features/desktop/index.vue";
import SettingsPage from "./features/settings/index.vue";

/**
 * 根路由只按查询参数区分设置窗和 Box 窗，避免再引入旧的全屏桌面覆盖入口。
 */
const searchParams = new URLSearchParams(window.location.search);
const boxId = searchParams.get("boxId");
</script>

<template>
  <DesktopWindow v-if="boxId" :box-id="boxId" />
  <SettingsPage v-else />
</template>
