use crate::infrastructure::tauri::frontend_dev;
use crate::infrastructure::windows::window_style;
use crate::services::app::lifecycle;
use tauri::AppHandle;

/// 查询当前是否以外置前端资源模式运行；前端据此决定动态窗口走内嵌协议还是 daskhot 协议。
#[tauri::command]
pub fn is_external_frontend() -> bool {
    frontend_dev::is_external_frontend_enabled()
}

/// 把调用方窗口标记为工具窗口：不进 Alt+Tab，Win+D 显示桌面时保持原位不被最小化。
#[tauri::command]
pub fn apply_desktop_toolbox(window: tauri::WebviewWindow) -> Result<(), String> {
    let hwnd = window
        .hwnd()
        .map_err(|error| format!("获取窗口句柄失败：{error}"))?;
    window_style::set_toolwindow_ex_style(hwnd.0 as isize)
}

/// 读取系统开机自启状态；状态来源是官方 autostart 插件，不写入前端 SQLite 设置表。
#[tauri::command]
pub fn is_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    lifecycle::resolve_autostart_enabled(&app)
}

/// 切换系统开机自启状态，并同步托盘菜单与设置页开关。
#[tauri::command]
pub fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<bool, String> {
    lifecycle::set_autostart_enabled(&app, enabled)
}

/// 读取系统级左键状态，跨 WebView 拖拽释放时不依赖当前窗口能否收到鼠标事件。
#[tauri::command]
pub fn is_primary_mouse_button_pressed() -> bool {
    lifecycle::is_primary_mouse_button_pressed()
}
