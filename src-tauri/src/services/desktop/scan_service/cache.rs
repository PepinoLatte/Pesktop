//! Box 文件夹扫描缓存，复用未变化文件项的 Shell 图标。

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::domain::desktop::DesktopItem;

use super::entry::FileItemCacheSignature;

/// 全局缓存按 Box 文件夹路径分桶，每个桶只保存该目录的直接子项展示快照。
type FolderScanCaches = HashMap<String, FolderScanCache>;

/// Box 文件夹扫描缓存复用未变化文件项的 Shell 图标，避免兜底轮询反复触发昂贵的系统缩略图解析。
static BOX_FOLDER_SCAN_CACHE: OnceLock<Mutex<FolderScanCaches>> = OnceLock::new();

/// 单个 Box 文件夹的扫描缓存，以完整路径作为文件项稳定键。
#[derive(Default)]
pub(super) struct FolderScanCache {
    pub(super) items_by_path: HashMap<String, CachedDesktopItem>,
}

/// 缓存项保存展示模型和文件签名，签名变化时必须重新读取图标和展示字段。
pub(super) struct CachedDesktopItem {
    pub(super) item: DesktopItem,
    pub(super) signature: FileItemCacheSignature,
}

/// 获取全局扫描缓存，锁中毒时继续取回内部数据，避免一次 panic 让后续扫描全部失败。
pub(super) fn lock_box_folder_scan_cache() -> MutexGuard<'static, FolderScanCaches> {
    BOX_FOLDER_SCAN_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
