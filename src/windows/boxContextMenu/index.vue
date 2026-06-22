<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import {
  Eye,
  FolderOpen,
  FolderPlus,
  Lock,
  RefreshCw,
  Settings,
  Trash2,
  Unlock,
} from "@lucide/vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { animate } from "motion";
import SegmentedControl from "@/shared/ui/SegmentedControl.vue";
import { useDesktopStore } from "@/entities/desktopBox/store";
import { BOX_CONTEXT_MENU_LAYOUT, BOX_TITLE_OPACITY } from "@/entities/desktopBox/layout";
import type { DesktopBoxTitlePosition } from "@/entities/desktopBox/types";
import {
  listenBoxContextMenuClose,
  listenBoxContextMenuOpen,
  notifyBoxContextMenuPrepared,
  notifyBoxContextMenuReady,
  notifyBoxContextMenuState,
} from "@/shared/ipc/boxContextMenu";
import { useDesktopBoxDeleteConfirmation } from "@/entities/desktopBox/deleteConfirmation";
import { openBoxFolder } from "@/entities/desktopBox/api";
import { closeBoxWindow, openBoxWindow, openSettingsWindow } from "@/entities/desktopBox/windows";

const currentWindow = getCurrentWindow();
const desktopStore = useDesktopStore();
const activeBoxId = ref<string | null>(null);
const isMenuRendered = ref(false);
const menuRef = ref<HTMLElement | null>(null);
const unlistenFns: UnlistenFn[] = [];
const {
  clearBoxDeleteConfirmation,
  isConfirmingBoxDelete,
  requestBoxDeleteConfirmation,
} = useDesktopBoxDeleteConfirmation();
let menuAnimation: ReturnType<typeof animate> | null = null;
let menuAnimationVersion = 0;
let lastBlurCloseAt = 0;
let activeOpenRequestId = "";

type BoxAutoCollapseMode = "always" | "rollup";

/**
 * Box 菜单里的标题位置使用紧凑分段控件，选项属于单个 Box 的窗口偏好
 */
const BOX_TITLE_POSITION_OPTIONS: Array<{
  label: string;
  value: DesktopBoxTitlePosition;
}> = [
  { label: "上方", value: "top" },
  { label: "下方", value: "bottom" },
];

/**
 * 菜单中的分段控件使用固定列宽，防止不同主题字体下文字把菜单撑宽
 */
const BOX_MENU_SEGMENT_WIDTH = {
  collapse: 54,
  titlePosition: 54,
} as const;

/**
 * 更多菜单的操作行统一使用同一信息结构：图标槽、标题说明、右侧状态，降低视觉割裂感
 */
const BOX_MENU_ACTION_ROW_CLASS =
  "flex min-h-[44px] w-full items-center gap-2 rounded-[8px] px-2 text-left text-[12px] transition-colors hover:bg-[#eef1f6] focus-visible:bg-[#eef1f6] dark:hover:bg-[#2a2d36] dark:focus-visible:bg-[#2a2d36]";

/**
 * 是否自动收起属于当前 Box 自身状态，用开关语义降低配置理解成本
 */
const BOX_AUTO_COLLAPSE_OPTIONS: Array<{
  label: string;
  value: BoxAutoCollapseMode;
}> = [
  { label: "关闭", value: "always" },
  { label: "开启", value: "rollup" },
];

const box = computed(() =>
  activeBoxId.value
    ? desktopStore.boxes.find((item) => item.id === activeBoxId.value)
    : undefined,
);

/**
 * 自动收起分段控件使用字符串值承载 UI 状态，落库时再转换为 Box 的布尔字段
 */
const boxAutoCollapseMode = computed<BoxAutoCollapseMode>(() =>
  box.value?.collapsed ? "rollup" : "always",
);

onMounted(async () => {
  await desktopStore.initializeBoxMenu();

  unlistenFns.push(
    await currentWindow.onFocusChanged(({ payload }) => {
      if (!payload) {
        lastBlurCloseAt = performance.now();
        void closeAnimated();
      }
    }),
  );

  unlistenFns.push(
    await listenBoxContextMenuClose(({ payload }) => {
      void closeAnimated(payload.boxId);
    }),
  );

  unlistenFns.push(
    await listenBoxContextMenuOpen(({ payload }) => {
      if (payload.stage === "hidden") {
        void prepareMenuOpen(payload.boxId, payload.requestId);
        return;
      }

      void openAnimated(payload.boxId, payload.requestId);
    }),
  );

  window.addEventListener("keydown", handleKeyDown);
  await notifyBoxContextMenuReady();
});

/**
 * 销毁时清理跨窗口监听和动画句柄，避免开发热更新后重复响应菜单事件
 */
onUnmounted(() => {
  menuAnimation?.stop();
  menuAnimation = null;
  window.removeEventListener("keydown", handleKeyDown);
  for (const unlisten of unlistenFns) {
    unlisten();
  }
});

/**
 * Escape 是系统菜单的常见关闭方式，给键盘用户保留一致退路
 */
function handleKeyDown(event: KeyboardEvent): void {
  if (event.key === "Escape") {
    void closeAnimated();
  }
}

/**
 * 隐藏窗口里先渲染菜单并写入动画首帧，避免原生窗口 show 时闪出旧内容
 */
async function prepareMenuOpen(boxId: string, requestId: string): Promise<void> {
  activeOpenRequestId = requestId;
  activeBoxId.value = boxId;
  isMenuRendered.value = true;
  clearBoxDeleteConfirmation();
  menuAnimationVersion += 1;

  await nextTick();
  if (!box.value) {
    await closeAnimated(boxId);
    return;
  }

  const menuElement = menuRef.value;
  menuAnimation?.stop();
  menuAnimation = null;
  if (menuElement) {
    applyMenuVisibleState(0, "translateY(-6px) scale(0.98)");
  }
  await notifyBoxContextMenuPrepared(requestId);
}

/**
 * 菜单打开只播放 motion 进入动画，窗口创建和首帧准备已在 hidden 阶段完成
 */
async function openAnimated(boxId: string, requestId: string): Promise<void> {
  if (activeOpenRequestId !== requestId) {
    return;
  }

  activeBoxId.value = boxId;
  isMenuRendered.value = true;
  menuAnimationVersion += 1;
  const activeAnimationVersion = menuAnimationVersion;

  await nextTick();
  if (!box.value) {
    await closeAnimated(boxId);
    return;
  }

  await notifyBoxContextMenuState({ boxId, isOpen: true });
  const menuElement = menuRef.value;
  if (!menuElement || shouldReduceMotion()) {
    applyMenuVisibleState(1, "translateY(0) scale(1)");
    return;
  }

  menuAnimation?.stop();
  applyMenuVisibleState(0, "translateY(-6px) scale(0.98)");
  menuAnimation = animate(
    menuElement,
    {
      opacity: 1,
      transform: "translateY(0) scale(1)",
    },
    {
      duration: BOX_CONTEXT_MENU_LAYOUT.openAnimationMs / 1000,
      ease: [0.2, 0.8, 0.2, 1],
    },
  );

  try {
    await menuAnimation.finished;
  } catch {
    // motion 在被新动画打断时会拒绝 finished，版本号会阻止旧动画继续收尾
  } finally {
    if (activeAnimationVersion === menuAnimationVersion) {
      menuAnimation = null;
    }
  }
}

/**
 * 菜单关闭先播放 motion 退出动画，再隐藏预载窗口，后续打开可以复用同一个 WebView
 */
async function closeAnimated(requestedBoxId?: string): Promise<void> {
  const closingBoxId = activeBoxId.value;
  if (requestedBoxId && closingBoxId && requestedBoxId !== closingBoxId) {
    return;
  }

  clearBoxDeleteConfirmation();
  if (!closingBoxId && !isMenuRendered.value) {
    await currentWindow.hide();
    return;
  }

  menuAnimationVersion += 1;
  const activeAnimationVersion = menuAnimationVersion;
  if (closingBoxId) {
    await notifyBoxContextMenuState({
      boxId: closingBoxId,
      isOpen: false,
      reason: resolveCloseReason(),
    });
  }

  const menuElement = menuRef.value;
  menuAnimation?.stop();
  if (!menuElement || shouldReduceMotion()) {
    await finishCloseAnimation(activeAnimationVersion);
    return;
  }

  menuAnimation = animate(
    menuElement,
    {
      opacity: 0,
      transform: "translateY(-4px) scale(0.98)",
    },
    {
      duration: BOX_CONTEXT_MENU_LAYOUT.closeAnimationMs / 1000,
      ease: [0.4, 0, 1, 1],
    },
  );

  try {
    await menuAnimation.finished;
  } catch {
    // 退出动画被新的打开动作打断时不再隐藏窗口，避免快速切换时闪烁
  } finally {
    await finishCloseAnimation(activeAnimationVersion);
  }
}

/**
 * 关闭动画收尾必须检查版本号，避免旧的退出动画覆盖新的菜单打开状态
 */
async function finishCloseAnimation(activeAnimationVersion: number): Promise<void> {
  if (activeAnimationVersion !== menuAnimationVersion) {
    return;
  }

  menuAnimation = null;
  activeBoxId.value = null;
  isMenuRendered.value = false;
  await currentWindow.hide();
}

/**
 * 减少动态效果时直接写入终态，既保留可见性语义，也遵守系统辅助功能设置
 */
function applyMenuVisibleState(opacity: number, transform: string): void {
  const menuElement = menuRef.value;
  if (!menuElement) {
    return;
  }

  menuElement.style.opacity = String(opacity);
  menuElement.style.transform = transform;
}

/**
 * 动效统一通过 motion 承载，但系统要求减少动态效果时仍需要跳过过渡
 */
function shouldReduceMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/**
 * 菜单失焦可能由更多按钮二次点击触发，父窗口据此避免把这次 click 当成重新打开
 */
function resolveCloseReason(): "blur" | "request" {
  return performance.now() - lastBlurCloseAt <= 120 ? "blur" : "request";
}

/**
 * Box 菜单只负责唤起设置页，具体配置仍由主窗口统一承载
 */
async function openSettingsFromMenu(): Promise<void> {
  void closeAnimated();
  await openSettingsWindow();
}

/**
 * 新增 Box 复用桌面 Store 和窗口打开逻辑，确保菜单入口与设置页创建行为一致
 */
async function createBoxFromMenu(): Promise<void> {
  clearBoxDeleteConfirmation();
  void closeAnimated();
  try {
    const createdBox = await desktopStore.createBox();

    await openBoxWindow(createdBox, { focus: true });
  } catch (error) {
    desktopStore.lastError = error instanceof Error ? error.message : String(error);
  }
}

/**
 * 刷新只重新读取桌面目录，不改变任何 Box 布局和真实文件位置
 */
async function refreshDesktopFromMenu(): Promise<void> {
  void closeAnimated();
  await desktopStore.refreshSnapshot();
}

/**
 * 菜单里的打开文件夹入口直接交给系统 Explorer，便于用户跳出 Box 检查真实目录
 */
async function openFolderFromMenu(): Promise<void> {
  if (!box.value) {
    return;
  }

  const folderPath = box.value.folderPath;
  void closeAnimated();
  await openBoxFolder(folderPath).catch((error) => {
    desktopStore.lastError = error instanceof Error ? error.message : String(error);
  });
}

/**
 * Box 标题位置从菜单直接切换，适合用户在整理时即时调整窗口布局
 */
async function updateTitlePositionFromMenu(position: DesktopBoxTitlePosition): Promise<void> {
  if (!box.value) {
    return;
  }

  clearBoxDeleteConfirmation();
  await desktopStore.updateBoxTitlePosition(box.value.id, position);
}

/**
 * 自动收起属于当前 Box 的桌面整理习惯，落库后由对应 Box 窗口自行同步窗口高度
 */
async function updateBoxAutoCollapseFromMenu(mode: BoxAutoCollapseMode): Promise<void> {
  if (!box.value) {
    return;
  }

  clearBoxDeleteConfirmation();
  await desktopStore.updateBoxCollapsed(box.value.id, mode === "rollup");
}

/**
 * 锁定只冻结当前 Box 的几何操作，不影响内部图标打开、排序和右键
 */
async function toggleBoxLockedFromMenu(): Promise<void> {
  if (!box.value) {
    return;
  }

  clearBoxDeleteConfirmation();
  await desktopStore.updateBoxLocked(box.value.id, !box.value.locked);
}

/**
 * 闲置可见度从菜单滑块实时保存，0 表示未 hover 时整个 Box 隐形，hover 后仍恢复可见
 */
async function updateIdleOpacityFromMenu(event: Event): Promise<void> {
  if (!box.value) {
    return;
  }

  clearBoxDeleteConfirmation();
  const nextOpacity = Number((event.target as HTMLInputElement).value);
  await desktopStore.updateBoxTitleOpacity(box.value.id, nextOpacity);
}

/**
 * 删除 Box 前先按当前删除策略处理真实文件夹，用户取消 Shell 操作时保留 Box
 */
async function deleteCurrentBox(): Promise<void> {
  if (!box.value) {
    return;
  }

  const targetBoxId = box.value.id;
  if (!requestBoxDeleteConfirmation(box.value)) {
    return;
  }

  void closeAnimated();
  try {
    await desktopStore.deleteBox(targetBoxId);
    await closeBoxWindow(targetBoxId);
  } catch (error) {
    desktopStore.lastError = error instanceof Error ? error.message : String(error);
  }
}
</script>

<template>
  <main class="h-screen w-screen overflow-hidden bg-transparent p-0">
    <nav
      v-if="box && isMenuRendered"
      ref="menuRef"
      aria-label="Box 更多菜单"
      class="dasktop-box-menu grid h-full w-full gap-1 overflow-hidden rounded-[10px] border border-[#d9dce3] bg-[#fbfbfd] p-1.5 text-slate-800 shadow-[0_18px_45px_rgba(15,23,42,0.24)] dark:border-[#30333c] dark:bg-[#202228] dark:text-slate-100"
      @click.stop
    >
      <div class="grid grid-cols-3 gap-1">
        <button
          class="flex items-center justify-center gap-1 rounded-[8px] px-1.5 py-1 text-[12px] font-medium transition-colors hover:bg-[#eceef3] dark:hover:bg-[#2b2e37]"
          type="button"
          @click="openSettingsFromMenu"
        >
          <Settings class="text-slate-500 dark:text-slate-400" :size="15" />
          设置
        </button>
        <button
          aria-label="新增 Box"
          class="flex items-center justify-center gap-1 rounded-[8px] px-1.5 py-1 text-[12px] font-medium transition-colors hover:bg-[#eceef3] dark:hover:bg-[#2b2e37]"
          title="新增 Box"
          type="button"
          @click="createBoxFromMenu"
        >
          <FolderPlus class="text-slate-500 dark:text-slate-400" :size="15" />
          新增
        </button>
        <button
          class="flex items-center justify-center gap-1 rounded-[8px] px-1.5 py-1 text-[12px] font-medium transition-colors hover:bg-[#eceef3] dark:hover:bg-[#2b2e37]"
          type="button"
          @click="refreshDesktopFromMenu"
        >
          <RefreshCw class="text-slate-500 dark:text-slate-400" :size="15" />
          刷新
        </button>
      </div>
      <span class="my-0.5 h-px bg-[#e4e6eb] dark:bg-[#30333c]" />
      <div class="grid grid-cols-[58px_1fr] items-center gap-2 px-1 py-0.5">
        <span class="text-[11px] font-medium text-slate-500 dark:text-slate-400">标题位置</span>
        <SegmentedControl
          density="compact"
          :model-value="box.titlePosition"
          :option-width-px="BOX_MENU_SEGMENT_WIDTH.titlePosition"
          :options="BOX_TITLE_POSITION_OPTIONS"
          @change="updateTitlePositionFromMenu"
        />
      </div>
      <div class="grid grid-cols-[58px_1fr] items-center gap-2 px-1 py-0.5">
        <span class="text-[11px] font-medium text-slate-500 dark:text-slate-400">自动收起</span>
        <SegmentedControl
          density="compact"
          :model-value="boxAutoCollapseMode"
          :option-width-px="BOX_MENU_SEGMENT_WIDTH.collapse"
          :options="BOX_AUTO_COLLAPSE_OPTIONS"
          @change="updateBoxAutoCollapseFromMenu"
        />
      </div>
      <span class="my-0.5 h-px bg-[#e4e6eb] dark:bg-[#30333c]" />
      <div class="grid gap-1 px-1">
        <button
          :class="BOX_MENU_ACTION_ROW_CLASS"
          type="button"
          @click="openFolderFromMenu"
        >
          <span class="grid size-7 shrink-0 place-items-center rounded-[7px] bg-[#e9edf5] text-slate-600 dark:bg-[#30343e] dark:text-slate-300">
            <FolderOpen :size="15" />
          </span>
          <span class="grid min-w-0 flex-1 gap-0.5">
            <span class="truncate text-[12px] font-semibold text-slate-800 dark:text-slate-100">打开真实文件夹</span>
            <span class="truncate text-[10px] leading-3 text-slate-500 dark:text-slate-400">在 Explorer 中查看内容</span>
          </span>
          <span class="rounded-[6px] bg-white px-1.5 py-0.5 text-[10px] font-medium text-slate-500 shadow-[inset_0_0_0_1px_rgba(148,163,184,0.35)] dark:bg-[#242730] dark:text-slate-300 dark:shadow-[inset_0_0_0_1px_rgba(100,116,139,0.35)]">打开</span>
        </button>
        <button
          :class="BOX_MENU_ACTION_ROW_CLASS"
          type="button"
          @click="toggleBoxLockedFromMenu"
        >
          <span class="grid size-7 shrink-0 place-items-center rounded-[7px] bg-[#e9edf5] text-slate-600 dark:bg-[#30343e] dark:text-slate-300">
            <Lock v-if="box.locked" :size="15" />
            <Unlock v-else :size="15" />
          </span>
          <span class="grid min-w-0 flex-1 gap-0.5">
            <span class="truncate text-[12px] font-semibold text-slate-800 dark:text-slate-100">{{ box.locked ? "解除锁定" : "锁定布局" }}</span>
            <span class="truncate text-[10px] leading-3 text-slate-500 dark:text-slate-400">
              {{ box.locked ? "当前位置不可移动缩放" : "允许移动和缩放" }}
            </span>
          </span>
          <span
            class="rounded-[6px] px-1.5 py-0.5 text-[10px] font-medium"
            :class="box.locked ? 'bg-[#fff2d8] text-[#8a5a00] dark:bg-[#3a2f1f] dark:text-[#f5c76b]' : 'bg-[#e9f8ef] text-[#237447] dark:bg-[#1f3528] dark:text-[#7bd59f]'"
          >
            {{ box.locked ? "已锁定" : "可移动" }}
          </span>
        </button>
        <div class="rounded-[8px] px-2 py-2 transition-colors hover:bg-[#eef1f6] dark:hover:bg-[#2a2d36]">
          <div class="flex items-center gap-2">
            <span class="grid size-7 shrink-0 place-items-center rounded-[7px] bg-[#e9edf5] text-slate-600 dark:bg-[#30343e] dark:text-slate-300">
              <Eye :size="15" />
            </span>
            <span class="grid min-w-0 flex-1 gap-0.5">
              <span class="truncate text-[12px] font-semibold text-slate-800 dark:text-slate-100">闲置可见度</span>
              <span class="truncate text-[10px] leading-3 text-slate-500 dark:text-slate-400">鼠标离开后的 Box 透明度</span>
            </span>
            <span class="rounded-[6px] bg-white px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-slate-500 shadow-[inset_0_0_0_1px_rgba(148,163,184,0.35)] dark:bg-[#242730] dark:text-slate-300 dark:shadow-[inset_0_0_0_1px_rgba(100,116,139,0.35)]">{{ box.titleOpacity }}%</span>
          </div>
          <input
            class="mt-2 h-4 w-full accent-[#2f6bff]"
            :max="BOX_TITLE_OPACITY.max"
            :min="BOX_TITLE_OPACITY.min"
            :step="BOX_TITLE_OPACITY.step"
            type="range"
            :value="box.titleOpacity"
            @input="updateIdleOpacityFromMenu"
          />
        </div>
      </div>
      <span class="my-0.5 h-px bg-[#e4e6eb] dark:bg-[#30333c]" />
      <button
        :aria-label="isConfirmingBoxDelete(box) ? '确认删除 Box' : '删除 Box'"
        class="flex items-center rounded-[7px] px-2.5 py-1 text-left text-[12px] transition-colors"
        :class="
          isConfirmingBoxDelete(box)
            ? 'bg-red-600 text-white hover:bg-red-700 dark:bg-red-500 dark:text-white dark:hover:bg-red-600'
            : 'text-red-600 hover:bg-[#fff0f0] dark:text-red-400 dark:hover:bg-[#3a2528]'
        "
        :title="isConfirmingBoxDelete(box) ? '再次点击确认删除并执行当前文件夹处理策略' : '删除 Box'"
        type="button"
        @click="deleteCurrentBox"
      >
        <Trash2 class="mr-2" :size="14" />
        {{ isConfirmingBoxDelete(box) ? "确认删除" : "删除 Box" }}
      </button>
    </nav>
  </main>
</template>
