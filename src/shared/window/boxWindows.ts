import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalPosition, LogicalSize } from "@tauri-apps/api/window";
import type { DesktopBox } from "../types/desktop";

/**
 * 打开 Box 的策略参数，启动恢复时不抢焦点，用户主动打开时再切到前台。
 */
export interface OpenBoxWindowOptions {
  focus?: boolean;
}

/**
 * Box 窗口标签统一加前缀，便于 capability 使用 `box_*` 授权动态窗口。
 */
export function boxWindowLabel(boxId: string): string {
  return `box_${boxId.replace(/-/g, "_")}`;
}

/**
 * 每个 Box 都创建独立 WebView 窗口，桌面空白区域不被全屏透明层截获。
 */
export async function openBoxWindow(
  box: DesktopBox,
  options: OpenBoxWindowOptions = {},
): Promise<void> {
  const label = boxWindowLabel(box.id);
  const existingWindow = await WebviewWindow.getByLabel(label);
  const shouldFocus = options.focus ?? true;

  if (existingWindow) {
    await existingWindow.setPosition(new LogicalPosition(box.x, box.y));
    await existingWindow.setSize(new LogicalSize(box.width, box.height));
    await existingWindow.show();
    if (shouldFocus) {
      await existingWindow.setFocus();
    }
    return;
  }

  await new Promise<void>((resolve, reject) => {
    const window = new WebviewWindow(label, {
      url: `/?boxId=${encodeURIComponent(box.id)}`,
      title: box.title,
      x: box.x,
      y: box.y,
      width: box.width,
      height: box.height,
      minWidth: 240,
      minHeight: 184,
      decorations: false,
      dragDropEnabled: true,
      focus: shouldFocus,
      transparent: true,
      shadow: false,
      resizable: true,
      skipTaskbar: true,
      preventOverflow: true,
      visible: true,
    });

    void window.once("tauri://created", async () => {
      if (!shouldFocus) {
        await window.show();
      }
      resolve();
    });
    void window.once<unknown>("tauri://error", (error) => reject(error.payload));
  });
}

/**
 * 设置页由 main 窗口承载，唤起时先恢复最小化状态，确保 Box 菜单能把设置窗真正带回前台。
 */
export async function openSettingsWindow(): Promise<void> {
  const existingMainWindow = await WebviewWindow.getByLabel("main");
  if (existingMainWindow) {
    await existingMainWindow.unminimize();
    await existingMainWindow.show();
    await existingMainWindow.setFocus();
    return;
  }
}
