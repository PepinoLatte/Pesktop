use crate::services::box_folder;
use tauri::AppHandle;

/// Box 窗口挂载后注册自定义 Windows DropTarget，补齐透明 WebView 文件拖入不稳定的问题。
#[tauri::command]
pub fn register_box_native_drop_target(app: AppHandle, window_label: String) -> Result<(), String> {
    if !window_label.starts_with("box_") {
        return Err("只有 Box 窗口可以注册原生拖放目标".to_string());
    }

    box_folder::register_box_native_drop(&app, &window_label)
}

/// Box 窗口卸载时注销自定义 DropTarget，避免旧窗口句柄继续接收拖放。
#[tauri::command]
pub fn unregister_box_native_drop_target(
    app: AppHandle,
    window_label: String,
) -> Result<(), String> {
    if !window_label.starts_with("box_") {
        return Ok(());
    }

    box_folder::unregister_box_native_drop(&app, &window_label)
}
