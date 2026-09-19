<script setup lang="ts">
import { ref } from "vue";
import { ArrowLeft, Trash2 } from "@lucide/vue";
import type { DesktopBox } from "@/entities/desktopBox/types";
import { BUILTIN_COVER_ICONS, builtinCoverValue, parseBuiltinCoverId } from "../model/builtinCovers";

/**
 * 封面选择器内嵌在更多菜单中：内置图标库点选即生效，也支持本地图片压缩后作为封面。
 */
const props = defineProps<{
  box: DesktopBox;
}>();

const emit = defineEmits<{
  back: [];
  select: [coverIcon: string | null];
}>();

const fileInputRef = ref<HTMLInputElement | null>(null);
const fileError = ref("");

const activeBuiltinId = parseBuiltinCoverId(props.box.coverIcon);

function selectBuiltin(id: string): void {
  emit("select", builtinCoverValue(id));
}

function clearCover(): void {
  emit("select", null);
}

function openFilePicker(): void {
  fileInputRef.value?.click();
}

/**
 * 本地图片统一压缩成 128px 居中 PNG 再落库：图标显示尺寸远小于原图，
 * 压缩同时规避超大 data URL 撑爆数据库行
 */
function handleFileChange(event: Event): void {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  fileError.value = "";
  if (!file) {
    return;
  }

  if (!file.type.startsWith("image/")) {
    fileError.value = "仅支持图片文件";
    return;
  }

  const reader = new FileReader();
  reader.onload = () => {
    const image = new Image();
    image.onload = () => {
      const canvas = document.createElement("canvas");
      canvas.width = 128;
      canvas.height = 128;
      const context = canvas.getContext("2d");
      if (!context) {
        fileError.value = "图片处理失败";
        return;
      }

      const scale = Math.min(128 / image.width, 128 / image.height);
      const width = image.width * scale;
      const height = image.height * scale;
      context.drawImage(image, (128 - width) / 2, (128 - height) / 2, width, height);
      emit("select", canvas.toDataURL("image/png"));
    };
    image.onerror = () => {
      fileError.value = "图片读取失败";
    };
    image.src = String(reader.result);
  };
  reader.readAsDataURL(file);
}
</script>

<template>
  <div class="grid gap-1 px-1">
    <button
      :class="'flex h-8 w-full items-center gap-1.5 rounded-[8px] px-2 text-left text-[12px] font-semibold text-slate-800 transition-colors hover:bg-[#eef1f6] dark:text-slate-100 dark:hover:bg-[#2a2d36]'"
      type="button"
      @click="$emit('back')"
    >
      <ArrowLeft :size="14" />
      设置封面
    </button>
    <div class="grid grid-cols-6 gap-1 rounded-[8px] bg-[#f2f4f9] p-1.5 dark:bg-[#191b21]">
      <button
        v-for="cover in BUILTIN_COVER_ICONS"
        :key="cover.id"
        :class="[
          'grid aspect-square place-items-center rounded-[7px] transition-colors',
          activeBuiltinId === cover.id
            ? 'bg-[#2f6bff]/15 text-[#2f6bff] ring-1 ring-[#2f6bff]/60'
            : 'text-slate-500 hover:bg-white hover:text-slate-800 dark:text-slate-400 dark:hover:bg-[#2a2d36] dark:hover:text-slate-100',
        ]"
        :title="cover.label"
        type="button"
        @click="selectBuiltin(cover.id)"
      >
        <component :is="cover.component" :size="17" />
      </button>
    </div>
    <div class="grid grid-cols-2 gap-1">
      <button
        class="rounded-[8px] bg-[#e9edf5] px-2 py-1.5 text-[11px] font-medium text-slate-700 transition-colors hover:bg-[#dfe4ee] dark:bg-[#30343e] dark:text-slate-200 dark:hover:bg-[#3a3f4a]"
        type="button"
        @click="openFilePicker"
      >
        本地图片
      </button>
      <button
        v-if="box.coverIcon"
        class="rounded-[8px] bg-[#fdeaea] px-2 py-1.5 text-[11px] font-medium text-red-600 transition-colors hover:bg-[#f9dcdc] dark:bg-[#3c2427] dark:text-red-300 dark:hover:bg-[#4a2b2f]"
        type="button"
        @click="clearCover"
      >
        <span class="inline-flex items-center gap-1">
          <Trash2 :size="12" />
          清除封面
        </span>
      </button>
    </div>
    <input
      ref="fileInputRef"
      accept="image/*"
      class="hidden"
      type="file"
      @change="handleFileChange"
    />
    <p v-if="fileError" class="px-1 text-[10px] text-red-500">{{ fileError }}</p>
  </div>
</template>
