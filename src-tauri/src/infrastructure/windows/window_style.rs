//! 窗口扩展样式辅助：把 Box 窗口标记为工具窗口（WS_EX_TOOLWINDOW），
//! 使其不出现在 Alt+Tab 列表中；通过 WS_POPUP 样式、窗口子类化与 Shell 事件监听，
//! 确保在 Win+D（显示桌面）时常驻桌面，且不被推入底层或移出屏幕。

use std::sync::{Mutex, OnceLock};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetClassNameW, GetMessageW, GetWindowLongPtrW, PostMessageW,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage, EVENT_SYSTEM_FOREGROUND,
    GWL_EXSTYLE, GWL_STYLE, HWND_TOP, MSG, SC_MINIMIZE, SWP_FRAMECHANGED, SWP_HIDEWINDOW,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_RESTORE,
    SW_SHOWNOACTIVATE, WINEVENT_OUTOFCONTEXT, WINDOWPOS, WM_APP, WM_NCDESTROY, WM_SHOWWINDOW,
    WM_SIZE, WM_SYSCOMMAND, WM_WINDOWPOSCHANGING, WS_CAPTION, WS_EX_TOOLWINDOW, WS_MAXIMIZEBOX,
    WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU,
};

const BOX_WINDOW_SUBCLASS_ID: usize = 0xD45B;
const WM_APP_RESTORE_BOX: u32 = WM_APP + 0xD45B;

/// 全局记录所有激活的 Box 窗口 HWND，用于在系统触发显示桌面时统一将其提升至桌面之上
static ACTIVE_BOX_HWNDS: Mutex<Vec<isize>> = Mutex::new(Vec::new());
static HOOK_INITIALIZED: OnceLock<()> = OnceLock::new();

/// 向所有登记的 Box 窗口投递恢复消息，保持常驻桌面
fn restore_all_box_windows() {
    let hwnds = {
        let Ok(guard) = ACTIVE_BOX_HWNDS.lock() else {
            return;
        };
        guard.clone()
    };

    for hwnd_value in hwnds {
        unsafe {
            let _ = PostMessageW(
                Some(HWND(hwnd_value as *mut _)),
                WM_APP_RESTORE_BOX,
                WPARAM(0),
                LPARAM(0),
            );
        }
    }
}

/// 注册全局系统前台窗口变化钩子：当 WorkerW 或 Progman 变为前台（用户触发 Win+D 或点击桌面）时，
/// 立即将所有 Box 窗口唤回并维持在桌面层上方。
fn ensure_win_event_hook_started() {
    HOOK_INITIALIZED.get_or_init(|| {
        std::thread::Builder::new()
            .name("dasktop-shell-hook".into())
            .spawn(|| unsafe {
                let _hook = SetWinEventHook(
                    EVENT_SYSTEM_FOREGROUND,
                    EVENT_SYSTEM_FOREGROUND,
                    None,
                    Some(shell_win_event_proc),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT,
                );

                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            })
            .ok();
    });
}

/// WinEvent 回调：检测桌面 Shell 窗口激活
unsafe extern "system" fn shell_win_event_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    if event == EVENT_SYSTEM_FOREGROUND {
        let mut class_name = [0u16; 64];
        let len = GetClassNameW(hwnd, &mut class_name);
        if len > 0 {
            let name = String::from_utf16_lossy(&class_name[..len as usize]);
            if name == "WorkerW" || name == "Progman" {
                restore_all_box_windows();
            }
        }
    }
}

/// 子类化回调：拦截 Win+D 等系统级最小化与隐藏，响应异步恢复消息
unsafe extern "system" fn box_window_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _uidsubclass: usize,
    _refdata: usize,
) -> LRESULT {
    match msg {
        WM_APP_RESTORE_BOX => {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOP),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            return LRESULT(0);
        }
        WM_SYSCOMMAND => {
            // 阻止系统级或快捷键触发的最小化命令 (SC_MINIMIZE)
            if (wparam.0 & 0xFFF0) == SC_MINIMIZE as usize {
                return LRESULT(0);
            }
        }
        WM_SHOWWINDOW => {
            // 当外部（如 Win+D ToggleDesktop）试图隐藏窗口时，排队异步恢复
            if wparam.0 == 0 {
                let _ = PostMessageW(Some(hwnd), WM_APP_RESTORE_BOX, WPARAM(0), LPARAM(0));
                return LRESULT(0);
            }
        }
        WM_SIZE => {
            // 阻止系统将窗口尺寸置为最小化 (SIZE_MINIMIZED = 1)
            if wparam.0 == 1 {
                let _ = PostMessageW(Some(hwnd), WM_APP_RESTORE_BOX, WPARAM(0), LPARAM(0));
                return LRESULT(0);
            }
        }
        WM_WINDOWPOSCHANGING => {
            if lparam.0 != 0 {
                let pos = &mut *(lparam.0 as *mut WINDOWPOS);
                // 当 Win+D 触发时，系统会尝试将窗口移至屏幕外坐标（-32000, -32000）或标记 SWP_HIDEWINDOW
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
        WM_NCDESTROY => {
            let hwnd_val = hwnd.0 as isize;
            if let Ok(mut list) = ACTIVE_BOX_HWNDS.lock() {
                list.retain(|&x| x != hwnd_val);
            }
            let _ = RemoveWindowSubclass(hwnd, Some(box_window_subclass_proc), BOX_WINDOW_SUBCLASS_ID);
        }
        _ => {}
    }

    DefSubclassProc(hwnd, msg, wparam, lparam)
}

/// 为指定句柄的窗口追加 WS_EX_TOOLWINDOW 与 WS_POPUP 样式，
/// 移除 WS_MINIMIZEBOX / WS_CAPTION，并挂载子类化与 Shell 事件监听保持桌面常驻。
pub(crate) fn set_toolwindow_ex_style(hwnd_value: isize) -> Result<(), String> {
    let hwnd = HWND(hwnd_value as *mut _);
    unsafe {
        // 1. 设置扩展样式 WS_EX_TOOLWINDOW（不出现在 Alt+Tab 切换栏）
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_TOOLWINDOW.0 as isize);

        // 2. 将窗口样式标记为 WS_POPUP 并剥除标题栏与最小化框；
        // Windows Shell ToggleDesktop (Win+D) 仅枚举最小化重叠窗口（WS_OVERLAPPED），
        // 标记为 WS_POPUP | WS_EX_TOOLWINDOW 后系统将其视作桌面挂件而非普通应用窗口。
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let new_style = (style
            & !(WS_CAPTION.0 as isize
                | WS_MINIMIZEBOX.0 as isize
                | WS_MAXIMIZEBOX.0 as isize
                | WS_SYSMENU.0 as isize))
            | WS_POPUP.0 as isize;
        SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);

        // 3. 记录 HWND 并确保 Shell WinEvent 监听已启动
        if let Ok(mut list) = ACTIVE_BOX_HWNDS.lock() {
            if !list.contains(&hwnd_value) {
                list.push(hwnd_value);
            }
        }
        ensure_win_event_hook_started();

        // 4. 挂载窗口子类化过程，拦截 Win+D 造成的隐藏、最小化和移出视口
        let _ = SetWindowSubclass(
            hwnd,
            Some(box_window_subclass_proc),
            BOX_WINDOW_SUBCLASS_ID,
            0,
        );

        // 5. 刷新窗口框架样式
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
