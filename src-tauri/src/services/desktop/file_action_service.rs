//! 桌面文件操作服务负责编排打开、菜单、重命名、删除和剪贴板粘贴。

use std::path::{Path, PathBuf};

use crate::domain::desktop::{BoxConflictPolicy, BoxDropAction, FileClipboardOperation};
use crate::infrastructure::filesystem::{box_folder as box_folder_fs, naming, transfer};
use crate::infrastructure::windows::{
    shell_clipboard, shell_context, shell_file, shell_virtual_item,
};

/// 桌面文件操作目标，统一区分 Shell 虚拟项和真实文件路径，避免各操作重复解析 path。
enum DesktopActionTarget<'a> {
    File(&'a str),
    Shell(&'a str),
}

/// 使用系统默认程序打开 Box 文件项，保持与 Explorer 双击一致。
pub fn open_desktop_item(path: &str) -> Result<(), String> {
    match resolve_desktop_action_target(path) {
        DesktopActionTarget::Shell(shell_id) => {
            shell_virtual_item::open_shell_virtual_item(shell_id)
        }
        DesktopActionTarget::File(item_path) => {
            shell_file::open_item_with_system_default(item_path)
        }
    }
}

/// 在 Box 文件项上弹出 Windows Shell 原生右键菜单，菜单项和执行逻辑全部交给系统。
pub fn show_native_item_context_menu(
    window: &tauri::WebviewWindow,
    path: &str,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    match resolve_desktop_action_target(path) {
        DesktopActionTarget::Shell(shell_id) => {
            shell_virtual_item::show_shell_virtual_item_context_menu(
                window, shell_id, screen_x, screen_y,
            )
        }
        DesktopActionTarget::File(item_path) => {
            let item_path = Path::new(item_path);
            if !item_path.exists() {
                return Err("文件项不存在，可能已经被移动或删除".to_string());
            }

            shell_context::show_native_context_menu_for_path(window, item_path, screen_x, screen_y)
        }
    }
}

/// 重命名 Box 文件项；真实路径变更后前端会重新扫描文件夹。
pub fn rename_desktop_item(path: &str, new_name: &str) -> Result<String, String> {
    box_folder_fs::rename_item(path, new_name)
}

/// 删除 Box 内选中文件项时走 Windows 回收站，避免误删后无法恢复。
pub fn delete_desktop_items(paths: &[String]) -> Result<(), String> {
    let Some(sources) = normalize_non_empty_paths(paths)? else {
        return Ok(());
    };

    shell_file::recycle_paths(&sources)
}

/// 将 Box 当前选区写入 Windows 文件剪贴板，支持后续跨 Box 或 Explorer 粘贴。
pub fn write_desktop_items_to_clipboard(
    paths: &[String],
    operation: FileClipboardOperation,
) -> Result<(), String> {
    let Some(sources) = normalize_non_empty_paths(paths)? else {
        return Ok(());
    };

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
    let action = if payload.operation.is_cut() {
        BoxDropAction::Move
    } else {
        BoxDropAction::Copy
    };
    let paths = payload
        .paths
        .iter()
        .map(|path| naming::display_path(path))
        .collect::<Vec<_>>();

    box_folder_fs::handle_box_dropped_paths(folder_path, &paths, action, conflict_policy)?;

    Ok(true)
}

fn resolve_desktop_action_target(path: &str) -> DesktopActionTarget<'_> {
    shell_virtual_item::strip_shell_item_path(path)
        .map(DesktopActionTarget::Shell)
        .unwrap_or(DesktopActionTarget::File(path))
}

/// 统一处理空选区，避免删除和剪贴板写入重复维护真实文件路径归一化分支。
fn normalize_non_empty_paths(paths: &[String]) -> Result<Option<Vec<PathBuf>>, String> {
    let sources = transfer::normalize_existing_paths(paths)?;

    Ok((!sources.is_empty()).then_some(sources))
}
