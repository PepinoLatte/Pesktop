import { onMounted, onUnmounted, ref } from "vue";
import type { Ref } from "vue";
import { emit } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalSize, PhysicalPosition } from "@tauri-apps/api/window";
import {
  listenBoxFileDrag,
  type BoxFileDragPreviewOptions,
} from "@/shared/ipc/boxFileDrag";
import type { DesktopItem } from "@/entities/desktopItem/types";
import { DRAG_PREVIEW_WINDOW_READY_EVENT } from "@/windows/dragPreview/lifecycle";
import {
  DRAG_PREVIEW_OFFSET,
  type DragPreviewWindowSize,
  resolvePreviewWindowSize,
} from "@/windows/dragPreview/model/previewLayout";

/**
 * 拖影窗口状态由全局文件拖拽 IPC 驱动，组件层只消费当前要展示的首个文件和数量。
 */
export interface DragPreviewWindowState {
  item: Readonly<Ref<DesktopItem | null>>;
  itemCount: Readonly<Ref<number>>;
  preview: Readonly<Ref<BoxFileDragPreviewOptions | null>>;
}

const currentWindow = getCurrentWindow();
const item = ref<DesktopItem | null>(null);
const itemCount = ref(0);
const preview = ref<BoxFileDragPreviewOptions | null>(null);
let unlistenDrag: UnlistenFn | null = null;
let lastPreviewWindowSize: DragPreviewWindowSize | null = null;

/**
 * 接管拖影窗口生命周期：监听拖拽广播、调整透明窗口尺寸、并让窗口跟随鼠标移动。
 */
export function useDragPreviewWindow(): DragPreviewWindowState {
  onMounted(async () => {
    unlistenDrag = await listenBoxFileDrag(async ({ payload }) => {
      if (payload.phase === "cancel" || payload.phase === "drop") {
        item.value = null;
        itemCount.value = 0;
        preview.value = null;
        await currentWindow.hide();
        return;
      }

      item.value = payload.item;
      itemCount.value = payload.paths.length;
      preview.value = payload.preview;
      await resizePreviewWindow(payload.preview);
      await currentWindow.setPosition(
        new PhysicalPosition(
          Math.round(payload.screenX + DRAG_PREVIEW_OFFSET.x),
          Math.round(payload.screenY + DRAG_PREVIEW_OFFSET.y),
        ),
      );
      await currentWindow.show();
    });
    await emit(DRAG_PREVIEW_WINDOW_READY_EVENT);
  });

  onUnmounted(() => {
    unlistenDrag?.();
  });

  return {
    item,
    itemCount,
    preview,
  };
}

/**
 * 仅在尺寸变化时调用原生窗口调整，减少拖动中不必要的窗口重排。
 */
async function resizePreviewWindow(nextPreview: BoxFileDragPreviewOptions): Promise<void> {
  const nextSize = resolvePreviewWindowSize(nextPreview);
  if (
    lastPreviewWindowSize &&
    lastPreviewWindowSize.height === nextSize.height &&
    lastPreviewWindowSize.width === nextSize.width
  ) {
    return;
  }

  lastPreviewWindowSize = nextSize;
  await currentWindow.setSize(new LogicalSize(nextSize.width, nextSize.height));
}
