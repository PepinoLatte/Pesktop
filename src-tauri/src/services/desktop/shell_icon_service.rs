//! 系统桌面图标服务负责读取和设置 Windows 原生桌面图标显示状态。

use crate::infrastructure::windows::shell_desktop_icon;

/// 读取 Windows 原生桌面上某个系统图标是否显示，供 Dasktop 接管前记录用户原始状态。
pub fn get_shell_desktop_icon_visible(shell_id: &str) -> Result<bool, String> {
    shell_desktop_icon::get_shell_desktop_icon_visible(shell_id)
}

/// 设置 Windows 原生桌面上某个系统图标显示状态，Box 内引用仍由前端数据库独立维护。
pub fn set_shell_desktop_icon_visible(shell_id: &str, visible: bool) -> Result<(), String> {
    shell_desktop_icon::set_shell_desktop_icon_visible(shell_id, visible)
}
