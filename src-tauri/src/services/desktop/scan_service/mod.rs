//! Box 文件夹扫描服务负责生成文件展示项、轻量 revision 和扫描缓存。

mod cache;
mod entry;
mod item_factory;
mod revision;

use crate::domain::desktop::{DesktopItem, DesktopSnapshot};
use crate::infrastructure::windows::common::desktop_path;

use cache::CachedDesktopItem;
use entry::BoxFolderScanEntry;

/// 缓存命中项直接进入排序，未命中项延后到锁外解析 Shell 图标。
enum BoxFolderScanResolution {
    Cached(DesktopItem),
    Pending(BoxFolderScanEntry),
}

/// 获取当前桌面路径快照，删除 Box 默认策略会把文件移回该目录。
pub fn get_desktop_snapshot() -> DesktopSnapshot {
    let desktop_path = desktop_path::resolve_desktop_path();

    DesktopSnapshot {
        desktop_path: desktop_path.to_string_lossy().to_string(),
    }
}

/// 扫描 Box 真实收纳文件夹的直接子项，供 WebView 自绘文件网格使用。
pub fn list_box_folder_items(folder_path: &str) -> Result<Vec<DesktopItem>, String> {
    scan_box_folder(folder_path).map_err(|error| error.to_string())
}

/// 计算 Box 文件夹轻量版本号，前端轮询用它判断是否需要拉取完整图标列表。
pub fn get_box_folder_revision(folder_path: &str) -> Result<String, String> {
    revision::calculate_box_folder_revision(folder_path).map_err(|error| error.to_string())
}

fn scan_box_folder(folder_path: &str) -> std::io::Result<Vec<DesktopItem>> {
    let folder = entry::resolve_existing_box_folder(folder_path)?;

    let folder_cache_key = entry::stable_path_key(&folder);
    let entries = entry::read_box_folder_entries(&folder)?;
    let mut resolutions = Vec::with_capacity(entries.len());
    {
        let mut scan_cache = cache::lock_box_folder_scan_cache();
        let folder_cache = scan_cache.entry(folder_cache_key.clone()).or_default();
        let present_path_keys = entries
            .iter()
            .map(|entry| entry.path_key.clone())
            .collect::<std::collections::HashSet<_>>();

        folder_cache
            .items_by_path
            .retain(|path_key, _| present_path_keys.contains(path_key));
        for entry in entries {
            if let Some(cached_item) = folder_cache
                .items_by_path
                .get(&entry.path_key)
                .filter(|cached_item| cached_item.signature == entry.signature)
            {
                resolutions.push(BoxFolderScanResolution::Cached(cached_item.item.clone()));
                continue;
            }

            resolutions.push(BoxFolderScanResolution::Pending(entry));
        }
    }

    let mut items = Vec::with_capacity(resolutions.len());
    let mut refreshed_items = Vec::new();
    for resolution in resolutions {
        match resolution {
            BoxFolderScanResolution::Cached(item) => items.push(item),
            BoxFolderScanResolution::Pending(entry) => {
                let item = item_factory::create_desktop_item(&entry.path, &entry.signature);
                refreshed_items.push((
                    entry.path_key,
                    CachedDesktopItem {
                        item: item.clone(),
                        signature: entry.signature,
                    },
                ));
                items.push(item);
            }
        }
    }
    if !refreshed_items.is_empty() {
        let mut scan_cache = cache::lock_box_folder_scan_cache();
        let folder_cache = scan_cache.entry(folder_cache_key).or_default();
        for (path_key, cached_item) in refreshed_items {
            folder_cache.items_by_path.insert(path_key, cached_item);
        }
    }
    items.sort_by(item_factory::compare_desktop_items);
    Ok(items)
}
