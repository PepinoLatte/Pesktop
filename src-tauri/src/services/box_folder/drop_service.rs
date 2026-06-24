//! Box 拖放服务，负责把前端拖拽意图编排为真实文件传输或原生 DropTarget 生命周期。

use crate::domain::desktop::{BoxConflictPolicy, BoxDropAction};
use crate::infrastructure::filesystem::box_folder as box_folder_fs;
use crate::infrastructure::windows::native_drop;
use crate::infrastructure::windows::shell_file::ShellOperationOwner;
use tauri::AppHandle;

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
