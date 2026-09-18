import { onUnmounted, ref, type Ref } from "vue";
import type { DesktopBox } from "./types";

/**
 * 二次删除确认只短暂停留，避免用户第一次误点后很久再点击造成非预期删除
 */
const DEFAULT_DELETE_CONFIRMATION_TIMEOUT_MS = 4000;

/**
 * 内联删除确认控制器暴露确认态查询和状态清理能力，调用方负责执行真实删除
 */
export interface DesktopBoxDeleteConfirmationController {
  clearBoxDeleteConfirmation: () => void;
  confirmingBoxId: Ref<string | null>;
  isConfirmingBoxDelete: (box: DesktopBox | string) => boolean;
  requestBoxDeleteConfirmation: (box: DesktopBox) => boolean;
}

/**
 * Box 删除确认状态统一在领域层维护，菜单和设置页可以共享同一套二段式交互语义
 */
export function useDesktopBoxDeleteConfirmation(
  timeoutMs = DEFAULT_DELETE_CONFIRMATION_TIMEOUT_MS,
): DesktopBoxDeleteConfirmationController {
  const confirmingBoxId = ref<string | null>(null);
  let confirmationTimer: ReturnType<typeof window.setTimeout> | null = null;

  /**
   * 判断某个 Box 是否处于等待第二次点击确认的状态，用于切换按钮文案和危险色
   */
  function isConfirmingBoxDelete(box: DesktopBox | string): boolean {
    const boxId = typeof box === "string" ? box : box.id;

    return confirmingBoxId.value === boxId;
  }

  /**
   * 第一次点击只进入确认态，第二次点击同一个 Box 才允许调用方执行真实删除
   */
  function requestBoxDeleteConfirmation(box: DesktopBox): boolean {
    if (isConfirmingBoxDelete(box)) {
      clearBoxDeleteConfirmation();
      return true;
    }

    confirmingBoxId.value = box.id;
    restartConfirmationTimer();
    return false;
  }

  /**
   * 主动清理确认态，用于菜单关闭、切换操作或组件卸载时避免保留过期危险状态
   */
  function clearBoxDeleteConfirmation(): void {
    confirmingBoxId.value = null;
    if (!confirmationTimer) {
      return;
    }

    window.clearTimeout(confirmationTimer);
    confirmationTimer = null;
  }

  /**
   * 每次新的第一次点击都重新计时，保证用户有固定时间完成第二次确认
   */
  function restartConfirmationTimer(): void {
    if (confirmationTimer) {
      window.clearTimeout(confirmationTimer);
    }

    confirmationTimer = window.setTimeout(() => {
      confirmingBoxId.value = null;
      confirmationTimer = null;
    }, timeoutMs);
  }

  onUnmounted(clearBoxDeleteConfirmation);

  return {
    clearBoxDeleteConfirmation,
    confirmingBoxId,
    isConfirmingBoxDelete,
    requestBoxDeleteConfirmation,
  };
}
