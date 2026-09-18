//! Windows OLE DropTarget 封装，补齐透明 WebView 文件拖入不稳定的问题。

#[cfg(target_os = "windows")]
mod controller;
#[cfg(target_os = "windows")]
mod data_object;
#[cfg(target_os = "windows")]
mod drop_target;
#[cfg(target_os = "windows")]
mod emitter;
#[cfg(target_os = "windows")]
mod payload;
#[cfg(target_os = "windows")]
mod window;

#[cfg(target_os = "windows")]
pub use controller::{register_box_native_drop, unregister_box_native_drop};

/// 非 Windows 平台没有 Explorer OLE 拖放目标，命令保持幂等成功以简化前端生命周期。
#[cfg(not(target_os = "windows"))]
pub fn register_box_native_drop(
    _app: &tauri::AppHandle,
    _window_label: &str,
) -> Result<(), String> {
    Ok(())
}

/// 非 Windows 平台没有 Explorer OLE 拖放目标，命令保持幂等成功以简化前端生命周期。
#[cfg(not(target_os = "windows"))]
pub fn unregister_box_native_drop(
    _app: &tauri::AppHandle,
    _window_label: &str,
) -> Result<(), String> {
    Ok(())
}
