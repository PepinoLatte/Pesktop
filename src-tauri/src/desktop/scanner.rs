use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{DesktopItem, DesktopItemKind, DesktopSnapshot};

/// 扫描用户桌面目录，返回可用于自绘图标的轻量元信息。
pub fn scan_desktop() -> io::Result<DesktopSnapshot> {
    let desktop_path = resolve_desktop_path();
    let mut items = Vec::new();

    if desktop_path.exists() {
        for entry in fs::read_dir(&desktop_path)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = entry.metadata()?;

            items.push(DesktopItem {
                id: stable_item_id(&path),
                name: entry.file_name().to_string_lossy().to_string(),
                path: path.to_string_lossy().to_string(),
                extension: path.extension().map(|value| value.to_string_lossy().to_string()),
                kind: resolve_item_kind(&path, metadata.is_dir()),
            });
        }
    }

    items.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

    Ok(DesktopSnapshot {
        desktop_path: desktop_path.to_string_lossy().to_string(),
        items,
    })
}

/// 优先使用 Windows 用户桌面路径，拿不到用户目录时才退回当前目录保证命令可返回。
fn resolve_desktop_path() -> PathBuf {
    if let Ok(profile_path) = env::var("USERPROFILE") {
        return PathBuf::from(profile_path).join("Desktop");
    }

    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// 文件类型只用于选择前端图标，不参与真实文件的打开方式判断。
fn resolve_item_kind(path: &Path, is_dir: bool) -> DesktopItemKind {
    if is_dir {
        return DesktopItemKind::Folder;
    }

    match path.extension().and_then(|value| value.to_str()) {
        Some(extension) if extension.eq_ignore_ascii_case("lnk") => DesktopItemKind::Shortcut,
        Some(_) => DesktopItemKind::File,
        None => DesktopItemKind::Unknown,
    }
}

/// 基于完整路径生成稳定 ID，避免重名文件在前端列表中互相覆盖。
fn stable_item_id(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .map(|value| if value.is_ascii_alphanumeric() { value } else { '_' })
        .collect()
}
