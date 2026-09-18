//! Windows 系统桌面图标显示状态封装，负责按单个 Shell 虚拟项控制 Explorer 原生图标。

#[cfg(target_os = "windows")]
mod explorer;
#[cfg(target_os = "windows")]
mod registry;
#[cfg(target_os = "windows")]
mod visibility;

#[cfg(target_os = "windows")]
pub use visibility::{get_shell_desktop_icon_visible, set_shell_desktop_icon_visible};

#[cfg(not(target_os = "windows"))]
mod visibility {
    /// 非 Windows 平台没有 Explorer 系统桌面图标，保持显式错误避免前端误判已同步。
    pub fn get_shell_desktop_icon_visible(_shell_id: &str) -> Result<bool, String> {
        Err("当前平台不支持读取系统桌面图标显示状态".to_string())
    }

    /// 非 Windows 平台没有 Explorer 系统桌面图标，保持显式错误避免前端误判已同步。
    pub fn set_shell_desktop_icon_visible(_shell_id: &str, _visible: bool) -> Result<(), String> {
        Err("当前平台不支持设置系统桌面图标显示状态".to_string())
    }
}

#[cfg(not(target_os = "windows"))]
pub use visibility::{get_shell_desktop_icon_visible, set_shell_desktop_icon_visible};
