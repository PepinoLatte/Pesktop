<script setup lang="ts">
import {
  ArrowUpRight,
  ExternalLink,
  FolderOpen,
  FolderPlus,
  Lock,
  LockOpen,
  MoveRight,
  RefreshCw,
  Trash2,
} from "@lucide/vue";
import { useDesktopBoxDeleteConfirmation } from "@/entities/desktopBox/deleteConfirmation";
import type { DesktopBox } from "@/entities/desktopBox/types";
import SettingGroup from "../components/SettingGroup.vue";
import SettingRow from "../components/SettingRow.vue";
import SettingSection from "../components/SettingSection.vue";

/**
 * Box 面板统一承载 Box 列表和收纳根目录，避免“收纳”作为独立菜单打断创建流程。
 */
defineProps<{
  boxes: DesktopBox[];
  collectionRootPath: string;
  /**
   * 迁移按钮由父级计算真实待迁移数量，避免设置面板直接理解 Box 路径规则。
   */
  migratableBoxCount: number;
  /**
   * 迁移涉及真实文件移动，执行期间禁用相关按钮防止重复提交。
   */
  migrationBusy: boolean;
  panelWidth: string;
}>();

const emit = defineEmits<{
  collectionRootChoose: [];
  collectionRootMigrate: [];
  collectionRootOpen: [];
  createBox: [];
  deleteBox: [box: DesktopBox];
  openBox: [box: DesktopBox];
  openFolder: [box: DesktopBox];
  refresh: [];
  toggleBoxLocked: [box: DesktopBox];
}>();

const {
  clearBoxDeleteConfirmation,
  isConfirmingBoxDelete,
  requestBoxDeleteConfirmation,
} = useDesktopBoxDeleteConfirmation();

/**
 * 收纳位置操作按钮共享同一视觉语言，避免 Box 面板内出现多个不等高按钮。
 */
const COLLECTION_ROOT_ACTION_BUTTON_CLASS =
  "inline-flex h-9 items-center gap-2 rounded-[9px] border border-[#dfe2e8] bg-[#ffffff] px-3 text-[13px] font-medium text-[#555b66] transition-colors hover:bg-[#f6f7fa] hover:text-[#17181c] disabled:cursor-not-allowed disabled:opacity-45 dark:border-[#292c34] dark:bg-[#181a20] dark:text-[#a7abb5] dark:hover:bg-[#202229] dark:hover:text-[#f4f4f5]";

/**
 * Box 页主操作沿用设置页强调色，只保留一个高权重入口，避免列表工具按钮抢焦点。
 */
const PRIMARY_ACTION_BUTTON_CLASS =
  "inline-flex h-9 items-center gap-2 rounded-[9px] bg-[#ff5c5c] px-4 text-[13px] font-semibold text-white shadow-[0_10px_24px_rgba(255,92,92,0.24)] transition-colors hover:bg-[#ee4d4d] disabled:cursor-not-allowed disabled:opacity-45";

/**
 * 列表行的图标按钮保持 32px 稳定尺寸，避免确认删除文案出现时挤压相邻操作。
 */
const BOX_ICON_BUTTON_CLASS =
  "grid size-8 place-items-center rounded-[7px] text-[#68707d] transition-colors hover:bg-[#eef0f4] hover:text-[#17181c] dark:text-[#a7abb5] dark:hover:bg-[#242730] dark:hover:text-[#f4f4f5]";

/**
 * 设置列表需要给空标题 Box 一个识别名称，真实 Box 标题仍保持用户保存的空文本。
 */
function displayBoxTitle(box: DesktopBox): string {
  return box.title || "未命名 Box";
}

/**
 * 创建新 Box 属于非危险操作，执行前清掉任何悬挂的删除确认态。
 */
function createBoxFromPanel(): void {
  clearBoxDeleteConfirmation();
  emit("createBox");
}

function openBoxFromPanel(box: DesktopBox): void {
  clearBoxDeleteConfirmation();
  emit("openBox", box);
}

function openFolderFromPanel(box: DesktopBox): void {
  clearBoxDeleteConfirmation();
  emit("openFolder", box);
}

function toggleBoxLockedFromPanel(box: DesktopBox): void {
  clearBoxDeleteConfirmation();
  emit("toggleBoxLocked", box);
}

/**
 * 删除按钮采用二段式交互，第一次点击只切换按钮状态，第二次点击才向父级发出删除事件。
 */
function deleteBoxFromPanel(box: DesktopBox): void {
  if (!requestBoxDeleteConfirmation(box)) {
    return;
  }

  emit("deleteBox", box);
}

function refreshFromPanel(): void {
  clearBoxDeleteConfirmation();
  emit("refresh");
}
</script>

<template>
  <SettingSection
    description="管理 Box 和新建 Box 的真实收纳位置"
    :panel-width="panelWidth"
    title="Box"
  >
    <SettingGroup>
      <SettingRow title="收纳位置" :divided="false">
        <template #description>
          <p class="mt-1 truncate text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">
            {{ collectionRootPath || "尚未选择" }}
          </p>
        </template>

        <div class="flex items-center gap-2">
          <button
            :class="COLLECTION_ROOT_ACTION_BUTTON_CLASS"
            :disabled="!collectionRootPath || migrationBusy"
            type="button"
            @click="emit('collectionRootOpen')"
          >
            <ExternalLink :size="15" />
            打开
          </button>
          <button
            :class="COLLECTION_ROOT_ACTION_BUTTON_CLASS"
            :disabled="migrationBusy"
            type="button"
            @click="emit('collectionRootChoose')"
          >
            <FolderOpen :size="15" />
            选择
          </button>
          <button
            :class="COLLECTION_ROOT_ACTION_BUTTON_CLASS"
            :disabled="!collectionRootPath || migrationBusy || migratableBoxCount === 0"
            type="button"
            @click="emit('collectionRootMigrate')"
          >
            <MoveRight :size="15" />
            {{ migrationBusy ? "迁移中" : migratableBoxCount > 0 ? `迁移 ${migratableBoxCount}` : "已迁移" }}
          </button>
        </div>
      </SettingRow>
    </SettingGroup>

    <SettingGroup class="mt-4">
      <SettingRow
        title="Box 列表"
        :description="boxes.length > 0 ? `${boxes.length} 个 Box 正在管理桌面文件` : '还没有 Box，创建后会自动使用上方收纳位置'"
        :divided="boxes.length > 0"
      >
        <div class="flex items-center gap-2">
          <button
            :class="COLLECTION_ROOT_ACTION_BUTTON_CLASS"
            type="button"
            @click="refreshFromPanel"
          >
            <RefreshCw :size="15" />
            刷新
          </button>
          <button
            :class="PRIMARY_ACTION_BUTTON_CLASS"
            type="button"
            @click="createBoxFromPanel"
          >
            <FolderPlus :size="16" />
            新增 Box
          </button>
        </div>
      </SettingRow>

      <SettingRow
        v-for="(box, index) in boxes"
        :key="box.id"
        :divided="index < boxes.length - 1"
        grid-class="grid-cols-[minmax(0,1fr)_auto]"
        :title="displayBoxTitle(box)"
      >
        <template #description>
          <p class="mt-1 truncate text-[12px] leading-5 text-[#707684] dark:text-[#9ca0aa]">
            {{ box.locked ? "已锁定" : "可移动" }} · {{ box.width }} × {{ box.height }} · {{ box.folderPath }}
          </p>
        </template>

        <div class="flex items-center gap-1">
          <button
            :aria-label="box.locked ? '解除锁定 Box' : '锁定 Box'"
            :class="BOX_ICON_BUTTON_CLASS"
            :title="box.locked ? '解除锁定 Box' : '锁定 Box'"
            type="button"
            @click="toggleBoxLockedFromPanel(box)"
          >
            <LockOpen v-if="box.locked" :size="15" />
            <Lock v-else :size="15" />
          </button>
          <button
            aria-label="打开真实文件夹"
            :class="BOX_ICON_BUTTON_CLASS"
            title="打开真实文件夹"
            type="button"
            @click="openFolderFromPanel(box)"
          >
            <FolderOpen :size="16" />
          </button>
          <button
            aria-label="打开 Box"
            :class="BOX_ICON_BUTTON_CLASS"
            title="打开 Box"
            type="button"
            @click="openBoxFromPanel(box)"
          >
            <ArrowUpRight :size="16" />
          </button>
          <span class="flex h-8 w-[78px] justify-end">
            <button
              :aria-label="isConfirmingBoxDelete(box) ? '确认删除 Box' : '删除 Box'"
              class="inline-flex h-8 items-center justify-center rounded-[7px] transition-colors"
              :class="
                isConfirmingBoxDelete(box)
                  ? 'w-[78px] gap-1.5 bg-red-600 px-2 text-[12px] font-medium text-white hover:bg-red-700 dark:bg-red-500 dark:hover:bg-red-600'
                  : 'w-8 text-red-500 hover:bg-[#fff0f0] hover:text-red-600 dark:text-red-400 dark:hover:bg-[#3a2528]'
              "
              :title="isConfirmingBoxDelete(box) ? '再次点击确认删除并执行当前文件夹处理策略' : '删除 Box'"
              type="button"
              @click="deleteBoxFromPanel(box)"
            >
              <Trash2 :size="15" />
              <span v-if="isConfirmingBoxDelete(box)">确认</span>
            </button>
          </span>
        </div>
      </SettingRow>

      <div v-if="boxes.length === 0" class="px-5 py-10 text-center">
        <p class="text-[13px] font-semibold text-[#202229] dark:text-[#f4f4f5]">还没有 Box</p>
        <p class="mt-1 text-[12px] text-[#707684] dark:text-[#9ca0aa]">点击上方新增按钮创建第一个真实文件夹 Box</p>
      </div>
    </SettingGroup>
  </SettingSection>
</template>
