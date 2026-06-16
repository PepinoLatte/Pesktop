use crate::desktop::{scan_desktop, DesktopSnapshot};

/// 获取当前桌面文件快照，前端会基于这些真实文件自绘图标。
#[tauri::command]
pub fn get_desktop_snapshot() -> Result<DesktopSnapshot, String> {
    scan_desktop().map_err(|error| error.to_string())
}
