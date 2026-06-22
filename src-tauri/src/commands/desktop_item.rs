use crate::domain::box_policy::BoxConflictPolicy;
use crate::domain::desktop_item::{DesktopItem, DesktopSnapshot, FileClipboardOperation};
use crate::services::desktop_item;

/// 获取当前桌面路径快照，删除 Box 默认策略会把文件移回该目录。
#[tauri::command]
pub fn get_desktop_snapshot() -> Result<DesktopSnapshot, String> {
    desktop_item::get_desktop_snapshot()
}

/// 扫描 Box 真实文件夹内容，前端用返回结果自绘文件网格。
#[tauri::command]
pub fn list_box_folder_items(folder_path: String) -> Result<Vec<DesktopItem>, String> {
    desktop_item::list_box_folder_items(&folder_path)
}

/// 使用系统默认程序打开 Box 文件项，保持与 Explorer 双击一致。
#[tauri::command]
pub fn open_desktop_item(path: String) -> Result<(), String> {
    desktop_item::open_desktop_item(&path)
}

/// 在 Box 文件项上弹出 Windows Shell 原生右键菜单，菜单项和执行逻辑全部交给系统。
#[tauri::command]
pub fn show_native_item_context_menu(
    window: tauri::WebviewWindow,
    path: String,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    desktop_item::show_native_item_context_menu(&window, &path, screen_x, screen_y)
}

/// 重命名 Box 文件项；真实路径变更后前端会重新扫描文件夹。
#[tauri::command]
pub fn rename_desktop_item(path: String, new_name: String) -> Result<String, String> {
    desktop_item::rename_desktop_item(&path, &new_name)
}

/// 删除 Box 文件项时放入回收站，避免自绘文件区绕过系统恢复能力。
#[tauri::command]
pub fn delete_desktop_items(paths: Vec<String>) -> Result<(), String> {
    desktop_item::delete_desktop_items(&paths)
}

/// 将 Box 文件选区写入 Windows 文件剪贴板，保留复制或剪切意图供后续粘贴使用。
#[tauri::command]
pub fn write_desktop_items_to_clipboard(
    paths: Vec<String>,
    operation: String,
) -> Result<(), String> {
    desktop_item::write_desktop_items_to_clipboard(
        &paths,
        FileClipboardOperation::from_str(&operation)?,
    )
}

/// 从 Windows 文件剪贴板粘贴到当前 Box，未读取到文件列表时返回 false。
#[tauri::command]
pub fn paste_desktop_items_from_clipboard(
    folder_path: String,
    conflict_policy: String,
) -> Result<bool, String> {
    desktop_item::paste_desktop_items_from_clipboard(
        &folder_path,
        BoxConflictPolicy::from_str(&conflict_policy)?,
    )
}
