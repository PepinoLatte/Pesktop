//! Explorer 原生右键菜单封装，命令层只负责提供窗口和屏幕坐标。

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

/// 通过 `IContextMenu` 获取 Explorer 同源菜单，选中项用 Shell 返回的命令 ID 执行。
#[cfg(target_os = "windows")]
fn show_platform_native_context_menu_for_path(
    window: &tauri::WebviewWindow,
    path: &Path,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::Common::ITEMIDLIST;
    use windows::Win32::UI::Shell::SHParseDisplayName;

    let hwnd = window
        .hwnd()
        .map_err(|error| format!("无法获取窗口句柄：{error}"))?;
    let wide_path = to_wide_path(path);
    let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        SHParseDisplayName(PCWSTR(wide_path.as_ptr()), None, &mut pidl, 0, None)
            .map_err(|error| format!("系统无法解析该文件项：{error}"))?;

        let result = show_context_menu_from_pidl(hwnd, pidl, screen_x, screen_y);
        CoTaskMemFree(Some(pidl as *const _));
        result
    }
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

/// 基于 PIDL 绑定父级 ShellFolder，再查询单个子项的上下文菜单。
#[cfg(target_os = "windows")]
unsafe fn show_context_menu_from_pidl(
    hwnd: windows::Win32::Foundation::HWND,
    pidl: *mut windows::Win32::UI::Shell::Common::ITEMIDLIST,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    use windows::core::PCSTR;
    use windows::Win32::UI::Shell::Common::ITEMIDLIST;
    use windows::Win32::UI::Shell::{
        IContextMenu, IShellFolder, SHBindToParent, CMF_NORMAL, CMINVOKECOMMANDINFO,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreatePopupMenu, DestroyMenu, TrackPopupMenu, SW_SHOWNORMAL, TPM_LEFTALIGN, TPM_RETURNCMD,
        TPM_RIGHTBUTTON,
    };

    let mut child_pidl: *mut ITEMIDLIST = std::ptr::null_mut();
    let parent_folder: IShellFolder = SHBindToParent(pidl, Some(&mut child_pidl))
        .map_err(|error| format!("系统无法绑定该文件项：{error}"))?;
    let context_menu: IContextMenu = parent_folder
        .GetUIObjectOf(hwnd, &[child_pidl as *const ITEMIDLIST], None)
        .map_err(|error| format!("系统无法创建原生右键菜单：{error}"))?;
    let menu = CreatePopupMenu().map_err(|error| format!("系统无法创建菜单：{error}"))?;
    let query_result = context_menu.QueryContextMenu(menu, 0, 1, 0x7fff, CMF_NORMAL);

    if query_result.is_err() {
        let _ = DestroyMenu(menu);
        return Err(format!("系统无法填充原生右键菜单：{query_result:?}"));
    }

    let selected_command = TrackPopupMenu(
        menu,
        TPM_LEFTALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD,
        screen_x,
        screen_y,
        None,
        hwnd,
        None,
    )
    .0;
    let destroy_result = DestroyMenu(menu);
    if let Err(error) = destroy_result {
        return Err(format!("系统菜单资源释放失败：{error}"));
    }

    if selected_command == 0 {
        return Ok(());
    }

    let command_offset = selected_command.saturating_sub(1) as usize;
    let invoke_info = CMINVOKECOMMANDINFO {
        cbSize: std::mem::size_of::<CMINVOKECOMMANDINFO>() as u32,
        hwnd,
        lpVerb: PCSTR(command_offset as *const u8),
        nShow: SW_SHOWNORMAL.0,
        ..CMINVOKECOMMANDINFO::default()
    };

    context_menu
        .InvokeCommand(&invoke_info)
        .map_err(|error| format!("系统无法执行该菜单命令：{error}"))
}

/// Windows Shell API 接收 UTF-16 零结尾路径，右键菜单命令复用此转换。
#[cfg(target_os = "windows")]
fn to_wide_path(path: &Path) -> Vec<u16> {
    path.to_string_lossy()
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
