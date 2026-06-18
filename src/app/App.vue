<script setup lang="ts">
import DesktopWindow from "@/windows/desktop/index.vue";
import BoxContextMenuWindow from "@/windows/boxContextMenu/index.vue";
import DragPreviewWindow from "@/windows/dragPreview/index.vue";
import SettingsPage from "@/windows/settings/index.vue";

/**
 * 根路由只按查询参数区分设置窗、Box 窗和拖影窗，避免再引入旧的全屏桌面覆盖入口
 */
const searchParams = new URLSearchParams(window.location.search);
const boxId = searchParams.get("boxId");
const isBoxMenu = searchParams.has("boxMenu");
const isDragPreview = searchParams.has("dragPreview");
</script>

<template>
  <DragPreviewWindow v-if="isDragPreview" />
  <BoxContextMenuWindow v-else-if="isBoxMenu" />
  <DesktopWindow v-else-if="boxId" :box-id="boxId" />
  <SettingsPage v-else />
</template>
