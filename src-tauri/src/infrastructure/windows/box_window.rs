//! Windows Box 窗口原生层级与毛玻璃效果管理。
//! 负责为透明 Box 窗口挂载 Windows DWM 原生毛玻璃虚化并管理 Z 序，
//! 确保切换前台应用时 Box 绝不浮于其他应用窗口之上。

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;

/// 为指定的 Box 窗口设置原生毛玻璃效果并置于常规桌面层级。
#[cfg(target_os = "windows")]
pub fn setup_box_window_native(window: &tauri::WebviewWindow) -> Result<(), String> {
    let hwnd = window
        .hwnd()
        .map_err(|error| format!("无法获取窗口句柄：{error}"))?;

    unsafe {
        apply_box_window_z_order(hwnd);
        apply_box_window_backdrop(hwnd);
    }

    Ok(())
}

/// 当 Box 窗口失焦时，将其放置到底部桌面层级，保证切换任何其他应用时均在 Box 前方。
#[cfg(target_os = "windows")]
pub fn send_box_window_to_bottom(window: &tauri::WebviewWindow) -> Result<(), String> {
    let hwnd = window
        .hwnd()
        .map_err(|error| format!("无法获取窗口句柄：{error}"))?;

    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_BOTTOM, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        };

        let _ = SetWindowPos(
            hwnd,
            Some(HWND_BOTTOM),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }

    Ok(())
}

/// 非 Windows 平台兜底实现
#[cfg(not(target_os = "windows"))]
pub fn setup_box_window_native(_window: &tauri::WebviewWindow) -> Result<(), String> {
    Ok(())
}

/// 非 Windows 平台兜底实现
#[cfg(not(target_os = "windows"))]
pub fn send_box_window_to_bottom(_window: &tauri::WebviewWindow) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
unsafe fn apply_box_window_z_order(hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE,
        HWND_NOTOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_TOPMOST,
    };

    // 确保移除 WS_EX_TOPMOST 属性，防止窗口悬浮在所有应用之上
    let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    if (ex_style & (WS_EX_TOPMOST.0 as isize)) != 0 {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style & !(WS_EX_TOPMOST.0 as isize));
    }

    // 将窗口重置为非置顶普通层级
    let _ = SetWindowPos(
        hwnd,
        Some(HWND_NOTOPMOST),
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    );
}

#[cfg(target_os = "windows")]
unsafe fn apply_box_window_backdrop(hwnd: HWND) {
    use std::ffi::c_void;
    use windows::Win32::Graphics::Dwm::{
        DwmEnableBlurBehindWindow, DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE,
        DWM_BB_ENABLE, DWM_BLURBEHIND,
    };
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    // 1. Windows 11 (Build >= 22000): 优先尝试 DWMWA_SYSTEMBACKDROP_TYPE = 4 (Acrylic 亚克力毛玻璃)
    let backdrop_type: u32 = 4;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_SYSTEMBACKDROP_TYPE,
        &backdrop_type as *const u32 as *const c_void,
        std::mem::size_of::<u32>() as u32,
    );

    // 2. 调用 DwmEnableBlurBehindWindow 确保 DWM 背景模糊开启
    let blur_behind = DWM_BLURBEHIND {
        dwFlags: DWM_BB_ENABLE,
        fEnable: windows::Win32::Foundation::TRUE,
        hRgnBlur: windows::Win32::Graphics::Gdi::HRGN::default(),
        fTransitionOnMaximized: windows::Win32::Foundation::FALSE,
    };
    let _ = DwmEnableBlurBehindWindow(hwnd, &blur_behind);

    // 3. Windows 10/11 深度兼容：通过 user32.dll 的 SetWindowCompositionAttribute 启用高保真毛玻璃
    let user32_name: Vec<u16> = "user32.dll\0".encode_utf16().collect();
    if let Ok(user32) = LoadLibraryW(windows::core::PCWSTR::from_raw(user32_name.as_ptr())) {
        if !user32.is_invalid() {
            #[repr(C)]
            struct AccentPolicy {
                accent_state: u32,
                accent_flags: u32,
                gradient_color: u32,
                animation_id: u32,
            }

            #[repr(C)]
            struct WindowCompositionAttribData {
                attrib: u32,
                pv_data: *mut c_void,
                cb_data: usize,
            }

            type PfnSetWindowCompositionAttribute =
                unsafe extern "system" fn(HWND, *mut WindowCompositionAttribData) -> windows::core::BOOL;

            let func_name = b"SetWindowCompositionAttribute\0";
            if let Some(proc) = GetProcAddress(user32, windows::core::PCSTR::from_raw(func_name.as_ptr())) {
                let set_comp_attr: PfnSetWindowCompositionAttribute = std::mem::transmute(proc);

                // ACCENT_ENABLE_BLURBEHIND = 3
                let mut policy = AccentPolicy {
                    accent_state: 3,
                    accent_flags: 2,
                    gradient_color: 0x01000000,
                    animation_id: 0,
                };

                let mut data = WindowCompositionAttribData {
                    attrib: 19, // WCA_ACCENT_POLICY
                    pv_data: &mut policy as *mut AccentPolicy as *mut c_void,
                    cb_data: std::mem::size_of::<AccentPolicy>(),
                };

                let _ = set_comp_attr(hwnd, &mut data);
            }
        }
    }
}
