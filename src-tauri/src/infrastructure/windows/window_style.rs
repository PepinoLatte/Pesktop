//! 窗口扩展样式辅助：把 Box 窗口标记为工具窗口（WS_EX_TOOLWINDOW）并将其所有者（Owner）
//! 关联到 Progman（桌面管理器），使其不出现在 Alt+Tab 列表中，且 Win+D（显示桌面）时常驻桌面不被最小化。

use windows::core::w;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, GWLP_HWNDPARENT,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_EX_TOOLWINDOW,
};

/// 为指定句柄的窗口追加 WS_EX_TOOLWINDOW 扩展样式，并将 Owner 设为桌面 Progman。
pub(crate) fn set_toolwindow_ex_style(hwnd_value: isize) -> Result<(), String> {
    let hwnd = HWND(hwnd_value as *mut _);
    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_TOOLWINDOW.0 as isize);

        // 将顶级窗口的 Owner 设为 Windows 桌面管理窗口 Progman；
        // 当用户按下 Win+D（ToggleDesktop）时，Explorer 会最小化非桌面窗口，
        // 由 Progman 所有的窗口被视为桌面环境一部分，因而保持原位不被隐藏。
        let progman = FindWindowW(w!("Progman"), None).unwrap_or(HWND::default());
        if !progman.is_invalid() {
            SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, progman.0 as isize);
        }

        // 通知窗口系统刷新框架样式与层级关系
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
        );
    }

    Ok(())
}
