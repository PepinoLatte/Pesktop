//! 窗口扩展样式辅助：把 Box 窗口标记为工具窗口（WS_EX_TOOLWINDOW），
//! 使其不出现在 Alt+Tab 列表中，且 Win+D 显示桌面时保持原位不被最小化。

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
};

/// 为指定句柄的窗口追加 WS_EX_TOOLWINDOW 扩展样式；重复调用幂等。
pub(crate) fn set_toolwindow_ex_style(hwnd_value: isize) -> Result<(), String> {
    let hwnd = HWND(hwnd_value as *mut _);
    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if ex_style & WS_EX_TOOLWINDOW.0 as isize != 0 {
            return Ok(());
        }

        let previous = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_TOOLWINDOW.0 as isize);
        if previous == 0 {
            return Err("SetWindowLongPtrW 返回 0，工具窗口样式写入失败".to_string());
        }
    }

    Ok(())
}
