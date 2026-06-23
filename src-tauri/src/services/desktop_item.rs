//! 桌面文件项服务负责 Box 文件夹扫描、文件项操作和 Explorer 原生菜单用例。

use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use crate::domain::box_policy::{BoxConflictPolicy, BoxDropAction};
use crate::domain::desktop_item::FileClipboardOperation;
use crate::domain::desktop_item::{
    DesktopItem, DesktopItemKind, DesktopItemSource, DesktopSnapshot,
};
use crate::infrastructure::filesystem::{box_folder as box_folder_fs, transfer};
use crate::infrastructure::windows::{
    desktop_path, shell_clipboard, shell_context, shell_desktop_icon_visibility, shell_file,
    shell_icon, shell_virtual_item,
};

/// 获取当前桌面路径快照，删除 Box 默认策略会把文件移回该目录。
pub fn get_desktop_snapshot() -> Result<DesktopSnapshot, String> {
    let desktop_path = desktop_path::resolve_desktop_path();

    Ok(DesktopSnapshot {
        desktop_path: desktop_path.to_string_lossy().to_string(),
    })
}

/// 扫描 Box 真实收纳文件夹的直接子项，供 WebView 自绘文件网格使用。
pub fn list_box_folder_items(folder_path: &str) -> Result<Vec<DesktopItem>, String> {
    scan_box_folder(folder_path).map_err(|error| error.to_string())
}

/// 根据 Box 持久化的 Shell ID 重建系统桌面图标展示项，未知项会被过滤掉。
pub fn list_shell_desktop_items(shell_ids: &[String]) -> Result<Vec<DesktopItem>, String> {
    Ok(shell_virtual_item::list_shell_virtual_desktop_items(
        shell_ids,
    ))
}

/// 读取 Windows 原生桌面上某个系统图标是否显示，供 Dasktop 接管前记录用户原始状态。
pub fn get_shell_desktop_icon_visible(shell_id: &str) -> Result<bool, String> {
    shell_desktop_icon_visibility::get_shell_desktop_icon_visible(shell_id)
}

/// 设置 Windows 原生桌面上某个系统图标显示状态，Box 内引用仍由前端数据库独立维护。
pub fn set_shell_desktop_icon_visible(shell_id: &str, visible: bool) -> Result<(), String> {
    shell_desktop_icon_visibility::set_shell_desktop_icon_visible(shell_id, visible)
}

/// 使用系统默认程序打开 Box 文件项，保持与 Explorer 双击一致。
pub fn open_desktop_item(path: &str) -> Result<(), String> {
    if let Some(shell_id) = shell_virtual_item::strip_shell_item_path(path) {
        return shell_virtual_item::open_shell_virtual_item(shell_id);
    }

    shell_file::open_item_with_system_default(path)
}

/// 在 Box 文件项上弹出 Windows Shell 原生右键菜单，菜单项和执行逻辑全部交给系统。
pub fn show_native_item_context_menu(
    window: &tauri::WebviewWindow,
    path: &str,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    if let Some(shell_id) = shell_virtual_item::strip_shell_item_path(path) {
        return shell_virtual_item::show_shell_virtual_item_context_menu(
            window, shell_id, screen_x, screen_y,
        );
    }

    let item_path = Path::new(path);
    if !item_path.exists() {
        return Err("文件项不存在，可能已经被移动或删除".to_string());
    }

    shell_context::show_native_context_menu_for_path(window, item_path, screen_x, screen_y)
}

/// 重命名 Box 文件项；真实路径变更后前端会重新扫描文件夹。
pub fn rename_desktop_item(path: &str, new_name: &str) -> Result<String, String> {
    box_folder_fs::rename_item(path, new_name)
}

/// 删除 Box 内选中文件项时走 Windows 回收站，避免误删后无法恢复。
pub fn delete_desktop_items(paths: &[String]) -> Result<(), String> {
    let sources = transfer::normalize_existing_paths(paths)?;
    if sources.is_empty() {
        return Ok(());
    }

    shell_file::recycle_paths(&sources)
}

/// 将 Box 当前选区写入 Windows 文件剪贴板，支持后续跨 Box 或 Explorer 粘贴。
pub fn write_desktop_items_to_clipboard(
    paths: &[String],
    operation: FileClipboardOperation,
) -> Result<(), String> {
    let sources = transfer::normalize_existing_paths(paths)?;
    if sources.is_empty() {
        return Ok(());
    }

    shell_clipboard::write_file_list(&sources, operation)
}

/// 从 Windows 文件剪贴板读取路径并粘贴到当前 Box，返回是否执行了文件传输。
pub fn paste_desktop_items_from_clipboard(
    folder_path: &str,
    conflict_policy: BoxConflictPolicy,
) -> Result<bool, String> {
    let Some(payload) = shell_clipboard::read_file_list()? else {
        return Ok(false);
    };
    let action = match payload.operation {
        FileClipboardOperation::Copy => BoxDropAction::Copy,
        FileClipboardOperation::Cut => BoxDropAction::Move,
    };
    let paths = payload
        .paths
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    box_folder_fs::handle_box_dropped_paths(folder_path, &paths, action, conflict_policy)?;

    Ok(true)
}

fn scan_box_folder(folder_path: &str) -> io::Result<Vec<DesktopItem>> {
    let folder = PathBuf::from(folder_path);
    if !folder.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Box folder does not exist",
        ));
    }

    let mut items = Vec::new();
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        items.push(create_desktop_item(&entry.path())?);
    }

    items.sort_by(compare_desktop_items);
    Ok(items)
}

/// 将真实文件系统路径转换成前端展示模型；打开、重命名和删除仍以后续路径操作为准。
fn create_desktop_item(path: &Path) -> io::Result<DesktopItem> {
    let metadata = fs::metadata(path)?;
    let is_dir = metadata.is_dir();

    Ok(DesktopItem {
        extension: path
            .extension()
            .map(|value| value.to_string_lossy().to_string()),
        icon_data_url: shell_icon::resolve_item_icon_data_url(path),
        id: stable_item_id(path),
        kind: resolve_item_kind(path, is_dir),
        name: resolve_item_name(path),
        path: path.to_string_lossy().to_string(),
        shell_id: None,
        source: DesktopItemSource::FileSystem,
    })
}

/// 文件夹优先、名称其次的排序接近 Explorer 默认直觉，避免刷新后项目位置随机跳动。
fn compare_desktop_items(left: &DesktopItem, right: &DesktopItem) -> std::cmp::Ordering {
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
        Some(extension) if extension.eq_ignore_ascii_case("lnk") => DesktopItemKind::Shortcut,
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
