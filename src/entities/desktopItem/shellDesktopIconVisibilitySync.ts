import {
  getShellDesktopIconVisible,
  setShellDesktopIconVisible,
} from "@/entities/desktopItem/api";
import {
  deleteShellIconVisibilityRecord,
  loadAllBoxVirtualItemIds,
  loadShellIconVisibilityRecords,
  saveShellIconVisibilityRecord,
  type ShellIconVisibilityRecord,
} from "@/shared/storage/database";

const MANAGED_SHELL_DESKTOP_ICON_IDS = [
  "this-pc",
  "recycle-bin",
  "network",
  "control-panel",
  "user-folder",
] as const;

const managedShellDesktopIconIdSet = new Set<string>(MANAGED_SHELL_DESKTOP_ICON_IDS);

/**
 * 同步配置用于区分正常自动隐藏、关闭总开关后的强制恢复和启动阶段兜底同步。
 */
interface SyncShellDesktopIconVisibilityOptions {
  autoHideEnabled?: boolean;
  forceRestoreManagedIcons?: boolean;
}

/**
 * 按所有 Box 的 Shell 虚拟项引用同步 Windows 原生系统桌面图标显示状态。
 */
export async function syncShellDesktopIconVisibility(
  options: SyncShellDesktopIconVisibilityOptions = {},
): Promise<void> {
  const [referencedShellIds, records] = await Promise.all([
    loadReferencedManagedShellIds(),
    loadShellIconVisibilityRecords(),
  ]);
  const autoHideEnabled = options.autoHideEnabled ?? true;
  if (!autoHideEnabled || options.forceRestoreManagedIcons) {
    await restoreManagedShellIcons(records);
    return;
  }

  const referencedShellIdSet = new Set(referencedShellIds);
  const recordByShellId = new Map(records.map((record) => [record.shellId, record]));
  const shellIdsToSync = dedupePreservingOrder([
    ...referencedShellIds,
    ...records.map((record) => record.shellId),
  ]).filter(isManagedShellDesktopIconId);

  for (const shellId of shellIdsToSync) {
    if (referencedShellIdSet.has(shellId)) {
      await hideReferencedShellIcon(shellId, recordByShellId.get(shellId));
    } else {
      await restoreUnreferencedShellIcon(recordByShellId.get(shellId));
    }
  }
}

/**
 * 读取所有 Box 的系统图标引用并去重，普通未知 Shell ID 不参与 Windows 原生图标显示控制。
 */
async function loadReferencedManagedShellIds(): Promise<string[]> {
  const shellIds = await loadAllBoxVirtualItemIds();
  return dedupePreservingOrder(shellIds).filter(isManagedShellDesktopIconId);
}

/**
 * 被任意 Box 引用的系统图标必须保持隐藏；首次接管前先记录 Windows 当前显示状态。
 */
async function hideReferencedShellIcon(
  shellId: string,
  record: ShellIconVisibilityRecord | undefined,
): Promise<void> {
  const previousVisible = record?.previousVisible ?? (await getShellDesktopIconVisible(shellId));
  await setShellDesktopIconVisible(shellId, false);
  await saveShellIconVisibilityRecord({
    managedHidden: true,
    previousVisible,
    shellId,
  });
}

/**
 * 所有 Box 都不再引用该系统图标时，按 Dasktop 接管前状态恢复显示或隐藏。
 */
async function restoreUnreferencedShellIcon(
  record: ShellIconVisibilityRecord | undefined,
): Promise<void> {
  if (!record) {
    return;
  }

  if (record.managedHidden) {
    await setShellDesktopIconVisible(record.shellId, record.previousVisible);
  }
  await deleteShellIconVisibilityRecord(record.shellId);
}

/**
 * 总开关关闭时释放所有 Dasktop 自动隐藏记录，但保留用户原本隐藏的系统图标状态。
 */
async function restoreManagedShellIcons(records: ShellIconVisibilityRecord[]): Promise<void> {
  for (const record of records) {
    if (!isManagedShellDesktopIconId(record.shellId)) {
      await deleteShellIconVisibilityRecord(record.shellId);
      continue;
    }

    await restoreUnreferencedShellIcon(record);
  }
}

/**
 * 同步层只管理当前版本明确支持的系统桌面图标，避免把未来未知 Shell 项误写入注册表。
 */
function isManagedShellDesktopIconId(shellId: string): boolean {
  return managedShellDesktopIconIdSet.has(shellId);
}

/**
 * 多个 Box 可以引用同一个系统图标，内部同步只需要每个 ID 处理一次。
 */
function dedupePreservingOrder(values: string[]): string[] {
  const seenValues = new Set<string>();
  const result: string[] = [];

  for (const value of values) {
    if (seenValues.has(value)) {
      continue;
    }

    seenValues.add(value);
    result.push(value);
  }

  return result;
}
