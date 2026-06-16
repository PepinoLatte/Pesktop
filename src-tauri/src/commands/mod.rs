use crate::desktop::{scan_desktop, DesktopSnapshot};
use std::path::Path;

#[cfg(target_os = "windows")]
use windows::core::PCWSTR;
#[cfg(target_os = "windows")]
use windows::Win32::UI::Shell::ShellExecuteW;
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// 获取当前桌面文件快照，前端会基于这些真实文件自绘图标。
#[tauri::command]
pub fn get_desktop_snapshot() -> Result<DesktopSnapshot, String> {
    scan_desktop().map_err(|error| error.to_string())
}

/// 使用系统默认程序打开桌面项目，保持快捷方式、文件夹和普通文件与 Windows Explorer 一致。
#[tauri::command]
pub fn open_desktop_item(path: String) -> Result<(), String> {
    let item_path = Path::new(&path);
    if !item_path.exists() {
        return Err("桌面项目不存在，可能已经被移动或删除".to_string());
    }

    open_path_with_system_default(item_path)
}

/// Windows 侧通过 ShellExecute 交给系统默认打开方式，避免前端猜测文件类型。
#[cfg(target_os = "windows")]
fn open_path_with_system_default(path: &Path) -> Result<(), String> {
    let path_wide = path
        .to_string_lossy()
        .encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let operation_wide = "open\0".encode_utf16().collect::<Vec<_>>();
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation_wide.as_ptr()),
            PCWSTR(path_wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    if result.0 as isize <= 32 {
        return Err(format!(
            "系统无法打开该桌面项目，错误码 {}",
            result.0 as isize
        ));
    }

    Ok(())
}

/// 非 Windows 平台暂不接管打开行为，避免产生和系统桌面语义不一致的兼容分支。
#[cfg(not(target_os = "windows"))]
fn open_path_with_system_default(_path: &Path) -> Result<(), String> {
    Err("当前平台暂不支持打开桌面项目".to_string())
}
