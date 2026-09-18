//! Native DropTarget 使用的 HWND 枚举与坐标转换工具。

use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, POINTL};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::UI::WindowsAndMessaging::EnumChildWindows;

/// 枚举父窗口及所有子窗口，WebView2 的真实拖放目标通常是子 HWND。
pub(super) fn enumerate_child_windows(parent: HWND, child_windows: &mut Vec<HWND>) {
    let closure_pointer = child_windows as *mut Vec<HWND> as isize;
    unsafe extern "system" fn enumerate_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let child_windows = &mut *(lparam.0 as *mut Vec<HWND>);
        child_windows.push(hwnd);

        true.into()
    }

    let _ = unsafe {
        EnumChildWindows(
            Some(parent),
            Some(enumerate_callback),
            LPARAM(closure_pointer),
        )
    };
}

/// 将屏幕坐标转换为 WebView client 坐标，保持与 Tauri 拖放事件 position 语义一致。
pub(super) fn client_point_from_screen(hwnd: HWND, point: &POINTL) -> POINT {
    let mut client_point = POINT {
        x: point.x,
        y: point.y,
    };
    let _ = unsafe { ScreenToClient(hwnd, &mut client_point) };

    client_point
}
