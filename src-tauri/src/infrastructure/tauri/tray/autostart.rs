//! 开机自启状态读写与 UI 同步。

use crate::domain::app::contract::{AUTOSTART_CHANGED_EVENT, SETTINGS_WINDOW_LABEL};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use super::state::AppTrayState;

/// 读取插件中的系统自启状态，供设置页和托盘初始化共享。
pub fn resolve_autostart_enabled(app: &AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| format!("无法读取开机自启状态：{error}"))
}

/// 写入系统自启状态并同步所有 UI 入口，避免设置页和托盘出现互相矛盾的勾选状态。
pub fn set_autostart_enabled(app: &AppHandle, enabled: bool) -> Result<bool, String> {
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch
            .enable()
            .map_err(|error| format!("无法开启开机自启：{error}"))?;
    } else {
        autolaunch
            .disable()
            .map_err(|error| format!("无法关闭开机自启：{error}"))?;
    }

    sync_autostart_state(app, enabled);
    Ok(enabled)
}

/// 同步托盘勾选状态和设置页开关状态，保证多个入口看到的系统状态一致。
pub(super) fn sync_autostart_state(app: &AppHandle, enabled: bool) {
    if let Some(tray_state) = app.try_state::<AppTrayState>() {
        if let Err(error) = tray_state.autostart_item.set_checked(enabled) {
            eprintln!("failed to sync tray autostart check state: {error}");
        }
    }
    if let Err(error) = app.emit_to(SETTINGS_WINDOW_LABEL, AUTOSTART_CHANGED_EVENT, enabled) {
        eprintln!("failed to emit autostart state change: {error}");
    }
}
