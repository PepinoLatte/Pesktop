//! Windows 桌面路径解析封装 Known Folder 和开发兜底路径策略。

use std::env;
use std::path::PathBuf;

/// 读取当前用户桌面路径；文件夹型 Box 只需要删除策略目标目录，不再扫描桌面图标列表。
pub fn resolve_desktop_path() -> PathBuf {
    resolve_platform_desktop_path()
}

/// 优先使用系统登记的桌面 Known Folder，桌面被 OneDrive 或策略重定向时也能拿到真实目标目录。
#[cfg(target_os = "windows")]
fn resolve_platform_desktop_path() -> PathBuf {
    resolve_windows_known_desktop_path()
        .or_else(resolve_profile_desktop_path)
        .unwrap_or_else(resolve_current_dir)
}

/// 非 Windows 平台没有当前产品语义里的 Explorer 桌面目录，保留用户目录兜底便于开发调试。
#[cfg(not(target_os = "windows"))]
fn resolve_platform_desktop_path() -> PathBuf {
    resolve_profile_desktop_path().unwrap_or_else(resolve_current_dir)
}

/// `USERPROFILE\Desktop` 只作为兜底路径；它不一定等于 Windows 当前真实桌面目录。
fn resolve_profile_desktop_path() -> Option<PathBuf> {
    env::var("USERPROFILE")
        .ok()
        .map(|profile_path| PathBuf::from(profile_path).join("Desktop"))
}

/// 桌面路径无法解析时仍返回当前目录，避免前端初始化被系统目录异常完全阻断。
fn resolve_current_dir() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// 调用 Windows Shell Known Folder API 读取当前用户真实桌面，覆盖 D 盘、OneDrive 和组策略重定向场景。
#[cfg(target_os = "windows")]
fn resolve_windows_known_desktop_path() -> Option<PathBuf> {
    use crate::infrastructure::windows::common::com;
    use windows::Win32::UI::Shell::{FOLDERID_Desktop, SHGetKnownFolderPath, KF_FLAG_DEFAULT};

    let path = unsafe { SHGetKnownFolderPath(&FOLDERID_Desktop, KF_FLAG_DEFAULT, None).ok()? };
    let path_text_result = unsafe { com::take_pwstr_string(path) };

    path_text_result
        .ok()
        .filter(|path_text| !path_text.is_empty())
        .map(PathBuf::from)
}
