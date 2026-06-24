//! 用户文件夹解析逻辑，避免把当前机器 Profile 路径写成硬编码。

/// 用户文件夹在不同账户和重定向配置下路径不同，运行时解析可以避免硬编码本机路径。
#[cfg(target_os = "windows")]
pub(super) fn resolve_user_folder_parsing_name() -> Option<String> {
    use crate::infrastructure::windows::common::com;
    use windows::Win32::UI::Shell::{FOLDERID_Profile, SHGetKnownFolderPath, KF_FLAG_DEFAULT};

    let path = unsafe { SHGetKnownFolderPath(&FOLDERID_Profile, KF_FLAG_DEFAULT, None).ok()? };
    unsafe { com::take_pwstr_string(path).ok() }
}

#[cfg(not(target_os = "windows"))]
pub(super) fn resolve_user_folder_parsing_name() -> Option<String> {
    None
}
