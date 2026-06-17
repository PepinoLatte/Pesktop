<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { Eye, Lock, RefreshCw, Settings, Trash2, Unlock } from "@lucide/vue";
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
import { closeBoxWindow, openSettingsWindow } from "@/entities/desktopBox/windows";

const currentWindow = getCurrentWindow();
const desktopStore = useDesktopStore();
const activeBoxId = ref<string | null>(null);
const isMenuRendered = ref(false);
const menuRef = ref<HTMLElement | null>(null);
const unlistenFns: UnlistenFn[] = [];
let menuAnimation: ReturnType<typeof animate> | null = null;
let menuAnimationVersion = 0;
let lastBlurCloseAt = 0;
let activeOpenRequestId = "";

type BoxAutoCollapseMode = "always" | "rollup";

/**
 * Box 菜单里的标题位置使用紧凑分段控件，选项属于单个 Box 的窗口偏好。
 */
const BOX_TITLE_POSITION_OPTIONS: Array<{
  label: string;
  value: DesktopBoxTitlePosition;
}> = [
  { label: "上方", value: "top" },
  { label: "下方", value: "bottom" },
];

/**
 * 菜单中的分段控件使用固定列宽，防止不同主题字体下文字把菜单撑宽。
 */
const BOX_MENU_SEGMENT_WIDTH = {
  collapse: 54,
  titlePosition: 54,
} as const;

/**
 * 是否自动收起属于当前 Box 自身状态，用开关语义降低配置理解成本。
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
 * 自动收起分段控件使用字符串值承载 UI 状态，落库时再转换为 Box 的布尔字段。
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
 * 销毁时清理跨窗口监听和动画句柄，避免开发热更新后重复响应菜单事件。
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
 * Escape 是系统菜单的常见关闭方式，给键盘用户保留一致退路。
 */
function handleKeyDown(event: KeyboardEvent): void {
  if (event.key === "Escape") {
    void closeAnimated();
  }
}

/**
 * 隐藏窗口里先渲染菜单并写入动画首帧，避免原生窗口 show 时闪出旧内容。
 */
async function prepareMenuOpen(boxId: string, requestId: string): Promise<void> {
  activeOpenRequestId = requestId;
  activeBoxId.value = boxId;
  isMenuRendered.value = true;
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
 * 菜单打开只播放 motion 进入动画，窗口创建和首帧准备已在 hidden 阶段完成。
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
    // motion 在被新动画打断时会拒绝 finished，版本号会阻止旧动画继续收尾。
  } finally {
    if (activeAnimationVersion === menuAnimationVersion) {
      menuAnimation = null;
    }
  }
}

/**
 * 菜单关闭先播放 motion 退出动画，再隐藏预载窗口，后续打开可以复用同一个 WebView。
 */
async function closeAnimated(requestedBoxId?: string): Promise<void> {
  const closingBoxId = activeBoxId.value;
  if (requestedBoxId && closingBoxId && requestedBoxId !== closingBoxId) {
    return;
  }

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
    // 退出动画被新的打开动作打断时不再隐藏窗口，避免快速切换时闪烁。
  } finally {
    await finishCloseAnimation(activeAnimationVersion);
  }
}

/**
 * 关闭动画收尾必须检查版本号，避免旧的退出动画覆盖新的菜单打开状态。
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
 * 减少动态效果时直接写入终态，既保留可见性语义，也遵守系统辅助功能设置。
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
 * 动效统一通过 motion 承载，但系统要求减少动态效果时仍需要跳过过渡。
 */
function shouldReduceMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/**
 * 菜单失焦可能由更多按钮二次点击触发，父窗口据此避免把这次 click 当成重新打开。
 */
function resolveCloseReason(): "blur" | "request" {
  return performance.now() - lastBlurCloseAt <= 120 ? "blur" : "request";
}

/**
 * Box 菜单只负责唤起设置页，具体配置仍由主窗口统一承载。
 */
async function openSettingsFromMenu(): Promise<void> {
  void closeAnimated();
  await openSettingsWindow();
}

/**
 * 刷新只重新读取桌面目录，不改变任何 Box 布局和真实文件位置。
 */
async function refreshDesktopFromMenu(): Promise<void> {
  void closeAnimated();
  await desktopStore.refreshSnapshot();
}

/**
 * Box 标题位置从菜单直接切换，适合用户在整理时即时调整窗口布局。
 */
async function updateTitlePositionFromMenu(position: DesktopBoxTitlePosition): Promise<void> {
  if (!box.value) {
    return;
  }

  await desktopStore.updateBoxTitlePosition(box.value.id, position);
}

/**
 * 自动收起属于当前 Box 的桌面整理习惯，落库后由对应 Box 窗口自行同步窗口高度。
 */
async function updateBoxAutoCollapseFromMenu(mode: BoxAutoCollapseMode): Promise<void> {
  if (!box.value) {
    return;
  }

  await desktopStore.updateBoxCollapsed(box.value.id, mode === "rollup");
}

/**
 * 锁定只冻结当前 Box 的几何操作，不影响内部图标打开、排序和右键。
 */
async function toggleBoxLockedFromMenu(): Promise<void> {
  if (!box.value) {
    return;
  }

  await desktopStore.updateBoxLocked(box.value.id, !box.value.locked);
}

/**
 * 闲置可见度从菜单滑块实时保存，0 表示未 hover 时整个 Box 隐形，hover 后仍恢复可见。
 */
async function updateIdleOpacityFromMenu(event: Event): Promise<void> {
  if (!box.value) {
    return;
  }

  const nextOpacity = Number((event.target as HTMLInputElement).value);
  await desktopStore.updateBoxTitleOpacity(box.value.id, nextOpacity);
}

/**
 * 删除 Box 只删除分组窗口和映射，真实桌面文件继续交给 Windows 管理。
 */
async function deleteCurrentBox(): Promise<void> {
  if (!box.value) {
    return;
  }

  const targetBoxId = box.value.id;
  void closeAnimated();
  await desktopStore.deleteBox(targetBoxId);
  await closeBoxWindow(targetBoxId);
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
      <div class="grid grid-cols-2 gap-1">
        <button
          class="flex h-8 items-center justify-center gap-1.5 rounded-[8px] px-2 text-[12px] font-medium transition-colors hover:bg-[#eceef3] dark:hover:bg-[#2b2e37]"
          type="button"
          @click="openSettingsFromMenu"
        >
          <Settings class="text-slate-500 dark:text-slate-400" :size="15" />
          设置
        </button>
        <button
          class="flex h-8 items-center justify-center gap-1.5 rounded-[8px] px-2 text-[12px] font-medium transition-colors hover:bg-[#eceef3] dark:hover:bg-[#2b2e37]"
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
          class="flex min-h-8 items-center rounded-[7px] px-2 text-left text-[12px] transition-colors hover:bg-[#eceef3] dark:hover:bg-[#2b2e37]"
          type="button"
          @click="toggleBoxLockedFromMenu"
        >
          <Lock v-if="box.locked" class="mr-2 text-slate-500 dark:text-slate-400" :size="14" />
          <Unlock v-else class="mr-2 text-slate-500 dark:text-slate-400" :size="14" />
          <span class="grid min-w-0 flex-1">
            <span class="truncate">{{ box.locked ? "解除锁定" : "锁定布局" }}</span>
            <span class="text-[10px] text-slate-500 dark:text-slate-400">
              {{ box.locked ? "当前位置不可移动缩放" : "允许移动和缩放" }}
            </span>
          </span>
        </button>
      </div>
      <div class="grid gap-1.5 px-1.5 py-0.5">
        <div class="flex items-center justify-between gap-3 text-[11px] font-medium text-slate-500 dark:text-slate-400">
          <span class="inline-flex items-center">
            <Eye class="mr-1.5" :size="13" />
            闲置可见度
          </span>
          <span class="shrink-0 tabular-nums">{{ box.titleOpacity }}%</span>
        </div>
        <input
          class="h-4 w-full accent-[#2f6bff]"
          :max="BOX_TITLE_OPACITY.max"
          :min="BOX_TITLE_OPACITY.min"
          :step="BOX_TITLE_OPACITY.step"
          type="range"
          :value="box.titleOpacity"
          @input="updateIdleOpacityFromMenu"
        />
      </div>
      <span class="my-0.5 h-px bg-[#e4e6eb] dark:bg-[#30333c]" />
      <button
        class="flex h-8 items-center rounded-[7px] px-2.5 text-left text-[12px] text-red-600 transition-colors hover:bg-[#fff0f0] dark:text-red-400 dark:hover:bg-[#3a2528]"
        type="button"
        @click="deleteCurrentBox"
      >
        <Trash2 class="mr-2" :size="14" />
        删除 Box
      </button>
    </nav>
  </main>
</template>
