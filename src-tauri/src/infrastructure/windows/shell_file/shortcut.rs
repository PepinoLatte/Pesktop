//! Windows 快捷方式创建逻辑，用于 Box 映射模式。

use std::path::Path;

#[cfg(target_os = "windows")]
use crate::infrastructure::windows::common::wide;

/// 为真实路径创建 `.lnk` 映射快捷方式，保持 Box “映射”语义不复制原文件。
#[cfg(target_os = "windows")]
pub fn create_shortcut_for_path(path: &Path, shortcut_path: &Path) -> Result<(), String> {
    use windows::core::{Interface, PCWSTR};
    use windows::Win32::System::Com::{CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER};
    use windows::Win32::System::Ole::OleInitialize;
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    if shortcut_path.exists() {
        return Err("目标快捷方式已经存在，已停止映射".to_string());
    }

    let target_wide = wide::path_null_terminated(path);
    let shortcut_wide = wide::path_null_terminated(shortcut_path);

    unsafe {
        let _ = OleInitialize(None);
        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("无法创建映射快捷方式：{error}"))?;
        shell_link
            .SetPath(PCWSTR(target_wide.as_ptr()))
            .map_err(|error| format!("无法写入映射目标：{error}"))?;
        if let Some(parent) = path.parent() {
            let working_directory = wide::path_null_terminated(parent);
            shell_link
                .SetWorkingDirectory(PCWSTR(working_directory.as_ptr()))
                .map_err(|error| format!("无法写入映射工作目录：{error}"))?;
        }

        let persist_file: IPersistFile = shell_link
            .cast()
            .map_err(|error| format!("无法保存映射快捷方式：{error}"))?;
        persist_file
            .Save(PCWSTR(shortcut_wide.as_ptr()), true)
            .map_err(|error| format!("无法保存映射快捷方式：{error}"))?;
    }

    Ok(())
}

/// 非 Windows 平台没有当前产品目标里的 `.lnk` 映射语义，保持显式错误避免误判已支持。
#[cfg(not(target_os = "windows"))]
pub fn create_shortcut_for_path(_path: &Path, _shortcut_path: &Path) -> Result<(), String> {
    Err("当前平台暂不支持创建 Box 映射快捷方式".to_string())
}
