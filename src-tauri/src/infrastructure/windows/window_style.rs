//! 窗口扩展样式辅助：把 Box 窗口标记为工具窗口（WS_EX_TOOLWINDOW），
//! 使其不出现在 Alt+Tab 列表中；通过窗口子类化拦截系统最小化与隐藏消息，
//! 确保在 Win+D（显示桌面）时常驻桌面，且不被推入底层或移出屏幕。

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, IsIconic, IsWindowVisible, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    GWL_EXSTYLE, GWL_STYLE, SC_MINIMIZE, SWP_FRAMECHANGED, SWP_HIDEWINDOW, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_RESTORE, SW_SHOWNOACTIVATE,
    WINDOWPOS, WM_SHOWWINDOW, WM_SIZE, WM_SYSCOMMAND, WM_WINDOWPOSCHANGED,
    WM_WINDOWPOSCHANGING, WS_EX_TOOLWINDOW, WS_MAXIMIZEBOX, WS_MINIMIZEBOX,
};

const BOX_WINDOW_SUBCLASS_ID: usize = 0xD45B;

/// 子类化回调：全方位拦截 Win+D 等系统级最小化与隐藏命令，确保 Box 常驻桌面
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
            // 阻止系统级或快捷键触发的最小化命令 (SC_MINIMIZE)
            if (wparam.0 & 0xFFF0) == SC_MINIMIZE as usize {
                return LRESULT(0);
            }
        }
        WM_SHOWWINDOW => {
            // 当外部（如 Win+D ToggleDesktop）试图隐藏窗口时，wparam.0 为 FALSE (0)
            // 直接拦截并返回 0，阻止系统默认隐藏
            if wparam.0 == 0 {
                return LRESULT(0);
            }
        }
        WM_SIZE => {
            // 阻止系统将窗口尺寸置为最小化 (SIZE_MINIMIZED = 1)
            if wparam.0 == 1 {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                return LRESULT(0);
            }
        }
        WM_WINDOWPOSCHANGING => {
            if lparam.0 != 0 {
                let pos = &mut *(lparam.0 as *mut WINDOWPOS);
                let mut modified = false;
                // 当 Win+D 触发时，系统会尝试将窗口移至屏幕外坐标（-32000, -32000）或标记 SWP_HIDEWINDOW
                if pos.x <= -30000 || pos.y <= -30000 {
                    pos.flags |= SWP_NOMOVE | SWP_NOSIZE;
                    pos.flags &= !SWP_HIDEWINDOW;
                    modified = true;
                }
                if (pos.flags & SWP_HIDEWINDOW).0 != 0 {
                    pos.flags &= !SWP_HIDEWINDOW;
                    pos.flags |= SWP_SHOWWINDOW;
                    modified = true;
                }
                if modified {
                    return LRESULT(0);
                }
            }
        }
        WM_WINDOWPOSCHANGED => {
            // 在 Win32 中，最小化窗口 IsIconic 为 TRUE，但 IsWindowVisible 仍可能为 TRUE
            // 同时检查 IsIconic 与 !IsWindowVisible，彻底保持桌面常驻显示
            if IsIconic(hwnd).as_bool() || !IsWindowVisible(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                return LRESULT(0);
            }
        }
        _ => {}
    }

    DefSubclassProc(hwnd, msg, wparam, lparam)
}

/// 为指定句柄的窗口追加 WS_EX_TOOLWINDOW 扩展样式并移除 WS_MINIMIZEBOX，
/// 并挂载子类化拦截器保持桌面常驻。
pub(crate) fn set_toolwindow_ex_style(hwnd_value: isize) -> Result<(), String> {
    let hwnd = HWND(hwnd_value as *mut _);
    unsafe {
        // 1. 设置扩展样式 WS_EX_TOOLWINDOW（不出现在 Alt+Tab 切换栏）
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_TOOLWINDOW.0 as isize);

        // 2. 剥除 WS_MINIMIZEBOX 和 WS_MAXIMIZEBOX，让 Windows Shell ToggleDesktop (Win+D)
        // 认定该窗口为不可最小化的常驻桌面组件
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_STYLE,
            style & !(WS_MINIMIZEBOX.0 as isize | WS_MAXIMIZEBOX.0 as isize),
        );

        // 3. 挂载窗口子类化过程，拦截 Win+D 造成的隐藏、最小化和移出视口
        let _ = SetWindowSubclass(
            hwnd,
            Some(box_window_subclass_proc),
            BOX_WINDOW_SUBCLASS_ID,
            0,
        );

        // 4. 刷新窗口框架样式
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
