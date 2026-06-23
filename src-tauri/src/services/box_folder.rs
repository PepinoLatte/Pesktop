//! Box 文件夹服务编排收纳目录、拖入拖出、原生拖放注册等业务用例。

use crate::domain::box_policy::{BoxConflictPolicy, BoxDeletePolicy, BoxDropAction};
use crate::infrastructure::filesystem::box_folder as box_folder_fs;
use crate::infrastructure::windows::folder_dialog::{self, FolderDialogOwner};
use crate::infrastructure::windows::native_drop;
use crate::infrastructure::windows::shell_file::{self, ShellOperationOwner};
use tauri::AppHandle;

/// 打开系统文件夹选择器，选择后续新建 Box 使用的收纳根目录。
pub fn choose_collection_root_folder(owner: FolderDialogOwner) -> Result<Option<String>, String> {
    folder_dialog::choose_collection_root_folder(owner)
}

/// 在用户设置的收纳根目录下创建单个 Box 的真实文件夹。
pub fn create_box_folder(root_path: &str, folder_name: &str) -> Result<String, String> {
    box_folder_fs::create_box_folder(root_path, folder_name)
}

/// 按当前删除策略处理 Box 真实文件夹；成功后前端才会删除 Box 记录。
pub fn delete_box_folder(
    folder_path: &str,
    desktop_path: &str,
    policy: BoxDeletePolicy,
    conflict_policy: BoxConflictPolicy,
) -> Result<(), String> {
    box_folder_fs::delete_box_folder(folder_path, desktop_path, policy, conflict_policy)
}

/// 把单个 Box 的真实文件夹移动到当前收纳根目录，成功后返回新的文件夹路径。
pub fn migrate_box_folder(folder_path: &str, root_path: &str) -> Result<String, String> {
    box_folder_fs::migrate_box_folder(folder_path, root_path)
}

/// 用系统 Explorer 打开 Box 对应的真实文件夹。
pub fn open_box_folder(folder_path: &str) -> Result<(), String> {
    shell_file::open_folder_in_explorer(folder_path)
}

/// 按当前拖入策略把外部文件复制、移动或映射到 Box 真实文件夹。
pub fn handle_box_dropped_paths(
    folder_path: &str,
    paths: &[String],
    action: BoxDropAction,
    _owner: ShellOperationOwner,
    conflict_policy: BoxConflictPolicy,
) -> Result<Vec<String>, String> {
    box_folder_fs::handle_box_dropped_paths(folder_path, paths, action, conflict_policy)
}

/// 按当前拖出策略把 Box 内文件复制、移动或映射到 Windows 桌面目录。
pub fn handle_box_dragged_paths_to_desktop(
    desktop_path: &str,
    paths: &[String],
    action: BoxDropAction,
    _owner: ShellOperationOwner,
    conflict_policy: BoxConflictPolicy,
) -> Result<(), String> {
    box_folder_fs::handle_box_dragged_paths_to_desktop(desktop_path, paths, action, conflict_policy)
}

/// Box 窗口挂载后注册自定义 Windows DropTarget，补齐透明 WebView 文件拖入不稳定的问题。
pub fn register_box_native_drop(app: &AppHandle, window_label: &str) -> Result<(), String> {
    native_drop::register_box_native_drop(app, window_label)
}

/// Box 窗口卸载时注销自定义 DropTarget，避免旧窗口句柄继续接收拖放。
pub fn unregister_box_native_drop(app: &AppHandle, window_label: &str) -> Result<(), String> {
    native_drop::unregister_box_native_drop(app, window_label)
}
