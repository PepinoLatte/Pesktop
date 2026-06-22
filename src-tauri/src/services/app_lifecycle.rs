//! 应用生命周期服务负责把命令层的请求转交给 Tauri 外设，并隐藏托盘同步细节。

use crate::infrastructure::tauri::tray;
use tauri::AppHandle;

/// 读取插件中的系统自启状态，供设置页命令使用。
pub fn resolve_autostart_enabled(app: &AppHandle) -> Result<bool, String> {
    tray::resolve_autostart_enabled(app)
}

/// 写入系统自启状态并同步所有 UI 入口，避免设置页和托盘出现互相矛盾的勾选状态。
pub fn set_autostart_enabled(app: &AppHandle, enabled: bool) -> Result<bool, String> {
    tray::set_autostart_enabled(app, enabled)
}
