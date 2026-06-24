//! Box 文件项展示模型构建与排序逻辑。

use std::path::Path;

use crate::domain::desktop::{DesktopItem, DesktopItemKind, DesktopItemSource};
use crate::domain::filesystem::WINDOWS_SHORTCUT_EXTENSION;
use crate::infrastructure::windows::shell_icon;

use super::entry::FileItemCacheSignature;

/// 将真实文件系统路径转换成前端展示模型；打开、重命名和删除仍以后续路径操作为准。
pub(super) fn create_desktop_item(path: &Path, signature: &FileItemCacheSignature) -> DesktopItem {
    DesktopItem {
        extension: path
            .extension()
            .map(|value| value.to_string_lossy().to_string()),
        icon_data_url: shell_icon::resolve_item_icon_data_url(path),
        id: stable_item_id(path),
        kind: resolve_item_kind(path, signature.is_dir),
        name: resolve_item_name(path),
        path: path.to_string_lossy().to_string(),
        shell_id: None,
        source: DesktopItemSource::FileSystem,
    }
}

/// 文件夹优先、名称其次的排序接近 Explorer 默认直觉，避免刷新后项目位置随机跳动。
pub(super) fn compare_desktop_items(left: &DesktopItem, right: &DesktopItem) -> std::cmp::Ordering {
    let left_is_folder = matches!(left.kind, DesktopItemKind::Folder);
    let right_is_folder = matches!(right.kind, DesktopItemKind::Folder);

    right_is_folder
        .cmp(&left_is_folder)
        .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        .then_with(|| left.path.to_lowercase().cmp(&right.path.to_lowercase()))
}

/// 根目录这类路径没有 file_name，使用完整路径作为兜底展示名可以避免空白项目。
fn resolve_item_name(path: &Path) -> String {
    path.file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

/// 文件类型只用于选择前端后备图标，不参与真实文件的打开方式判断。
fn resolve_item_kind(path: &Path, is_dir: bool) -> DesktopItemKind {
    if is_dir {
        return DesktopItemKind::Folder;
    }

    match path.extension().and_then(|value| value.to_str()) {
        Some(extension) if extension.eq_ignore_ascii_case(WINDOWS_SHORTCUT_EXTENSION) => {
            DesktopItemKind::Shortcut
        }
        Some(_) => DesktopItemKind::File,
        None => DesktopItemKind::Unknown,
    }
}

/// 基于完整路径生成稳定 ID，避免重名文件在前端列表中互相覆盖。
fn stable_item_id(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() {
                value
            } else {
                '_'
            }
        })
        .collect()
}
