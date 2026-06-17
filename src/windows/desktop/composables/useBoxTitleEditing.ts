import { nextTick, ref, type ComponentPublicInstance, type ComputedRef } from "vue";
import type { DesktopBox } from "@/entities/desktopBox/types";

/**
 * 标题编辑组合式逻辑只处理 Box 显示名称，不触碰真实桌面文件名。
 */
export function useBoxTitleEditing(
  box: ComputedRef<DesktopBox | undefined>,
  updateBox: (box: DesktopBox) => Promise<void>,
  closeContextMenu: () => void,
) {
  const isEditingTitle = ref(false);
  const titleDraft = ref("");
  const titleInputRef = ref<HTMLInputElement | null>(null);

  /**
   * Box 标题双击进入编辑态，只修改 Dasktop 的分组名称，不重命名真实桌面文件。
   */
  function startTitleEditing(event: MouseEvent): void {
    event.stopPropagation();

    if (!box.value) {
      return;
    }

    closeContextMenu();
    titleDraft.value = box.value.title;
    isEditingTitle.value = true;
    void nextTick(() => {
      titleInputRef.value?.focus();
      titleInputRef.value?.select();
    });
  }

  /**
   * 保存标题时允许空文本，设置页会用兜底名称识别该 Box，不强迫用户显示标题。
   */
  async function commitTitleEditing(): Promise<void> {
    if (!box.value || !isEditingTitle.value) {
      return;
    }

    const nextTitle = titleDraft.value.trim();
    isEditingTitle.value = false;

    if (nextTitle === box.value.title) {
      return;
    }

    await updateBox({
      ...box.value,
      title: nextTitle,
    });
  }

  /**
   * 取消编辑只还原标题草稿，不触发数据库写入。
   */
  function cancelTitleEditing(): void {
    titleDraft.value = box.value?.title ?? "";
    isEditingTitle.value = false;
  }

  /**
   * 模板函数 ref 将真实输入框写回组合式逻辑，保证进入编辑态后的聚焦行为稳定可控。
   */
  function setTitleInputRef(element: Element | ComponentPublicInstance | null): void {
    titleInputRef.value = element instanceof HTMLInputElement ? element : null;
  }

  return {
    cancelTitleEditing,
    commitTitleEditing,
    isEditingTitle,
    setTitleInputRef,
    startTitleEditing,
    titleDraft,
  };
}
