<script setup lang="ts">
import { computed } from "vue";
import {
  Archive,
  Briefcase,
  Code2,
  Coffee,
  Compass,
  FileText,
  Film,
  Folder,
  FolderArchive,
  Gamepad2,
  Layers,
  Monitor,
  Music,
  Palette,
  Rocket,
  Sparkles,
  Star,
  Terminal,
  Download,
} from "@lucide/vue";

/**
 * 图标映射字典，根据持久化的预设图标名称动态匹配 Lucide 图标组件
 */
const ICON_COMPONENT_MAP: Record<string, any> = {
  Archive,
  Briefcase,
  Code2,
  Coffee,
  Compass,
  Download,
  FileText,
  Film,
  Folder,
  FolderArchive,
  Gamepad2,
  Layers,
  Monitor,
  Music,
  Palette,
  Rocket,
  Sparkles,
  Star,
  Terminal,
};

const props = withDefaults(
  defineProps<{
    icon?: string;
    size?: number;
    strokeWidth?: number;
  }>(),
  {
    icon: "Folder",
    size: 20,
    strokeWidth: 2,
  },
);

const resolvedComponent = computed(() => {
  if (!props.icon) {
    return Folder;
  }
  return ICON_COMPONENT_MAP[props.icon] || Folder;
});
</script>

<template>
  <component
    :is="resolvedComponent"
    :size="size"
    :stroke-width="strokeWidth"
    class="shrink-0"
  />
</template>
