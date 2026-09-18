//! Shell 虚拟项展示缓存，避免轮询时重复解析系统图标和用户文件夹路径。

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::domain::desktop::DesktopItem;

use super::create_shell_desktop_item;

/// Shell 虚拟项进程内展示缓存，避免 Box 文件轮询时重复解析系统图标和用户文件夹路径。
static SHELL_VIRTUAL_ITEM_CACHE: OnceLock<Mutex<HashMap<String, DesktopItem>>> = OnceLock::new();

/// 读取 Shell 虚拟项展示模型；已知项在当前进程生命周期内稳定，可复用图标 data URL 降低轮询成本。
pub(super) fn resolve_cached_shell_desktop_item(shell_id: &str) -> Option<DesktopItem> {
    {
        let cache = lock_shell_virtual_item_cache();
        if let Some(item) = cache.get(shell_id) {
            return Some(item.clone());
        }
    }

    let item = create_shell_desktop_item(shell_id)?;
    lock_shell_virtual_item_cache().insert(shell_id.to_string(), item.clone());
    Some(item)
}

/// 获取 Shell 虚拟项缓存；锁中毒时继续复用内部数据，保证展示降级不影响文件操作链路。
fn lock_shell_virtual_item_cache() -> MutexGuard<'static, HashMap<String, DesktopItem>> {
    SHELL_VIRTUAL_ITEM_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
