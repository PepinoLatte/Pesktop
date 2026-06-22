import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

/**
 * 拖影窗口只负责跟随鼠标显示当前拖动图标，不参与真实 Drop 命中和数据提交。
 */
const DRAG_PREVIEW_WINDOW_LABEL = "drag_preview";

/**
 * 拖影页 mounted 后主动回报 ready，确保首次拖拽事件不会早于预览页监听注册。
 */
export const DRAG_PREVIEW_WINDOW_READY_EVENT = "dasktop-drag-preview-ready";

/**
 * 拖影窗口初始尺寸只用于创建阶段，实际尺寸会在收到拖拽配置后由预览页动态调整。
 */
const DRAG_PREVIEW_INITIAL_WINDOW_SIZE = {
  height: 118,
  width: 132,
} as const;

/**
 * 创建或复用全局拖影窗口，并设置穿透鼠标，避免预览层挡住目标 Box 的命中判断。
 */
export async function openDragPreviewWindow(): Promise<WebviewWindow> {
  const existingWindow = await WebviewWindow.getByLabel(DRAG_PREVIEW_WINDOW_LABEL);
  if (existingWindow) {
    await prepareDragPreviewWindow(existingWindow);
    return existingWindow;
  }

  return new Promise<WebviewWindow>((resolve, reject) => {
    let previewWindow: WebviewWindow | null = null;
    let unlistenReady: UnlistenFn | null = null;
    const cleanupReadyListener = (): void => {
      unlistenReady?.();
      unlistenReady = null;
    };

    void listen(DRAG_PREVIEW_WINDOW_READY_EVENT, async () => {
      if (!previewWindow) {
        return;
      }

      cleanupReadyListener();
      await prepareDragPreviewWindow(previewWindow);
      resolve(previewWindow);
    })
      .then((unlisten) => {
        unlistenReady = unlisten;
        previewWindow = new WebviewWindow(DRAG_PREVIEW_WINDOW_LABEL, {
          alwaysOnTop: true,
          decorations: false,
          dragDropEnabled: false,
          focus: false,
          height: DRAG_PREVIEW_INITIAL_WINDOW_SIZE.height,
          resizable: false,
          shadow: false,
          skipTaskbar: true,
          title: "Dasktop Drag Preview",
          transparent: true,
          url: "/?dragPreview=1",
          visible: false,
          width: DRAG_PREVIEW_INITIAL_WINDOW_SIZE.width,
          x: 0,
          y: 0,
        });

        void previewWindow.once<unknown>("tauri://error", (error) => {
          cleanupReadyListener();
          reject(error.payload);
        });
      })
      .catch(reject);
  });
}

/**
 * 拖影窗口必须置顶且鼠标穿透，否则它会挡住 Box 的坐标命中和原生拖放目标。
 */
async function prepareDragPreviewWindow(previewWindow: WebviewWindow): Promise<void> {
  await previewWindow.setIgnoreCursorEvents(true);
  await previewWindow.setAlwaysOnTop(true);
}
