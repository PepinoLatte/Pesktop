use crate::domain::desktop::{
    BoxConflictPolicy, DesktopItem, DesktopSnapshot, FileClipboardOperation,
};
use crate::services::desktop::{file_action_service, query_service, shell_icon_service};

/// 获取当前桌面路径快照，删除 Box 默认策略会把文件移回该目录。
#[tauri::command]
pub fn get_desktop_snapshot() -> Result<DesktopSnapshot, String> {
    Ok(query_service::get_desktop_snapshot())
}

/// 扫描 Box 真实文件夹内容，前端用返回结果自绘文件网格。
#[tauri::command]
pub fn list_box_folder_items(folder_path: String) -> Result<Vec<DesktopItem>, String> {
    query_service::list_box_folder_items(&folder_path)
}

/// 读取 Box 文件夹轻量版本号，前端轮询用它避免无变化时重复传输图标列表。
#[tauri::command]
pub fn get_box_folder_revision(folder_path: String) -> Result<String, String> {
    query_service::get_box_folder_revision(&folder_path)
}

/// 根据 Box 保存的 Shell 虚拟项 ID 生成展示模型，供系统桌面图标进入文件网格。
#[tauri::command]
pub fn list_shell_desktop_items(shell_ids: Vec<String>) -> Result<Vec<DesktopItem>, String> {
    Ok(query_service::list_shell_desktop_items(&shell_ids))
}

/// 读取 Windows 原生桌面上某个系统图标当前是否显示，用于记录 Dasktop 接管前状态。
#[tauri::command]
pub fn get_shell_desktop_icon_visible(shell_id: String) -> Result<bool, String> {
    shell_icon_service::get_shell_desktop_icon_visible(&shell_id)
}

/// 设置 Windows 原生桌面上某个系统图标是否显示，Box 内虚拟项引用不受影响。
#[tauri::command]
pub fn set_shell_desktop_icon_visible(shell_id: String, visible: bool) -> Result<(), String> {
    shell_icon_service::set_shell_desktop_icon_visible(&shell_id, visible)
}

/// 使用系统默认程序打开 Box 文件项，保持与 Explorer 双击一致。
#[tauri::command]
pub fn open_desktop_item(path: String) -> Result<(), String> {
    file_action_service::open_desktop_item(&path)
}

/// 在 Box 文件项上弹出 Windows Shell 原生右键菜单，菜单项和执行逻辑全部交给系统。
#[tauri::command]
pub fn show_native_item_context_menu(
    window: tauri::WebviewWindow,
    path: String,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    file_action_service::show_native_item_context_menu(&window, &path, screen_x, screen_y)
}

/// 重命名 Box 文件项；真实路径变更后前端会重新扫描文件夹。
#[tauri::command]
pub fn rename_desktop_item(path: String, new_name: String) -> Result<String, String> {
    file_action_service::rename_desktop_item(&path, &new_name)
}

/// 删除 Box 文件项时放入回收站，避免自绘文件区绕过系统恢复能力。
#[tauri::command]
pub fn delete_desktop_items(paths: Vec<String>) -> Result<(), String> {
    file_action_service::delete_desktop_items(&paths)
}

/// 将 Box 文件选区写入 Windows 文件剪贴板，保留复制或剪切意图供后续粘贴使用。
#[tauri::command]
pub fn write_desktop_items_to_clipboard(
    paths: Vec<String>,
    operation: String,
) -> Result<(), String> {
    file_action_service::write_desktop_items_to_clipboard(
        &paths,
        operation.parse::<FileClipboardOperation>()?,
    )
}

/// 从 Windows 文件剪贴板粘贴到当前 Box，未读取到文件列表时返回 false。
#[tauri::command]
pub fn paste_desktop_items_from_clipboard(
    folder_path: String,
    conflict_policy: String,
) -> Result<bool, String> {
    file_action_service::paste_desktop_items_from_clipboard(
        &folder_path,
        conflict_policy.parse::<BoxConflictPolicy>()?,
    )
}
