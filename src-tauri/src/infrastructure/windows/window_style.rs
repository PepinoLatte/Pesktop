//! 窗口扩展样式辅助：把 Box 窗口标记为工具窗口（WS_EX_TOOLWINDOW），
//! 使其不出现在 Alt+Tab 列表中；通过窗口子类化拦截系统最小化与隐藏消息，
//! 确保在 Win+D（显示桌面）时常驻桌面，且不被推入底层或移出屏幕。

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, SC_MINIMIZE, SWP_FRAMECHANGED,
    SWP_HIDEWINDOW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW,
    WINDOWPOS, WM_SYSCOMMAND, WM_WINDOWPOSCHANGING, WS_EX_TOOLWINDOW,
};

const BOX_WINDOW_SUBCLASS_ID: usize = 0xD45B;

/// 子类化回调：拦截 Win+D 等系统级最小化与隐藏命令
unsafe extern "system" fn box_window_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _uidsubclass: usize,
    _refdata: usize,
) -> LRESULT {
    match msg {
        WM_SYSCOMMAND => {
            // 阻止系统级或快捷键触发的最小化命令
            if (wparam.0 & 0xFFF0) == SC_MINIMIZE as usize {
                return LRESULT(0);
            }
        }
        WM_WINDOWPOSCHANGING => {
            if lparam.0 != 0 {
                let pos = &mut *(lparam.0 as *mut WINDOWPOS);
                // 当 Win+D（ToggleDesktop）触发时，系统会尝试将窗口移至屏幕外坐标（-32000, -32000）或标记 SWP_HIDEWINDOW
                if pos.x <= -30000 || pos.y <= -30000 {
                    pos.flags |= SWP_NOMOVE | SWP_NOSIZE;
                    pos.flags &= !SWP_HIDEWINDOW;
                }
                if (pos.flags & SWP_HIDEWINDOW).0 != 0 {
                    pos.flags &= !SWP_HIDEWINDOW;
                    pos.flags |= SWP_SHOWWINDOW;
                }
            }
        }
        _ => {}
    }

    DefSubclassProc(hwnd, msg, wparam, lparam)
}

/// 为指定句柄的窗口追加 WS_EX_TOOLWINDOW 扩展样式，并挂载子类化拦截器保持常驻。
pub(crate) fn set_toolwindow_ex_style(hwnd_value: isize) -> Result<(), String> {
    let hwnd = HWND(hwnd_value as *mut _);
    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_TOOLWINDOW.0 as isize);

        // 挂载窗口子类化过程，拦截 Win+D 造成的隐藏和移出视口
        let _ = SetWindowSubclass(
            hwnd,
            Some(box_window_subclass_proc),
            BOX_WINDOW_SUBCLASS_ID,
            0,
        );

        // 刷新窗口框架样式
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
