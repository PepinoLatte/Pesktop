//! 系统桌面图标显示状态用例，编排注册表读写和 Explorer 刷新。

use super::explorer;
use super::registry::{format_registry_error, RegistryKey};
use crate::infrastructure::windows::shell_virtual_item::catalog;
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;

/// Explorer 同时存在新旧开始菜单配置分支，写入两处可覆盖 Windows 10/11 与兼容模式差异。
const DESKTOP_ICON_VISIBILITY_PATHS: &[&str] = &[
    "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\HideDesktopIcons\\NewStartPanel",
    "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\HideDesktopIcons\\ClassicStartMenu",
];

/// 读取某个系统桌面图标当前是否显示；注册表值缺失时回落到 Windows 默认桌面状态。
pub fn get_shell_desktop_icon_visible(shell_id: &str) -> Result<bool, String> {
    let entry = resolve_shell_desktop_icon_entry(shell_id)?;
    let clsid = entry
        .desktop_icon_clsid
        .ok_or_else(|| format!("暂不支持管理该系统桌面图标：{shell_id}"))?;
    let key = match RegistryKey::open_read(DESKTOP_ICON_VISIBILITY_PATHS[0]) {
        Ok(key) => key,
        Err(error) if error == ERROR_FILE_NOT_FOUND => return Ok(entry.default_visible),
        Err(error) => return Err(format_registry_error("读取系统桌面图标配置", error)),
    };

    match key.query_dword(clsid) {
        Ok(value) => Ok(value == 0),
        Err(error) if error == ERROR_FILE_NOT_FOUND => Ok(entry.default_visible),
        Err(error) => Err(format_registry_error("读取系统桌面图标显示状态", error)),
    }
}

/// 设置某个系统桌面图标是否显示，并通知 Explorer 刷新桌面。
pub fn set_shell_desktop_icon_visible(shell_id: &str, visible: bool) -> Result<(), String> {
    let entry = resolve_shell_desktop_icon_entry(shell_id)?;
    let clsid = entry
        .desktop_icon_clsid
        .ok_or_else(|| format!("暂不支持管理该系统桌面图标：{shell_id}"))?;
    let hidden_value = if visible { 0 } else { 1 };

    for path in DESKTOP_ICON_VISIBILITY_PATHS {
        let key = RegistryKey::create_write(path)
            .map_err(|error| format_registry_error("打开系统桌面图标配置", error))?;
        key.set_dword(clsid, hidden_value)
            .map_err(|error| format_registry_error("写入系统桌面图标显示状态", error))?;
    }

    explorer::refresh_explorer_desktop_icons();
    Ok(())
}

fn resolve_shell_desktop_icon_entry(
    shell_id: &str,
) -> Result<catalog::KnownShellVirtualItem, String> {
    catalog::resolve_desktop_icon_item(shell_id)
        .ok_or_else(|| format!("暂不支持管理该系统桌面图标：{shell_id}"))
}
