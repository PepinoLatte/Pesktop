//! Explorer 原生右键菜单封装，命令层只负责提供窗口和屏幕坐标。

#[cfg(target_os = "windows")]
mod menu;
#[cfg(target_os = "windows")]
mod pidl;

use std::path::Path;

/// 在 Box 文件项上弹出 Windows Shell 原生右键菜单，菜单项和执行逻辑全部交给系统。
pub fn show_native_context_menu_for_path(
    window: &tauri::WebviewWindow,
    path: &Path,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    show_platform_native_context_menu_for_path(window, path, screen_x, screen_y)
}

/// 在 Shell 虚拟项上弹出 Windows 原生右键菜单，调用方传入 Shell 可解析的 `::{GUID}` 或路径。
pub fn show_native_context_menu_for_parsing_name(
    window: &tauri::WebviewWindow,
    parsing_name: &str,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    show_platform_native_context_menu_for_parsing_name(window, parsing_name, screen_x, screen_y)
}

/// 通过 `IContextMenu` 获取 Explorer 同源菜单，选中项用 Shell 返回的命令 ID 执行。
#[cfg(target_os = "windows")]
fn show_platform_native_context_menu_for_path(
    window: &tauri::WebviewWindow,
    path: &Path,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    let hwnd = window
        .hwnd()
        .map_err(|error| format!("无法获取窗口句柄：{error}"))?;
    let pidl = pidl::ShellPidl::from_path(path)?;

    unsafe { menu::show_context_menu_from_pidl(hwnd, pidl.as_ptr(), screen_x, screen_y) }
}

/// Shell 虚拟项同样通过解析名转 PIDL，再复用底层菜单创建逻辑。
#[cfg(target_os = "windows")]
fn show_platform_native_context_menu_for_parsing_name(
    window: &tauri::WebviewWindow,
    parsing_name: &str,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    let hwnd = window
        .hwnd()
        .map_err(|error| format!("无法获取窗口句柄：{error}"))?;
    let pidl = pidl::ShellPidl::from_parsing_name(parsing_name)?;

    unsafe { menu::show_context_menu_from_pidl(hwnd, pidl.as_ptr(), screen_x, screen_y) }
}

/// 非 Windows 平台没有 Explorer Shell 菜单，保持显式错误避免前端误以为已生效。
#[cfg(not(target_os = "windows"))]
fn show_platform_native_context_menu_for_path(
    _window: &tauri::WebviewWindow,
    _path: &Path,
    _screen_x: i32,
    _screen_y: i32,
) -> Result<(), String> {
    Err("当前平台暂不支持 Windows 原生右键菜单".to_string())
}

/// 非 Windows 平台没有 Explorer Shell 菜单，保持显式错误避免前端误以为已生效。
#[cfg(not(target_os = "windows"))]
fn show_platform_native_context_menu_for_parsing_name(
    _window: &tauri::WebviewWindow,
    _parsing_name: &str,
    _screen_x: i32,
    _screen_y: i32,
) -> Result<(), String> {
    Err("当前平台暂不支持 Windows 原生右键菜单".to_string())
}
