use crate::desktop::{scan_desktop, scan_paths, DesktopItem, DesktopSnapshot};
use std::path::Path;
use tauri::AppHandle;

#[cfg(target_os = "windows")]
use windows::core::{PCSTR, PCWSTR};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_APARTMENTTHREADED};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
#[cfg(target_os = "windows")]
use windows::Win32::UI::Shell::{
    IContextMenu, IShellFolder, SHBindToParent, SHParseDisplayName, ShellExecuteW, CMF_NORMAL,
    CMINVOKECOMMANDINFO,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, TrackPopupMenu, SW_SHOWNORMAL, TPM_LEFTALIGN, TPM_RETURNCMD,
    TPM_RIGHTBUTTON,
};

/// 获取当前桌面文件快照，前端会基于这些真实文件自绘图标。
#[tauri::command]
pub fn get_desktop_snapshot() -> Result<DesktopSnapshot, String> {
    scan_desktop().map_err(|error| error.to_string())
}

/// 按路径解析任意磁盘文件元信息，让 Box 可以收纳桌面目录之外的项目。
#[tauri::command]
pub fn get_desktop_items_by_paths(paths: Vec<String>) -> Result<Vec<DesktopItem>, String> {
    scan_paths(&paths).map_err(|error| error.to_string())
}

/// 使用系统默认程序打开桌面项目，保持快捷方式、文件夹和普通文件与 Windows Explorer 一致。
#[tauri::command]
pub fn open_desktop_item(path: String) -> Result<(), String> {
    let item_path = Path::new(&path);
    if !item_path.exists() {
        return Err("桌面项目不存在，可能已经被移动或删除".to_string());
    }

    open_path_with_system_default(item_path)
}

/// 在 Box 图标上弹出 Windows Shell 原生右键菜单，菜单项和执行逻辑全部交给系统。
#[tauri::command]
pub fn show_native_item_context_menu(
    window: tauri::WebviewWindow,
    path: String,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    let item_path = Path::new(&path);
    if !item_path.exists() {
        return Err("桌面项目不存在，可能已经被移动或删除".to_string());
    }

    show_native_context_menu_for_path(&window, item_path, screen_x, screen_y)
}

/// 切换 Windows Explorer 原生桌面图标层可见性；真实文件和图标坐标不会被修改。
#[tauri::command]
pub fn set_native_desktop_icons_hidden(hidden: bool) -> Result<(), String> {
    crate::desktop::set_native_desktop_icons_hidden(hidden)
}

/// 读取系统开机自启状态；状态来源是官方 autostart 插件，不写入前端 SQLite 设置表。
#[tauri::command]
pub fn is_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    crate::app_tray::resolve_autostart_enabled(&app)
}

/// 切换系统开机自启状态，并同步托盘菜单与设置页开关。
#[tauri::command]
pub fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<bool, String> {
    crate::app_tray::set_autostart_enabled(&app, enabled)
}

/// 同步托盘菜单里的原生桌面图标隐藏勾选状态，设置值仍由前端 Store 持久化。
#[tauri::command]
pub fn set_tray_native_desktop_icons_hidden_checked(
    app: AppHandle,
    hidden: bool,
) -> Result<bool, String> {
    crate::app_tray::set_native_desktop_icons_hidden_checked(&app, hidden)
}

/// 读取系统级左键状态，跨 WebView 拖拽释放时不依赖当前窗口能否收到鼠标事件。
#[tauri::command]
pub fn is_primary_mouse_button_pressed() -> bool {
    is_left_mouse_button_pressed()
}

/// Windows 通过 GetAsyncKeyState 判断当前左键是否仍处于按下状态。
#[cfg(target_os = "windows")]
fn is_left_mouse_button_pressed() -> bool {
    unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 }
}

/// 非 Windows 平台没有当前产品目标里的原生桌面拖拽语义，保持未按下避免拖拽卡住。
#[cfg(not(target_os = "windows"))]
fn is_left_mouse_button_pressed() -> bool {
    false
}

/// Windows 侧通过 ShellExecute 交给系统默认打开方式，避免前端猜测文件类型。
#[cfg(target_os = "windows")]
fn open_path_with_system_default(path: &Path) -> Result<(), String> {
    let path_wide = path
        .to_string_lossy()
        .encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let operation_wide = "open\0".encode_utf16().collect::<Vec<_>>();
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation_wide.as_ptr()),
            PCWSTR(path_wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    if result.0 as isize <= 32 {
        return Err(format!(
            "系统无法打开该桌面项目，错误码 {}",
            result.0 as isize
        ));
    }

    Ok(())
}

/// 通过 `IContextMenu` 获取 Explorer 同源菜单，选中项用 Shell 返回的命令 ID 执行。
#[cfg(target_os = "windows")]
fn show_native_context_menu_for_path(
    window: &tauri::WebviewWindow,
    path: &Path,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    let hwnd = window
        .hwnd()
        .map_err(|error| format!("无法获取窗口句柄：{error}"))?;
    let wide_path = to_wide_path(path);
    let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        SHParseDisplayName(PCWSTR(wide_path.as_ptr()), None, &mut pidl, 0, None)
            .map_err(|error| format!("系统无法解析该桌面项目：{error}"))?;

        let result = show_context_menu_from_pidl(hwnd, pidl, screen_x, screen_y);
        CoTaskMemFree(Some(pidl as *const _));
        result
    }
}

/// 非 Windows 平台没有 Explorer Shell 菜单，保持显式错误避免前端误以为已生效。
#[cfg(not(target_os = "windows"))]
fn show_native_context_menu_for_path(
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
    hwnd: HWND,
    pidl: *mut ITEMIDLIST,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    let mut child_pidl: *mut ITEMIDLIST = std::ptr::null_mut();
    let parent_folder: IShellFolder = SHBindToParent(pidl, Some(&mut child_pidl))
        .map_err(|error| format!("系统无法绑定该桌面项目：{error}"))?;
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

/// Windows Shell API 接收 UTF-16 零结尾路径，打开和右键菜单命令共用此转换。
#[cfg(target_os = "windows")]
fn to_wide_path(path: &Path) -> Vec<u16> {
    path.to_string_lossy()
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

/// 非 Windows 平台暂不接管打开行为，避免产生和系统桌面语义不一致的兼容分支。
#[cfg(not(target_os = "windows"))]
fn open_path_with_system_default(_path: &Path) -> Result<(), String> {
    Err("当前平台暂不支持打开桌面项目".to_string())
}
