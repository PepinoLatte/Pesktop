//! 鼠标状态读取封装跨 WebView 拖拽释放判断所需的系统级输入能力。

#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};

/// 读取系统级左键状态，跨 WebView 拖拽释放时不依赖当前窗口能否收到鼠标事件。
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
