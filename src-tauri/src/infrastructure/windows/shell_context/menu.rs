//! Explorer IContextMenu 绑定、展示和命令执行逻辑。

/// Shell 右键菜单从第一个位置插入系统菜单项，避免覆盖调用方自定义菜单。
const SHELL_CONTEXT_MENU_INSERT_POSITION: u32 = 0;
/// Win32 IContextMenu 命令 ID 从 1 开始，0 被 TrackPopupMenu 用作取消选择。
const SHELL_CONTEXT_FIRST_COMMAND_ID: u32 = 1;
/// Explorer 右键菜单可分配的最大命令 ID，沿用 Win32 传统 15 位命令空间。
const SHELL_CONTEXT_LAST_COMMAND_ID: u32 = 0x7fff;
/// TrackPopupMenu 在用户点击菜单外部或按 Escape 时返回 0。
const NO_CONTEXT_MENU_COMMAND_SELECTED: i32 = 0;

/// 基于 PIDL 绑定父级 ShellFolder，再查询单个子项的上下文菜单。
pub(super) unsafe fn show_context_menu_from_pidl(
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
    let query_result = context_menu.QueryContextMenu(
        menu,
        SHELL_CONTEXT_MENU_INSERT_POSITION,
        SHELL_CONTEXT_FIRST_COMMAND_ID,
        SHELL_CONTEXT_LAST_COMMAND_ID,
        CMF_NORMAL,
    );

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

    if selected_command == NO_CONTEXT_MENU_COMMAND_SELECTED {
        return Ok(());
    }

    // InvokeCommand 需要传入相对命令偏移，而 TrackPopupMenu 返回的是绝对命令 ID。
    let command_offset =
        selected_command.saturating_sub(SHELL_CONTEXT_FIRST_COMMAND_ID as i32) as usize;
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
