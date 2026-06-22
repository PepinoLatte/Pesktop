use crate::desktop::{
    scan_box_folder, scan_desktop, BoxConflictPolicy, BoxDeletePolicy, BoxDropAction, DesktopItem,
    DesktopSnapshot,
};
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
    IContextMenu, IShellFolder, SHBindToParent, SHParseDisplayName, CMF_NORMAL, CMINVOKECOMMANDINFO,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, TrackPopupMenu, SW_SHOWNORMAL, TPM_LEFTALIGN, TPM_RETURNCMD,
    TPM_RIGHTBUTTON,
};

/// 获取当前桌面路径快照，删除 Box 默认策略会把文件移回该目录
#[tauri::command]
pub fn get_desktop_snapshot() -> Result<DesktopSnapshot, String> {
    scan_desktop().map_err(|error| error.to_string())
}

/// 扫描 Box 真实文件夹内容，前端用返回结果自绘文件网格
#[tauri::command]
pub fn list_box_folder_items(folder_path: String) -> Result<Vec<DesktopItem>, String> {
    scan_box_folder(&folder_path).map_err(|error| error.to_string())
}

/// 打开系统文件夹选择器，选择后续新建 Box 使用的收纳根目录
#[tauri::command]
pub fn choose_collection_root_folder(
    window: tauri::WebviewWindow,
) -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let hwnd = window
            .hwnd()
            .map_err(|error| format!("无法获取设置窗口句柄：{error}"))?;

        crate::desktop::choose_collection_root_folder(hwnd)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;

        crate::desktop::choose_collection_root_folder(())
    }
}

/// 在用户设置的收纳根目录下创建单个 Box 的真实文件夹
#[tauri::command]
pub fn create_box_folder(root_path: String, folder_name: String) -> Result<String, String> {
    crate::desktop::create_box_folder(&root_path, &folder_name)
}

/// 按当前删除策略处理 Box 真实文件夹；成功后前端才会删除 Box 记录
#[tauri::command]
pub fn delete_box_folder(
    folder_path: String,
    desktop_path: String,
    policy: String,
    conflict_policy: String,
) -> Result<(), String> {
    crate::desktop::delete_box_folder(
        &folder_path,
        &desktop_path,
        BoxDeletePolicy::from_str(&policy)?,
        BoxConflictPolicy::from_str(&conflict_policy)?,
    )
}

/// 把单个 Box 的真实文件夹移动到当前收纳根目录，成功后返回新的文件夹路径
#[tauri::command]
pub fn migrate_box_folder(folder_path: String, root_path: String) -> Result<String, String> {
    crate::desktop::migrate_box_folder(&folder_path, &root_path)
}

/// 用系统 Explorer 打开 Box 对应的真实文件夹
#[tauri::command]
pub fn open_box_folder(folder_path: String) -> Result<(), String> {
    crate::desktop::open_folder_in_explorer(&folder_path)
}

/// 使用系统默认程序打开 Box 文件项，保持与 Explorer 双击一致
#[tauri::command]
pub fn open_desktop_item(path: String) -> Result<(), String> {
    crate::desktop::open_item_with_system_default(&path)
}

/// 在 Box 文件项上弹出 Windows Shell 原生右键菜单，菜单项和执行逻辑全部交给系统
#[tauri::command]
pub fn show_native_item_context_menu(
    window: tauri::WebviewWindow,
    path: String,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
    let item_path = Path::new(&path);
    if !item_path.exists() {
        return Err("文件项不存在，可能已经被移动或删除".to_string());
    }

    show_native_context_menu_for_path(&window, item_path, screen_x, screen_y)
}

/// 重命名 Box 文件项；真实路径变更后前端会重新扫描文件夹
#[tauri::command]
pub fn rename_desktop_item(path: String, new_name: String) -> Result<(), String> {
    crate::desktop::rename_item(&path, &new_name)
}

/// 删除 Box 文件项时放入回收站，避免自绘文件区绕过系统恢复能力
#[tauri::command]
pub fn delete_desktop_items(paths: Vec<String>) -> Result<(), String> {
    crate::desktop::recycle_items(&paths)
}

/// 按当前拖入策略把外部文件复制、移动或映射到 Box 真实文件夹
#[tauri::command]
pub fn handle_box_dropped_paths(
    window: tauri::WebviewWindow,
    folder_path: String,
    paths: Vec<String>,
    action: String,
    conflict_policy: String,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let owner = window
        .hwnd()
        .map_err(|error| format!("无法获取 Box 窗口句柄：{error}"))?;

    #[cfg(not(target_os = "windows"))]
    let owner = {
        let _ = window;
    };

    crate::desktop::handle_box_dropped_paths(
        &folder_path,
        &paths,
        BoxDropAction::from_str(&action)?,
        owner,
        BoxConflictPolicy::from_str(&conflict_policy)?,
    )
}

/// 按当前拖出策略把 Box 内文件复制、移动或映射到 Windows 桌面目录
#[tauri::command]
pub fn handle_box_dragged_paths_to_desktop(
    window: tauri::WebviewWindow,
    desktop_path: String,
    paths: Vec<String>,
    action: String,
    conflict_policy: String,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let owner = window
        .hwnd()
        .map_err(|error| format!("无法获取 Box 窗口句柄：{error}"))?;

    #[cfg(not(target_os = "windows"))]
    let owner = {
        let _ = window;
    };

    crate::desktop::handle_box_dragged_paths_to_desktop(
        &desktop_path,
        &paths,
        BoxDropAction::from_str(&action)?,
        owner,
        BoxConflictPolicy::from_str(&conflict_policy)?,
    )
}

/// Box 窗口挂载后注册自定义 Windows DropTarget，补齐透明 WebView 文件拖入不稳定的问题
#[tauri::command]
pub fn register_box_native_drop_target(app: AppHandle, window_label: String) -> Result<(), String> {
    if !window_label.starts_with("box_") {
        return Err("只有 Box 窗口可以注册原生拖放目标".to_string());
    }

    crate::desktop::register_box_native_drop(&app, &window_label)
}

/// Box 窗口卸载时注销自定义 DropTarget，避免旧窗口句柄继续接收拖放
#[tauri::command]
pub fn unregister_box_native_drop_target(
    app: AppHandle,
    window_label: String,
) -> Result<(), String> {
    if !window_label.starts_with("box_") {
        return Ok(());
    }

    crate::desktop::unregister_box_native_drop(&app, &window_label)
}

/// 读取系统开机自启状态；状态来源是官方 autostart 插件，不写入前端 SQLite 设置表
#[tauri::command]
pub fn is_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    crate::app_tray::resolve_autostart_enabled(&app)
}

/// 切换系统开机自启状态，并同步托盘菜单与设置页开关
#[tauri::command]
pub fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<bool, String> {
    crate::app_tray::set_autostart_enabled(&app, enabled)
}

/// 读取系统级左键状态，跨 WebView 拖拽释放时不依赖当前窗口能否收到鼠标事件
#[tauri::command]
pub fn is_primary_mouse_button_pressed() -> bool {
    is_left_mouse_button_pressed()
}

/// Windows 通过 GetAsyncKeyState 判断当前左键是否仍处于按下状态
#[cfg(target_os = "windows")]
fn is_left_mouse_button_pressed() -> bool {
    unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 }
}

/// 非 Windows 平台没有当前产品目标里的原生桌面拖拽语义，保持未按下避免拖拽卡住
#[cfg(not(target_os = "windows"))]
fn is_left_mouse_button_pressed() -> bool {
    false
}

/// 通过 `IContextMenu` 获取 Explorer 同源菜单，选中项用 Shell 返回的命令 ID 执行
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
            .map_err(|error| format!("系统无法解析该文件项：{error}"))?;

        let result = show_context_menu_from_pidl(hwnd, pidl, screen_x, screen_y);
        CoTaskMemFree(Some(pidl as *const _));
        result
    }
}

/// 非 Windows 平台没有 Explorer Shell 菜单，保持显式错误避免前端误以为已生效
#[cfg(not(target_os = "windows"))]
fn show_native_context_menu_for_path(
    _window: &tauri::WebviewWindow,
    _path: &Path,
    _screen_x: i32,
    _screen_y: i32,
) -> Result<(), String> {
    Err("当前平台暂不支持 Windows 原生右键菜单".to_string())
}

/// 基于 PIDL 绑定父级 ShellFolder，再查询单个子项的上下文菜单
#[cfg(target_os = "windows")]
unsafe fn show_context_menu_from_pidl(
    hwnd: HWND,
    pidl: *mut ITEMIDLIST,
    screen_x: i32,
    screen_y: i32,
) -> Result<(), String> {
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

/// Windows Shell API 接收 UTF-16 零结尾路径，右键菜单命令复用此转换
#[cfg(target_os = "windows")]
fn to_wide_path(path: &Path) -> Vec<u16> {
    path.to_string_lossy()
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
